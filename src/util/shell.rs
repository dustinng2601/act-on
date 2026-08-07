//! Shell helpers: default shell per OS, command-string split.

use crate::runner::RunContext;
use crate::sandbox::ExecutionsEnvironment;
use crate::platform::Os;

/// Default shell for the current sandbox. On Windows defaults to `pwsh`
/// (matching GitHub), on Linux/macOS to `bash`.
pub fn default_shell(_rc: &RunContext) -> String {
    // Respect `defaults.run.shell` from the workflow when set (handled by
    // the caller via step.shell). Here we only fall back.
    match Os::current() {
        Os::Windows => "pwsh".into(),
        _ => "bash".into(),
    }
}

/// File extension for a temporary shell script given a shell name.
pub fn script_extension(shell: &str) -> &'static str {
    match shell {
        "pwsh" | "powershell" => "ps1",
        "cmd" => "cmd",
        "python" => "py",
        "sh" => "sh",
        _ => "sh",
    }
}

/// Split a command string into argv, honouring shell quoting.
pub fn split(s: &str) -> Vec<String> {
    shell_words::split(s).unwrap_or_else(|_| vec![s.into()])
}

/// Look up an executable in `PATH`. Returns the full path or the original.
pub fn which(cmd: &str) -> Option<std::path::PathBuf> {
    if cmd.contains(std::path::MAIN_SEPARATOR) || cfg!(windows) && cmd.contains('/') {
        let p = std::path::PathBuf::from(cmd);
        if p.exists() {
            return Some(p);
        }
    }
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

// Windows-only: the module does not exist elsewhere, so this needs a cfg rather
// than an allow — `allow(unused_imports)` silences a warning, it does not stop
// the path being resolved.
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
