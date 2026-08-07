//! The `ExecutionsEnvironment` trait — the analogue of nektos/act's
//! `container.ExecutionsEnvironment`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;

pub type ExecResult = std::result::Result<i32, std::io::Error>;

#[async_trait]
pub trait ExecutionsEnvironment: Send + Sync {
    /// Start the environment (no-op for host).
    async fn start(&self) -> anyhow::Result<()>;

    /// Stop and remove the environment.
    async fn stop(&self) -> anyhow::Result<()>;

    /// Execute `argv` inside the environment, returning the exit code.
    async fn exec(
        &self,
        argv: Vec<String>,
        env: HashMap<String, String>,
        workdir: &str,
    ) -> ExecResult;

    /// Copy a directory into the sandbox (used by `actions/checkout`
    /// short-circuit and `uses: ./local`).
    async fn copy_dir(&self, src: &Path, dst: &str) -> anyhow::Result<()>;

    /// The workspace path inside the sandbox.
    fn workspace(&self) -> PathBuf;

    /// The "act scratch" path inside the sandbox (e.g. `/act` on Linux).
    fn act_path(&self) -> PathBuf;

    /// The temp dir (`runner.temp`).
    fn temp_dir(&self) -> PathBuf;

    /// The tool cache dir (`runner.tool_cache`).
    fn tool_cache(&self) -> PathBuf;

    /// `runner.os` (e.g. "Linux").
    fn runner_os(&self) -> &'static str;

    /// `runner.arch` (e.g. "X64").
    fn runner_arch(&self) -> &'static str;
}
