//! Runtime configuration assembled from CLI flags, `--actrc` defaults,
//! secret / env / var files, and platform mappings.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Mapping `runs-on` label -> runner image. Equivalent to `act -P`.
///
/// A `-` (or absent value) means "run on host" (no container).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformMapping(pub HashMap<String, String>);

impl PlatformMapping {
    /// Returns the image for `runs-on` label or `None`.
    pub fn image_for(&self, label: &str) -> Option<&str> {
        self.0.get(label).map(|s| s.as_str())
    }

    /// When the resolved label maps to `-self-hosted` (or has no image), the
    /// job should run on the host instead of a container.
    pub fn runs_on_host(&self, label: &str) -> bool {
        match self.0.get(label) {
            None => true,
            Some(v) => v == "-self-hosted" || v.is_empty(),
        }
    }
}

/// A name=value pair that may be supplied on the CLI or read from a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameValue {
    pub name: String,
    pub value: String,
}

impl NameValue {
    pub fn parse(s: &str) -> Self {
        match s.find('=') {
            Some(i) => Self {
                name: s[..i].trim().to_string(),
                value: s[i + 1..].trim_matches(['\'', '"', ' ']).to_string(),
            },
            None => Self {
                name: s.trim().to_string(),
                value: String::new(),
            },
        }
    }
}

/// Parsed env / secret / var / input file (one `KEY=VALUE` per line,
/// `#` comments and blank lines ignored).
pub fn parse_kv_file(contents: &str) -> Vec<NameValue> {
    contents
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(NameValue::parse)
        .collect()
}

/// Top-level `Config`. Built by [`crate::cli::run`] from CLI flags.
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub workdir: PathBuf,
    pub workflows: Vec<PathBuf>,

    pub job: Option<String>,
    pub actor: String,
    pub event_name: String,
    pub event_path: Option<PathBuf>,
    pub event_json: Option<String>,

    pub workflows_inputs: Vec<NameValue>,

    pub secrets: Vec<NameValue>,
    pub vars: Vec<NameValue>,
    pub env: Vec<NameValue>,

    pub platforms: PlatformMapping,

    pub matrix: Vec<String>,
    pub default_branch: String,
    pub github_instance: String,
    pub remote_name: String,

    pub bind_workdir: bool,
    pub reuse_containers: bool,
    pub force_pull: bool,
    pub force_rebuild: bool,
    pub dryrun: bool,
    pub strict: bool,
    pub list: bool,
    pub graph: bool,

    pub concurrent_jobs: usize,
    pub json_logger: bool,
    pub quiet: bool,
    pub verbose: u8,

    pub policy_path: Option<PathBuf>,
}

impl Config {
    /// Resolve `env` map (workflow-level env), interpolated later.
    pub fn env_map(&self) -> HashMap<String, String> {
        self.env.iter().map(|nv| (nv.name.clone(), nv.value.clone())).collect()
    }

    pub fn secrets_map(&self) -> HashMap<String, String> {
        self.secrets.iter().map(|nv| (nv.name.clone(), nv.value.clone())).collect()
    }

    pub fn vars_map(&self) -> HashMap<String, String> {
        self.vars.iter().map(|nv| (nv.name.clone(), nv.value.clone())).collect()
    }

    pub fn inputs_map(&self) -> HashMap<String, String> {
        self.workflows_inputs
            .iter()
            .map(|nv| (nv.name.clone(), nv.value.clone()))
            .collect()
    }
}
