//! `git` utilities: figure out repo URL, ref, sha from a working directory.

use std::path::{Path, PathBuf};
use std::process::Command;

/// `git rev-parse --show-toplevel`.
pub fn workdir_root(start: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(start)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}

/// `git rev-parse HEAD`.
pub fn head_sha(start: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(start)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `git symbolic-ref --short HEAD` (returns "main" / "master" / ...).
pub fn current_branch(start: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(start)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `git remote get-url origin`.
pub fn remote_url(start: &Path, remote: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["remote", "get-url", remote])
        .current_dir(start)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
