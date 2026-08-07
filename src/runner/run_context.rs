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
        let actpath = workdir.join(".act-on");
        Self {
            config,
            workflow,
            job_id,
            job,
            matrix,
            result: Mutex::new(String::new()),
            step_results: Arc::new(Mutex::new(HashMap::new())),
            masks: Arc::new(Mutex::new(Vec::new())),
            env: Arc::new(Mutex::new(HashMap::new())),
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
            workflow: self
                .workflow
                .file
                .clone()
                .unwrap_or_default(),
            workspace: self.workdir.to_string_lossy().into(),
            event_name: self.config.event_name.clone(),
            actor: self.config.actor.clone(),
            repository: String::new(),
            repository_owner: String::new(),
            run_id: uuid::Uuid::new_v4().to_string(),
            run_number: 1,
            run_attempt: 1,
            server_url: "https://github.com".into(),
            api_url: "https://api.github.com".into(),
            graphql_url: "https://api.github.com/graphql".into(),
            ..Default::default()
        };
        env.env = self.env.lock().clone();
        env.steps = self.step_results.lock().clone();
        env.secrets = self.config.secrets_map();
        env.vars = self.config.vars_map();
        env.inputs = self.config.inputs_map();
        env.matrix = self.matrix.clone();
        env.runner = HashMap::from([
            ("os".into(), self.sandbox.runner_os().into()),
            ("arch".into(), self.sandbox.runner_arch().into()),
            ("temp".into(), self.sandbox.temp_dir().to_string_lossy().into()),
            (
                "tool_cache".into(),
                self.sandbox.tool_cache().to_string_lossy().into(),
            ),
        ]);
        env
    }
}
