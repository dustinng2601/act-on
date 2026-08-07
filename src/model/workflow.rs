//! Workflow, Job, Step, Strategy, ContainerSpec — mirrors of GH Actions schema.
//!
//! `on:` is stored as `RawOn` (a `serde_yaml::Value`) so we preserve the
//! scalar / sequence / mapping shapes and decode friendlier APIs on top.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

/// A workflow (top-level `.github/workflows/foo.yml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: Option<String>,
    /// Raw `on:` — scalar / sequence / mapping.
    #[serde(default, rename = "on")]
    pub raw_on: Option<Value>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub jobs: HashMap<String, Job>,
    #[serde(default)]
    pub defaults: Option<Defaults>,
    #[serde(default)]
    pub permissions: Option<Value>,
    #[serde(default, skip)]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default)]
    pub run: Option<DefaultsRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsRun {
    pub shell: Option<String>,
    #[serde(rename = "working-directory")]
    pub working_directory: Option<String>,
}

/// A job inside a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub name: Option<String>,
    /// Raw `needs:` (scalar / sequence).
    #[serde(default, rename = "needs")]
    pub raw_needs: Option<Value>,
    /// Raw `runs-on:` (scalar / sequence / mapping `{group, labels}`).
    #[serde(default, rename = "runs-on")]
    pub raw_runs_on: Option<Value>,
    #[serde(default)]
    pub env: Option<Value>,
    #[serde(default, rename = "if")]
    pub if_expr: Option<String>,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default, rename = "timeout-minutes")]
    pub timeout_minutes: Option<f64>,
    #[serde(default)]
    pub strategy: Option<Strategy>,
    #[serde(default)]
    pub services: HashMap<String, ContainerSpec>,
    #[serde(default)]
    pub container: Option<Value>,
    #[serde(default)]
    pub defaults: Option<Defaults>,
    #[serde(default)]
    pub outputs: HashMap<String, String>,
    #[serde(default)]
    pub uses: Option<String>,
    #[serde(default)]
    pub with: HashMap<String, Value>,
    #[serde(default, rename = "continue-on-error")]
    pub continue_on_error: Option<Value>,
    /// '{ "inherit": true }' or a secret-name → value-secret mapping.
    #[serde(default)]
    pub secrets: Option<Value>,
    #[serde(default)]
    pub environment: Option<Value>,
    /// Set by the runner after the job executes: "success" / "failure" /
    /// "cancelled" / "skipped".
    #[serde(default, skip)]
    pub result: String,
}

impl Job {
    pub fn needs(&self) -> Vec<String> {
        match &self.raw_needs {
            Some(Value::String(s)) => vec![s.clone()],
            Some(Value::Sequence(seq)) => seq
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        }
    }

    pub fn runs_on(&self) -> Vec<String> {
        match &self.raw_runs_on {
            Some(Value::String(s)) => vec![s.clone()],
            Some(Value::Sequence(seq)) => seq
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            Some(Value::Mapping(m)) => m
                .get(Value::String("labels".into()))
                .and_then(|v| v.as_sequence())
                .map(|s| {
                    s.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Decide the job kind.
    pub fn kind(&self) -> JobType {
        if let Some(uses) = &self.uses {
            if uses.starts_with("./.github/workflows/") || uses.starts_with(".github/workflows/") {
                return JobType::ReusableWorkflowLocal;
            }
            if uses.contains("/.github/workflows/") {
                return JobType::ReusableWorkflowRemote;
            }
        }
        JobType::Default
    }
}

/// `strategy:` block.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Strategy {
    #[serde(default, rename = "fail-fast")]
    pub fail_fast: Option<bool>,
    #[serde(default, rename = "max-parallel")]
    pub max_parallel: Option<i64>,
    /// Raw `matrix:` — assignment of `name -> [values...]` plus the
    /// `include` / `exclude` directives.
    #[serde(default)]
    pub matrix: Option<Value>,
}

/// Container spec (job.container or job.services.<name>).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    pub image: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub ports: Vec<Value>,
    #[serde(default)]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub options: Option<String>,
    #[serde(default)]
    pub credentials: Option<Credentials>,
    #[serde(default)]
    pub env_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// A step inside a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "if")]
    pub if_expr: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub uses: Option<String>,
    #[serde(default)]
    pub run: Option<String>,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default, rename = "working-directory")]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub env: Option<Value>,
    #[serde(default)]
    pub with: Option<HashMap<String, Value>>,
    #[serde(default, rename = "continue-on-error")]
    pub continue_on_error: Option<Value>,
    #[serde(default, rename = "timeout-minutes")]
    pub timeout_minutes: Option<f64>,
}

impl Step {
    /// Decide the step kind.
    pub fn kind(&self) -> StepType {
        if self.run.is_some() {
            return StepType::Run;
        }
        match &self.uses {
            Some(s) => {
                if s.starts_with("docker://") {
                    StepType::UsesDocker
                } else if s.starts_with("./") {
                    if s.contains("/.github/workflows/") {
                        StepType::ReusableWorkflowLocal
                    } else {
                        StepType::UsesActionLocal
                    }
                } else {
                    if s.contains("/.github/workflows/") {
                        StepType::ReusableWorkflowRemote
                    } else {
                        StepType::UsesActionRemote
                    }
                }
            }
            None => StepType::Invalid,
        }
    }
}

/// Mapping the configured shell string -> argv templates.
pub fn shell_command(shell: &str) -> Option<String> {
    let s = shell.trim();
    let result = match s {
        "bash" => "bash --noprofile --norc -e -o pipefail {0}".to_string(),
        "sh" => "sh -e {0}".to_string(),
        "python" => "python {0}".to_string(),
        "pwsh" => "pwsh -command . '{0}'".to_string(),
        "powershell" => "powershell -command . '{0}'".to_string(),
        "cmd" => r#"cmd /D /E:ON /V:OFF /S /C "CALL \"{0}\"""#.to_string(),
        _ => {
            // Allow `bash {0}` style custom strings.
            if s.contains("{0}") {
                s.to_string()
            } else {
                return Some(s.to_string());
            }
        }
    };
    Some(result)
}

/// Step "kind" enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepType {
    Invalid,
    Run,
    UsesDocker,
    UsesActionLocal,
    UsesActionRemote,
    ReusableWorkflowLocal,
    ReusableWorkflowRemote,
}

impl StepType {
    pub fn is_action(&self) -> bool {
        matches!(
            self,
            StepType::UsesActionLocal | StepType::UsesActionRemote | StepType::UsesDocker
        )
    }
}

/// Job "kind" enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobType {
    Default,
    ReusableWorkflowLocal,
    ReusableWorkflowRemote,
    Invalid,
}

/// Step-level env assembled for execution.
pub type StepEnv = HashMap<String, String>;

/// Available `strategy.matrix` representation; produced by
/// [`crate::util::cartesian`].
#[derive(Debug, Clone, Default)]
pub struct StrategyKind {
    pub combinations: Vec<HashMap<String, Value>>,
    pub fail_fast: bool,
    pub max_parallel: i64,
}
