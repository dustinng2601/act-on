//! Sandbox profile: per-OS environment defaults that aim to match GitHub's
//! hosted runner images (`ubuntu-*`, `windows-*`, `macos-*`).

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SandboxProfile {
    pub os: crate::platform::Os,
    pub arch: crate::platform::Arch,
    pub workspace: PathBuf,
    pub temp: PathBuf,
    pub tool_cache: PathBuf,
    pub env: HashMap<String, String>,
}

impl SandboxProfile {
    /// Build the GitHub-hosted-style profile for a given platform.
    pub fn hosted(plat: crate::platform::Platform, workdir: &std::path::Path) -> Self {
        let (workspace, temp, tool_cache) = match plat.os {
            crate::platform::Os::Linux | crate::platform::Os::Windows => {
                let workspace = workdir.to_path_buf();
                let temp = std::env::temp_dir().join("runner").join("_temp");
                let tool_cache = std::env::temp_dir().join("runner").join("tool-cache");
                (workspace, temp, tool_cache)
            }
            crate::platform::Os::MacOS => {
                let workspace = workdir.to_path_buf();
                let temp = std::env::temp_dir().join("runner").join("_temp");
                let tool_cache = std::env::temp_dir().join("runner").join("tool-cache");
                (workspace, temp, tool_cache)
            }
        };

        let env = HashMap::from([
            ("HOME".into(), workspace.to_string_lossy().into()),
            ("CI".into(), "true".into()),
            ("RUNNER_OS".into(), plat.os.runner_os().into()),
            ("RUNNER_ARCH".into(), plat.arch.runner_arch().into()),
            ("RUNNER_TEMP".into(), temp.to_string_lossy().into()),
            (
                "RUNNER_TOOL_CACHE".into(),
                tool_cache.to_string_lossy().into(),
            ),
        ]);

        Self {
            os: plat.os,
            arch: plat.arch,
            workspace,
            temp,
            tool_cache,
            env,
        }
    }
}
