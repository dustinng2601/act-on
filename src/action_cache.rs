//! Action cache — clone `org/repo@ref` to a local directory and look it up
//! later.

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::config::Config;

/// Parsed `org/repo/path@ref` token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRef {
    pub org: String,
    pub repo: String,
    pub path: String,
    pub rref: String,
}

/// `uses: org/repo/path@ref` parser.
///
/// Accepts `actions/checkout@v4`, `nick-fields/retry@v3`, `org/repo/sub/dir@main-tag`,
/// `./local/path` (returns `None`), and `docker://image:tag` (returns `None`).
pub fn parse_uses(uses: &str) -> Option<RemoteRef> {
    if uses.starts_with("./") || uses.starts_with("docker://") {
        return None;
    }
    let (path_part, rref) = match uses.split_once('@') {
        Some((p, r)) => (p, r.to_string()),
        None => return None,
    };
    let mut parts = path_part.splitn(3, '/');
    let org = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    let path = parts.next().unwrap_or("").to_string();
    Some(RemoteRef {
        org,
        repo,
        path,
        rref,
    })
}

/// Cache keyed by `org/repo/path@ref` -> on-disk path.
pub struct ActionCache {
    root: PathBuf,
    inner: Mutex<std::collections::HashMap<String, PathBuf>>,
}

impl ActionCache {
    pub fn new(root: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            inner: Mutex::new(std::collections::HashMap::new()),
        })
    }

    /// Fetch (clone or reuse) an action and return the local directory that
    /// contains its `action.yml`.
    pub async fn fetch(&self, _cfg: &Config, uses: &str) -> anyhow::Result<Option<PathBuf>> {
        let rr = match parse_uses(uses) {
            Some(rr) => rr,
            None => return Ok(None),
        };
        let key = format!("{}-{}-{}-{}", rr.org, rr.repo, rr.path, rr.rref);
        if let Some(p) = self.inner.lock().get(&key) {
            return Ok(Some(p.clone()));
        }

        let safe_key = key.replace(['/', ':', '@'], "_");
        let dest = self.root.join(&safe_key);
        if !dest.exists() {
            std::fs::create_dir_all(&dest)?;
            let url = format!("https://github.com/{}/{}.git", rr.org, rr.repo);
            // Use `git clone` for now — bollard/git2 later. Spawned as a
            // blocking subprocess because git2 is heavy to depend on.
            let out = tokio::process::Command::new("git")
                .arg("clone")
                .arg("--depth")
                .arg("1")
                .arg("--branch")
                .arg(&rr.rref)
                .arg(&url)
                .arg(&dest)
                .output()
                .await;
            match out {
                Ok(o) if o.status.success() => {}
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    if stderr.contains("not found") || stderr.contains("does not match") {
                        // try without --branch (let git resolve ref)
                        let _ = tokio::process::Command::new("git")
                            .arg("clone")
                            .arg("--depth")
                            .arg("1")
                            .arg(&url)
                            .arg(&dest)
                            .output()
                            .await;
                    }
                }
                Err(e) => {
                    tracing::warn!(target: "act_on::cache", "git clone failed: {e}");
                    return Ok(None);
                }
            }
        }

        let dir = if rr.path.is_empty() {
            dest
        } else {
            dest.join(&rr.path)
        };
        self.inner.lock().insert(key, dir.clone());
        Ok(Some(dir))
    }
}

// Static instance via once_cell.
static GLOBAL_CACHE: once_cell::sync::Lazy<Arc<ActionCache>> = once_cell::sync::Lazy::new(|| {
    let root = std::env::var_os("ACT_ON_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut p = dirs_or_temp();
            p.push("act-on");
            p.push("action-cache");
            p
        });
    Arc::new(ActionCache::new(root).expect("cannot create action cache"))
});

/// Get the global [`ActionCache`].
pub fn global() -> Arc<ActionCache> {
    GLOBAL_CACHE.clone()
}

fn dirs_or_temp() -> PathBuf {
    if let Some(p) = std::env::var_os("HOME").map(PathBuf::from) {
        return p;
    }
    std::env::temp_dir()
}

/// Convenience: fetch via the global cache.
pub async fn fetch(cfg: &Config, uses: &str) -> anyhow::Result<Option<PathBuf>> {
    global().fetch(cfg, uses).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_ref() {
        let r = parse_uses("actions/checkout@v4").unwrap();
        assert_eq!(r.org, "actions");
        assert_eq!(r.repo, "checkout");
        assert_eq!(r.rref, "v4");
    }

    #[test]
    fn parses_nested_path() {
        let r = parse_uses("ORG/repo/sub/dir@main").unwrap();
        assert_eq!(r.org, "ORG");
        assert_eq!(r.repo, "repo");
        assert_eq!(r.path, "sub/dir");
        assert_eq!(r.rref, "main");
    }

    #[test]
    fn local_is_none() {
        assert!(parse_uses("./foo/bar").is_none());
    }
    #[test]
    fn docker_is_none() {
        assert!(parse_uses("docker://alpine:3.8").is_none());
    }
}
