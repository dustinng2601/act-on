//! Device model — who owns what, what platform it provides.

use serde::{Deserialize, Serialize};

use crate::platform::{Arch, Os};

/// A stable identifier for a device within the enterprise. Two devices
/// with the same id across policies are treated as the same machine.
pub type DeviceId = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Device {
    pub id: DeviceId,
    pub os: Os,
    pub arch: Arch,
    /// `runs-on:` labels this device satisfies. Always includes the OS /
    /// arch aliases plus any extra labels the owner declared.
    pub labels: Vec<String>,
    /// How this device can be used by the pool.
    #[serde(default)]
    pub share: crate::pool::ShareMode,
    /// Owner override (defaults to the policy-level `owner`).
    pub owner: Option<String>,
    /// Optional SSH/WinRM endpoint address, so the router can dispatch
    /// remote jobs. When empty the device is assumed local.
    pub endpoint: Option<String>,
}

/// Live status of a device within the pool.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum DeviceStatus {
    #[default]
    Idle,
    Busy,
    Offline,
}

impl Device {
    pub fn matches(&self, labels: &[String]) -> bool {
        // All required labels must be satisfied, loosely.
        for l in labels {
            let l = l.to_lowercase();
            if l == "self-hosted" {
                continue;
            }
            let hit = self.labels.iter().any(|d| d.to_lowercase() == l)
                || self.os.runner_os().eq_ignore_ascii_case(&l)
                || self.arch.runner_arch().eq_ignore_ascii_case(&l);
            if !hit {
                return false;
            }
        }
        true
    }
}
