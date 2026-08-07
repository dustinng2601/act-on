//! OS / arch detection.

use std::env::consts;

/// Operating system kind. Mirrors GitHub's `runner.os` enum, plus a `Host`
/// alias used when the runner is the local host itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Windows,
    Linux,
    MacOS,
}

impl Os {
    /// Best-effort detect at compile time.
    pub const fn current() -> Self {
        #[cfg(target_os = "windows")]
        {
            Os::Windows
        }
        #[cfg(target_os = "linux")]
        {
            Os::Linux
        }
        #[cfg(target_os = "macos")]
        {
            Os::MacOS
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            Os::Linux
        }
    }

    /// GitHub's `runner.os` string.
    pub fn runner_os(&self) -> &'static str {
        match self {
            Os::Windows => "Windows",
            Os::Linux => "Linux",
            Os::MacOS => "macOS",
        }
    }
}

impl std::fmt::Display for Os {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.runner_os())
    }
}

/// Architecture kind mirroring GitHub's `runner.arch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    X64,
    Arm64,
    X86,
    Other,
}

impl Arch {
    pub const fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Arch::X64
        }
        #[cfg(target_arch = "aarch64")]
        {
            Arch::Arm64
        }
        #[cfg(target_arch = "x86")]
        {
            Arch::X86
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "x86")))]
        {
            Arch::Other
        }
    }

    pub fn runner_arch(&self) -> &'static str {
        match self {
            Arch::X64 => "X64",
            Arch::Arm64 => "ARM64",
            Arch::X86 => "X86",
            Arch::Other => "OTHER",
        }
    }
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.runner_arch())
    }
}

/// Detected host platform.
#[derive(Debug, Clone, Copy)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    pub fn current() -> Self {
        Self {
            os: Os::current(),
            arch: Arch::current(),
        }
    }
}

/// Match a list of `runs-on:` labels to a platform hint.
///
/// Accepts both GitHub-hosted aliases (`ubuntu-latest`, `windows-latest`,
/// `macos-latest`, `ubuntu-22.04`, `windows-2022`, `macos-14`,
/// `macos-13-arm`, ...) and self-hosted labels (`self-hosted`, `linux`,
/// `windows`, `macos`, `x64`, `arm64`).
pub fn platform_of_labels(labels: &[String]) -> Option<Platform> {
    let mut os: Option<Os> = None;
    let mut arch: Option<Arch> = None;

    for l in labels {
        let l = l.to_lowercase();
        let l = l.as_str();
        if l == "linux" || l == "ubuntu" || l.starts_with("ubuntu-") {
            os = Some(Os::Linux);
        } else if l == "windows" || l == "win" || l.starts_with("windows-") {
            os = Some(Os::Windows);
        } else if l == "macos" || l == "mac" || l.starts_with("macos-") {
            os = Some(Os::MacOS);
        } else if l == "x64" || l == "amd64" || l == "x86_64" {
            arch = Some(Arch::X64);
        } else if l == "arm64" || l == "aarch64" {
            arch = Some(Arch::Arm64);
        } else if l == "x86" {
            arch = Some(Arch::X86);
        }
    }

    // ubuntu-* defaults to x64
    if os == Some(Os::Linux) && arch.is_none() {
        arch = Some(Arch::X64);
    }
    // windows-* defaults to x64
    if os == Some(Os::Windows) && arch.is_none() {
        arch = Some(Arch::X64);
    }
    // macos-13 (intel) defaults to x64, macos-14 (apple silicon) to arm64
    if os == Some(Os::MacOS) && arch.is_none() {
        arch = Some(Arch::X64);
    }

    let os = os?;
    Some(Platform {
        os,
        arch: arch.unwrap_or(Arch::X64),
    })
}

pub use consts::{ARCH as TARGET_ARCH_STR, OS as TARGET_OS_STR};

impl Default for Os {
    /// The machine this is running on.
    ///
    /// A `Device` with no OS stated is the local one — that is the only reading
    /// that is ever right, and picking a variant arbitrarily would silently
    /// describe someone else's machine.
    fn default() -> Self {
        Self::current()
    }
}

impl Default for Arch {
    /// The architecture this is running on, for the same reason as [`Os`].
    fn default() -> Self {
        Self::current()
    }
}
