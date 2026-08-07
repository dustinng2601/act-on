//! Per-job execution state (the analogue of `RunContext` in nektos/act).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use serde_yaml::Value;
use tokio::sync::Notify;

use crate::config::Config;
use crate::expr::Env as ExprEnv;
use crate::model::{Job, StepResult, Workflow};
use crate::sandbox::ExecutionsEnvironment;

/// Per-job shared state.
pub struct RunContext {
    pub config: Arc<Config>,
    pub workflow: Arc<Workflow>,
    pub job_id: String,
    pub job: Job,

    /// Per-matrix-fork values, e.g. {"python": "3.10"}.
    pub matrix: HashMap<String, Value>,

    /// Final result for this job ("success"/"failure"/"cancelled").
    pub result: parking_lot::Mutex<String>,

    /// Step-id -> result. Backed by the Evaluator as `steps.<id>.*`.
    pub step_results: Arc<Mutex<HashMap<String, StepResult>>>,

    /// Masked values (for `::add-mask::`).
    pub masks: Arc<Mutex<Vec<String>>>,

    /// Job-level env (mutable across steps).
    pub env: Arc<Mutex<HashMap<String, String>>>,

    /// The inputs of the action currently running, when one is.
    ///
    /// Inside a composite action `inputs.*` means that action's inputs, not the
    /// workflow's. Held here for the duration of the action and cleared after,
    /// so a step outside one still sees the workflow's.
    pub action_inputs: Arc<Mutex<Option<HashMap<String, String>>>>,
    /// Extra-path entries appended by steps.
    pub extra_path: Arc<Mutex<Vec<String>>>,

    /// Where the workflow's repo lives on local disk.
    pub workdir: PathBuf,

    /// Act scratch directory (under workdir or under the user cache dir).
    pub actpath: PathBuf,

    /// The currently executing step id.
    pub current_step: parking_lot::Mutex<String>,

    /// Sandbox backed by Docker or the host.
    pub sandbox: Arc<dyn ExecutionsEnvironment + Send + Sync>,

    /// Cancellation token.
    pub cancel: Arc<Notify>,
}

impl RunContext {
    /// Build a new RunContext.
    pub fn new(
        config: Arc<Config>,
        workflow: Arc<Workflow>,
        job_id: String,
        job: Job,
        matrix: HashMap<String, Value>,
        sandbox: Arc<dyn ExecutionsEnvironment + Send + Sync>,
    ) -> Self {
        let workdir = config.workdir.clone();
        // The sandbox's own working area, not a second guess at it. Deriving it
        // here again put every concurrent matrix leg back on one `run.sh` and
        // one set of command files, so a leg ran whichever script was written
        // last — three legs of a three-way matrix all ran the same one.
        let actpath = sandbox.act_path();
        Self {
            config,
            workflow,
            job_id,
            job,
            matrix,
            result: Mutex::new(String::new()),
            step_results: Arc::new(Mutex::new(HashMap::new())),
            masks: Arc::new(Mutex::new(Vec::new())),
            env: Arc::new(Mutex::new(runner_env(sandbox.as_ref(), &workdir))),
            action_inputs: Arc::new(Mutex::new(None)),
            extra_path: Arc::new(Mutex::new(Vec::new())),
            workdir,
            actpath,
            current_step: Mutex::new(String::new()),
            sandbox,
            cancel: Arc::new(Notify::new()),
        }
    }

    /// Build the StepResult map into an [`ExprEnv`].
    pub fn expr_env(&self) -> ExprEnv {
        let mut env = ExprEnv::default();
        env.github = crate::model::GithubContext {
            workflow: self.workflow.file.clone().unwrap_or_default(),
            workspace: self.workdir.to_string_lossy().into(),
            event_name: self.config.event_name.clone(),
            actor: self.config.actor.clone(),
            repository: String::new(),
            repository_owner: String::new(),
            run_id: uuid::Uuid::new_v4().to_string(),
            run_number: 1,
            run_attempt: 1,
            // Actions read this through `${{ github.token }}`, which is
            // usually an input's default — the sccache action asks for a token
            // it never sees the workflow pass. GitHub injects one; act-on cannot
            // mint it, so it takes one the caller supplied with `--secret` or
            // left in the environment, and otherwise leaves it empty.
            token: self
                .config
                .secrets_map()
                .get("GITHUB_TOKEN")
                .cloned()
                .or_else(|| std::env::var("GITHUB_TOKEN").ok())
                .or_else(|| std::env::var("GH_TOKEN").ok())
                .unwrap_or_default(),
            server_url: "https://github.com".into(),
            api_url: "https://api.github.com".into(),
            graphql_url: "https://api.github.com/graphql".into(),
            ..Default::default()
        };
        env.env = self.env.lock().clone();
        env.steps = self.step_results.lock().clone();
        env.secrets = self.config.secrets_map();
        env.vars = self.config.vars_map();
        // An action's inputs shadow the workflow's while it runs.
        env.inputs = match self.action_inputs.lock().clone() {
            Some(inputs) => inputs,
            None => self.config.inputs_map(),
        };
        env.matrix = self.matrix.clone();
        env.runner = HashMap::from([
            ("os".into(), self.sandbox.runner_os().into()),
            ("arch".into(), self.sandbox.runner_arch().into()),
            (
                "temp".into(),
                self.sandbox.temp_dir().to_string_lossy().into(),
            ),
            (
                "tool_cache".into(),
                self.sandbox.tool_cache().to_string_lossy().into(),
            ),
        ]);
        env
    }
}

/// The variables a runner is expected to provide.
///
/// Actions read these directly — the sccache action asserts `RUNNER_TOOL_CACHE`
/// is defined before it will download anything — and nothing was setting them.
///
/// Deliberately not the whole of [`crate::sandbox::SandboxProfile`]: that also
/// sets `HOME` to the workspace, which is right for a throwaway hosted runner
/// and wrong here, where it would point rustup and cargo away from the caller's
/// real home. Paths come from the sandbox, which already made them.
fn runner_env(
    sandbox: &dyn crate::sandbox::ExecutionsEnvironment,
    workdir: &std::path::Path,
) -> HashMap<String, String> {
    let plat = crate::platform::Platform::current();
    HashMap::from([
        ("CI".to_string(), "true".to_string()),
        ("RUNNER_OS".to_string(), plat.os.runner_os().to_string()),
        (
            "RUNNER_ARCH".to_string(),
            plat.arch.runner_arch().to_string(),
        ),
        ("RUNNER_TEMP".to_string(), absolute(&sandbox.temp_dir())),
        (
            "RUNNER_TOOL_CACHE".to_string(),
            absolute(&sandbox.tool_cache()),
        ),
        ("GITHUB_WORKSPACE".to_string(), absolute(workdir)),
    ])
}

/// A path an action can hand to something else and expect to work.
///
/// These are exported for actions to build paths from, and an action that
/// installs a tool adds the result to PATH — the sccache action wrote
/// `.act-on/tool-cache/sccache/0.17.0/arm64` there. A relative PATH entry only
/// resolves against whatever directory a process happens to be in, and cargo
/// spawns rustc from several, so the wrapper was never found.
fn absolute(path: &std::path::Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| {
            // Not created yet: make it absolute without requiring it to exist.
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        })
        .to_string_lossy()
        .into_owned()
}
