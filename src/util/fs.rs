//! Common fs helpers.

use std::path::Path;

pub fn ensure_dir(p: &Path) -> std::io::Result<()> {
    if !p.exists() {
        std::fs::create_dir_all(p)?;
    }
    Ok(())
}

pub fn touch(p: &Path) -> std::io::Result<()> {
    if !p.exists() {
        std::fs::write(p, "")?;
    }
    Ok(())
}
