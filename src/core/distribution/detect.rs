//! Install-method detection: who owns the binary we are running from?
//!
//! The self-managed engine only ever mutates `SelfManaged` installs.
//! Brew-managed and system-package binaries are detected so every
//! mutating verb refuses cleanly instead of creating a second `rtk`
//! copy on PATH (the silent-shadowing bug this feature exists to prevent).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const SYSTEM_DIRS: &[&str] = &[
    "/usr/bin/",
    "/usr/sbin/",
    "/bin/",
    "/sbin/",
    "/usr/local/sbin/",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallMethod {
    BrewManaged { path: PathBuf },
    SelfManaged { path: PathBuf },
    SystemPackage { path: PathBuf },
}

impl InstallMethod {
    pub fn path(&self) -> &Path {
        match self {
            InstallMethod::BrewManaged { path }
            | InstallMethod::SelfManaged { path }
            | InstallMethod::SystemPackage { path } => path,
        }
    }

    pub fn describe(&self) -> &'static str {
        match self {
            InstallMethod::BrewManaged { .. } => "brew-managed",
            InstallMethod::SelfManaged { .. } => "self-managed",
            InstallMethod::SystemPackage { .. } => "system package",
        }
    }
}

pub fn detect() -> Result<InstallMethod> {
    let exe = std::env::current_exe().context("Failed to resolve own executable path")?;
    // A brew-linked bin/rtk canonicalizes into the Cellar.
    let real = exe.canonicalize().unwrap_or(exe);
    let writable = real.parent().is_some_and(dir_writable);
    Ok(classify(
        &real,
        std::env::var("HOMEBREW_PREFIX").ok().as_deref(),
        writable,
    ))
}

fn classify(real: &Path, homebrew_prefix: Option<&str>, dir_writable: bool) -> InstallMethod {
    let s = real.to_string_lossy();
    let path = real.to_path_buf();

    let brew = s.contains("/Cellar/")
        || s.starts_with("/opt/homebrew/")
        || s.starts_with("/home/linuxbrew/.linuxbrew/")
        || homebrew_prefix.is_some_and(|p| !p.is_empty() && s.starts_with(p));
    if brew {
        return InstallMethod::BrewManaged { path };
    }

    // The swap engine needs a writable parent dir (temp file + rename);
    // an unwritable one means a package-manager or root install.
    if SYSTEM_DIRS.iter().any(|p| s.starts_with(p)) || !dir_writable {
        return InstallMethod::SystemPackage { path };
    }

    InstallMethod::SelfManaged { path }
}

fn dir_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".rtk-write-probe-{}", std::process::id()));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_self_managed_home() {
        let m = classify(Path::new("/home/dev/.local/bin/rtk"), None, true);
        assert!(matches!(m, InstallMethod::SelfManaged { .. }));
    }

    #[test]
    fn test_classify_brew_cellar() {
        for path in [
            "/opt/homebrew/Cellar/rtk/0.43.0/bin/rtk",
            "/usr/local/Cellar/rtk/0.43.0/bin/rtk",
            "/home/linuxbrew/.linuxbrew/Cellar/rtk/0.43.0/bin/rtk",
        ] {
            let m = classify(Path::new(path), None, true);
            assert!(matches!(m, InstallMethod::BrewManaged { .. }), "{}", path);
        }
    }

    #[test]
    fn test_classify_homebrew_prefix_env() {
        let m = classify(
            Path::new("/custom/brew/bin/rtk"),
            Some("/custom/brew"),
            true,
        );
        assert!(matches!(m, InstallMethod::BrewManaged { .. }));
    }

    #[test]
    fn test_classify_empty_homebrew_prefix_ignored() {
        let m = classify(Path::new("/home/dev/.local/bin/rtk"), Some(""), true);
        assert!(matches!(m, InstallMethod::SelfManaged { .. }));
    }

    #[test]
    fn test_classify_system_dir() {
        let m = classify(Path::new("/usr/bin/rtk"), None, false);
        assert!(matches!(m, InstallMethod::SystemPackage { .. }));
    }

    #[test]
    fn test_classify_unwritable_dir_is_system() {
        let m = classify(Path::new("/opt/tools/rtk"), None, false);
        assert!(matches!(m, InstallMethod::SystemPackage { .. }));
    }

    #[test]
    fn test_dir_writable_probe() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(dir_writable(tmp.path()));
    }
}
