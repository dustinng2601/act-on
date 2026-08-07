//! HostExecutionEnvironment — runs commands directly on the local host via
//! `tokio::process::Command`. This is the analogue of nektos/act
//! `HostEnvironment` (`pkg/container/host_environment.go`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::process::Command;

use super::env::{ExecResult, ExecutionsEnvironment};
use crate::platform::{Arch, Os};

/// Run everything on the local host (no container).
pub struct HostEnvironment {
    workdir: PathBuf,
    actpath: PathBuf,
    temp: PathBuf,
    tool_cache: PathBuf,
    os: Os,
    arch: Arch,
}

impl HostEnvironment {
    pub fn new(workdir: PathBuf) -> std::io::Result<Self> {
        let actpath = workdir.join(".act-on");
        std::fs::create_dir_all(&actpath)?;
        let temp = std::env::temp_dir().join("act-on").join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&temp)?;
        let tool_cache = actpath.join("tool-cache");
        std::fs::create_dir_all(&tool_cache)?;
        Ok(Self {
            workdir,
            actpath,
            temp,
            tool_cache,
            os: Os::current(),
            arch: Arch::current(),
        })
    }
}

#[async_trait]
impl ExecutionsEnvironment for HostEnvironment {
    async fn start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn exec(
        &self,
        argv: Vec<String>,
        env: HashMap<String, String>,
        workdir: &str,
    ) -> ExecResult {
        if argv.is_empty() {
            return Ok(0);
        }
        let mut cmd = Command::new(&argv[0]);
        cmd.args(&argv[1..]);
        cmd.envs(env);
        let cwd = if workdir.is_empty() {
            self.workdir.clone()
        } else {
            PathBuf::from(workdir)
        };
        cmd.current_dir(&cwd);
        let status = cmd.status().await?;
        Ok(status.code().unwrap_or(127))
    }

    async fn copy_dir(&self, src: &Path, dst: &str) -> anyhow::Result<()> {
        let dst = if dst.is_empty() {
            self.workdir.clone()
        } else {
            PathBuf::from(dst)
        };

        // Copying a directory onto itself destroys it: the walk reads each file
        // while the copy truncates the same path, so every file is emptied. On
        // this sandbox `workspace()` *is* the working directory, so a checkout
        // asks for exactly that, and there is genuinely nothing to do — the
        // files are already where they need to be.
        if same_dir(src, &dst) {
            tracing::debug!(
                target: "act_on::sandbox",
                "copy_dir skipped: {} is already the destination",
                src.display()
            );
            return Ok(());
        }

        std::fs::create_dir_all(&dst)?;
        copy_recursive(src, &dst)?;
        Ok(())
    }

    fn workspace(&self) -> PathBuf {
        self.workdir.clone()
    }

    fn act_path(&self) -> PathBuf {
        self.actpath.clone()
    }

    fn temp_dir(&self) -> PathBuf {
        self.temp.clone()
    }

    fn tool_cache(&self) -> PathBuf {
        self.tool_cache.clone()
    }

    fn runner_os(&self) -> &'static str {
        self.os.runner_os()
    }

    fn runner_arch(&self) -> &'static str {
        self.arch.runner_arch()
    }
}

fn copy_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let target = dst.join(&name);
        if path.is_dir() {
            copy_recursive(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

/// Whether two paths name the same directory.
///
/// Compared after canonicalising so `.`, a trailing slash, or a symlinked
/// checkout do not read as different places. A path that cannot be canonicalised
/// does not exist yet, and cannot be the one being copied from.
fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::env::ExecutionsEnvironment;

    /// The bug this guards against emptied every file in the checkout.
    ///
    /// `workspace()` is the working directory on this sandbox, so the
    /// `actions/checkout` short-circuit asked to copy the tree onto itself. The
    /// walk read each file while the copy truncated the same path, and a real
    /// repository came back with 251 files changed and its workflow emptied.
    #[tokio::test]
    async fn copying_a_directory_onto_itself_leaves_it_alone() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let file = root.join("keep.txt");
        std::fs::write(&file, "contents that must survive").unwrap();

        let sandbox = HostEnvironment::new(root.clone()).unwrap();
        sandbox
            .copy_dir(&root, root.to_string_lossy().as_ref())
            .await
            .expect("copying onto itself should be a no-op, not an error");

        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "contents that must survive"
        );
    }
}
