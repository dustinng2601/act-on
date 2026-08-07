//! Execution contexts: `github`, `job`, `steps.<id>.*`, etc.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

/// Mirrors of GitHub's `runner.os` / `runner.arch` / `runner.temp` /
/// `runner.tool_cache`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunnerContext {
    pub os: String,
    pub arch: String,
    pub temp: String,
    pub tool_cache: String,
}

/// `steps.<id>.*` outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum StepStatus {
    #[default]
    Success,
    Failure,
    Skipped,
}

impl std::fmt::Display for StepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            StepStatus::Success => "success",
            StepStatus::Failure => "failure",
            StepStatus::Skipped => "skipped",
        })
    }
}

/// Per-step result backing `steps.<id>.outputs/conclusion/outcome`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepResult {
    #[serde(default)]
    pub outputs: HashMap<String, String>,
    #[serde(default)]
    pub conclusion: StepStatus,
    #[serde(default)]
    pub outcome: StepStatus,
}

impl StepResult {
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
            conclusion: StepStatus::Success,
            outcome: StepStatus::Success,
        }
    }
}

/// Job-level `job.status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum JobStatus {
    #[default]
    Success,
    Failure,
    Cancelled,
    Skipped,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            JobStatus::Success => "success",
            JobStatus::Failure => "failure",
            JobStatus::Cancelled => "cancelled",
            JobStatus::Skipped => "skipped",
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobContext {
    pub status: JobStatus,
}

/// `needs.<job>.outputs.*` / `needs.<job>.result`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Needs {
    pub outputs: HashMap<String, String>,
    pub result: JobStatus,
}

/// Top-level `github.*` context. Subset of `model.GithubContext` in `act`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GithubContext {
    pub event: HashMap<String, Value>,
    pub event_path: String,
    pub workflow: String,
    pub run_id: String,
    pub run_number: i64,
    pub run_attempt: i64,
    pub actor: String,
    pub repository: String,
    pub repository_owner: String,
    pub event_name: String,
    pub sha: String,
    pub ref_: String,
    pub head_ref: String,
    pub base_ref: String,
    pub ref_name: String,
    pub ref_type: String,
    pub workspace: String,
    pub action: String,
    pub action_path: String,
    pub action_ref: String,
    pub action_repository: String,
    pub token: String,
    pub server_url: String,
    pub api_url: String,
    pub graphql_url: String,
    pub job: String,
    pub job_name: String,
    pub retention_days: i64,
    pub runner_perflog: String,
    pub runner_tracking_id: String,
}

/// Final outcome of a job after running.
pub type Result = JobStatus;
