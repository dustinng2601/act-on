//! DockerExecutionEnvironment — the analogue of nektos/act's
//! `pkg/container/docker_run.go`.
//!
//! The actual container plumbing (pull / create / exec / copy) uses
//! `bollard` (the moby client for Rust). For the v0.1 milestone, we ship a
//! stub implementation that surfaces a clear "not enabled" error, so the
//! rest of the runner is testable end-to-end on the host. The full Docker
//! driver is tracked behind v1.0.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;

use super::env::{ExecResult, ExecutionsEnvironment};

/// Docker sandbox stub.
pub struct DockerEnvironmentStub {
    pub image: String,
    pub os: &'static str,
    pub arch: &'static str,
}

#[async_trait]
impl ExecutionsEnvironment for DockerEnvironmentStub {
    async fn start(&self) -> anyhow::Result<()> {
        anyhow::bail!(
            "docker sandbox is not enabled in v0.1; build with `--features docker` to enable the bollard backend"
        )
    }
    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn exec(
        &self,
        _argv: Vec<String>,
        _env: HashMap<String, String>,
        _workdir: &str,
    ) -> ExecResult {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "docker sandbox not enabled",
        ))
    }
    async fn copy_dir(&self, _src: &Path, _dst: &str) -> anyhow::Result<()> {
        anyhow::bail!("docker sandbox not enabled")
    }
    fn workspace(&self) -> PathBuf {
        PathBuf::from("/github/workspace")
    }
    fn act_path(&self) -> PathBuf {
        PathBuf::from("/act")
    }
    fn temp_dir(&self) -> PathBuf {
        PathBuf::from("/home/runner/_temp")
    }
    fn tool_cache(&self) -> PathBuf {
        PathBuf::from("/opt/hostedtoolcache")
    }
    fn runner_os(&self) -> &'static str {
        self.os
    }
    fn runner_arch(&self) -> &'static str {
        self.arch
    }
}
