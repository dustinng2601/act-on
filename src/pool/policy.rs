//! `policy.yml` types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub version: u32,
    pub owner: String,
    #[serde(default)]
    pub devices: Vec<crate::pool::device::Device>,
    #[serde(default)]
    pub fallback: Fallback,
    /// When `true` (default), `act-on` will first try the device pool then
    /// fall back to GitHub CI; when `false`, owned-devices first.
    #[serde(default)]
    pub prefer_pool: bool,
    /// Per-owner concurrency budget (-1 = unlimited).
    #[serde(default)]
    pub max_concurrent_jobs_per_owner: i64,
    /// Per-job global timeout override (minutes). 0 = let the workflow decide.
    #[serde(default)]
    pub default_timeout_minutes: i64,
    /// Arbitrary labels -> pool endpoint URL.
    #[serde(default)]
    pub pool_endpoints: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Fallback {
    /// What to do when no local + pool device matches `runs-on:`.
    pub missing_platform: FallbackStrategy,
    /// Optional shared enterprise pool endpoint URL.
    #[serde(default)]
    pub pool_endpoint: Option<String>,
    /// Optional GitHub CI endpoint (defaults to https://api.github.com).
    #[serde(default)]
    pub github_endpoint: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FallbackStrategy {
    #[default]
    Github,
    Pool,
    Queue,
    Fail,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShareMode {
    /// Only the owner can use the device.
    Exclusive,
    /// The owner always wins; pool borrows only when idle.
    #[default]
    Pool,
    /// Anyone in the enterprise pool can borrow.
    Open,
}

impl Default for Policy {
    /// An empty policy: no devices, nothing shared, no limits.
    ///
    /// `version` starts at 1 rather than 0 so a default-constructed policy is
    /// indistinguishable from a v1 file, which is what it is.
    ///
    /// Note `prefer_pool` is false here, matching what `#[serde(default)]`
    /// produces for a policy file that omits it. The doc comment on the field
    /// says the default is true; the two disagree, and changing either would
    /// change how existing policies route, so this only matches the behaviour
    /// rather than picking a side.
    fn default() -> Self {
        Self {
            version: 1,
            owner: String::new(),
            devices: Vec::new(),
            fallback: Fallback::default(),
            prefer_pool: false,
            max_concurrent_jobs_per_owner: 0,
            default_timeout_minutes: 0,
            pool_endpoints: HashMap::new(),
        }
    }
}

impl Policy {
    /// Read and parse a `policy.yml`.
    pub fn from_path(path: &std::path::Path) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path)?;
        let p: Policy = serde_yaml::from_slice(&bytes)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        Ok(p)
    }
}
