//! File-command glue (`GITHUB_OUTPUT` / `GITHUB_ENV` / `GITHUB_PATH`
//! / `GITHUB_STATE` / `GITHUB_STEP_SUMMARY`).
//!
//! Re-exports the ready-to-use [`crate::workflow_cmd::FileCommands`].
//!
//! This module also defines the env-var names GitHub uses.

pub use crate::workflow_cmd::FileCommands;

pub const GITHUB_OUTPUT: &str = "GITHUB_OUTPUT";
pub const GITHUB_ENV: &str = "GITHUB_ENV";
pub const GITHUB_PATH: &str = "GITHUB_PATH";
pub const GITHUB_STATE: &str = "GITHUB_STATE";
pub const GITHUB_STEP_SUMMARY: &str = "GITHUB_STEP_SUMMARY";
pub const GITHUB_EVENT_PATH: &str = "GITHUB_EVENT_PATH";

/// Set the `GITHUB_*` env vars in `env` to point at the file-command files.
pub fn populate_env(env: &mut std::collections::HashMap<String, String>, fc: &FileCommands) {
    env.insert(GITHUB_OUTPUT.into(), fc.output.to_string_lossy().into());
    env.insert(GITHUB_ENV.into(), fc.env.to_string_lossy().into());
    env.insert(GITHUB_PATH.into(), fc.path.to_string_lossy().into());
    env.insert(GITHUB_STATE.into(), fc.state.to_string_lossy().into());
    env.insert(
        GITHUB_STEP_SUMMARY.into(),
        fc.summary.to_string_lossy().into(),
    );
}
