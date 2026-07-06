//! Self-managed install lifecycle: `rtk update` / `upgrade` / `downgrade`
//! / `uninstall`.
//!
//! One engine drives every flow: resolve the release on the edition's
//! channel, verify, atomically swap the binary at its own path. The `rtk`
//! name always has exactly one owner — mutating verbs refuse on
//! brew-managed or system-package installs instead of creating a second
//! copy on PATH (Homebrew transitions arrive in a later phase).

pub mod channel;
pub mod detect;
pub mod platform;
pub mod swap;
#[cfg(test)]
pub(crate) mod test_http;
pub mod version;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use channel::Channel;
use detect::InstallMethod;

const LOGIN_HINT: &str = "rtk login --endpoint <your tenant URL>";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    Oss,
    Plus,
}

impl Edition {
    fn channel(&self) -> Channel {
        match self {
            Edition::Oss => Channel::Github,
            Edition::Plus => Channel::Blob,
        }
    }

    fn describe(&self) -> &'static str {
        match self {
            Edition::Oss => "OSS",
            Edition::Plus => "Plus",
        }
    }
}

pub fn update(
    edition: Edition,
    current_version: &str,
    check: bool,
    rollback: bool,
    yes: bool,
) -> Result<i32> {
    if check {
        return status_check(edition, current_version);
    }
    if rollback {
        let target = require_self_managed(&detect::detect()?, "rollback")?;
        confirm(
            &format!("Restore the previous binary at {}?", target.display()),
            yes,
        )?;
        swap::restore_backup(&target)?;
        println!("Previous binary restored. Verify with: rtk --version");
        return Ok(0);
    }

    let target = require_self_managed(&detect::detect()?, "update")?;
    let release = edition.channel().latest(&platform::detect()?)?;
    if !version::is_newer(&release.version, current_version) {
        println!(
            "Already up to date ({} v{})",
            edition.describe(),
            version::numeric(current_version)
        );
        return Ok(0);
    }

    confirm(
        &format!(
            "Update {} v{} → v{}?",
            edition.describe(),
            version::numeric(current_version),
            release.version
        ),
        yes,
    )?;
    swap::download_and_swap(&release.tarball_url, release.sha256.as_deref(), &target)?;
    println!(
        "Updated to {} v{} (previous binary kept at {})",
        edition.describe(),
        release.version,
        swap::backup_path(&target).display()
    );
    Ok(0)
}

pub fn upgrade(edition: Edition, current_version: &str, yes: bool) -> Result<i32> {
    if edition == Edition::Plus {
        println!(
            "Already on the Plus edition (v{}) — use 'rtk update' to get the latest version.",
            current_version
        );
        return Ok(0);
    }

    let target = require_self_managed(&detect::detect()?, "upgrade")?;
    let release = Channel::Blob.latest(&platform::detect()?)?;
    confirm(
        &format!(
            "Upgrade to rtk Plus v{}? This replaces the OSS binary at {} (a backup is kept).",
            release.version,
            target.display()
        ),
        yes,
    )?;
    swap::download_and_swap(&release.tarball_url, release.sha256.as_deref(), &target)?;
    create_sibling(&target)?;
    println!(
        "Upgraded to rtk Plus v{} (OSS binary kept at {})",
        release.version,
        swap::backup_path(&target).display()
    );
    println!("Activate your license: {}", LOGIN_HINT);
    Ok(0)
}

pub fn downgrade(edition: Edition, yes: bool) -> Result<i32> {
    if edition == Edition::Oss {
        println!("Already OSS — nothing to do.");
        return Ok(0);
    }

    let target = require_self_managed(&detect::detect()?, "downgrade")?;
    let release = Channel::Github.latest(&platform::detect()?)?;
    confirm(
        &format!(
            "Downgrade to rtk OSS v{}? This replaces the Plus binary at {} (a backup is kept).",
            release.version,
            target.display()
        ),
        yes,
    )?;
    swap::download_and_swap(&release.tarball_url, release.sha256.as_deref(), &target)?;
    remove_sibling(&target);
    println!(
        "Downgraded to rtk OSS v{} (Plus binary kept at {})",
        release.version,
        swap::backup_path(&target).display()
    );
    Ok(0)
}

pub fn uninstall(edition: Edition, yes: bool, purge: bool) -> Result<i32> {
    let target = require_self_managed(&detect::detect()?, "uninstall")?;
    confirm(
        &format!(
            "Remove rtk {} from {}?{}",
            edition.describe(),
            target.display(),
            if purge {
                " Config and data directories will also be removed."
            } else {
                ""
            }
        ),
        yes,
    )?;

    std::fs::remove_file(&target)
        .with_context(|| format!("Failed to remove '{}'", target.display()))?;
    remove_sibling(&target);
    let _ = std::fs::remove_file(swap::backup_path(&target));
    println!("Removed {}", target.display());

    if purge {
        purge_user_dirs();
    }
    Ok(0)
}

fn status_check(edition: Edition, current_version: &str) -> Result<i32> {
    let method = detect::detect()?;
    let target = platform::detect()?;
    println!("edition:    {}", edition.describe());
    println!("version:    {}", current_version);
    println!(
        "binary:     {} ({})",
        method.path().display(),
        method.describe()
    );
    println!(
        "backup:     {}",
        if swap::backup_path(method.path()).exists() {
            "present (rtk update --rollback)"
        } else {
            "none"
        }
    );
    for ch in [Channel::Github, Channel::Blob] {
        let latest = match ch.latest(&target) {
            Ok(release) => release.version,
            Err(e) => format!("unavailable ({:#})", e),
        };
        println!("latest {}: {}", ch.describe(), latest);
    }
    if let Ok(release) = edition.channel().latest(&target) {
        if version::is_newer(&release.version, current_version) {
            println!("update available: v{} — run 'rtk update'", release.version);
        }
    }
    Ok(0)
}

fn require_self_managed(method: &InstallMethod, verb: &str) -> Result<PathBuf> {
    match method {
        InstallMethod::SelfManaged { path } => Ok(path.clone()),
        InstallMethod::BrewManaged { path } => bail!(
            "This rtk is managed by Homebrew ({}) — '{}' only operates on self-managed installs.\n\
             Use 'brew upgrade rtk' / 'brew uninstall rtk'. Homebrew-aware flows arrive in a later version.",
            path.display(),
            verb
        ),
        InstallMethod::SystemPackage { path } => bail!(
            "This rtk is managed by a system package manager ({}) — '{}' only operates on self-managed installs.\n\
             Use 'apt upgrade rtk' / 'dnf upgrade rtk' or your package tool.",
            path.display(),
            verb
        ),
    }
}

fn confirm(prompt: &str, yes: bool) -> Result<()> {
    if yes {
        return Ok(());
    }
    if !std::io::stdin().is_terminal() {
        bail!("Confirmation required — re-run with --yes in non-interactive contexts");
    }
    eprint!("{} [y/N] ", prompt);
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .context("Failed to read confirmation")?;
    if matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
        Ok(())
    } else {
        bail!("Aborted");
    }
}

/// Sibling symlink next to the binary: a backward-compatible alias and an
/// on-disk edition marker for later phases.
fn create_sibling(target: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let sibling = swap::sibling_path(target);
        let _ = std::fs::remove_file(&sibling);
        std::os::unix::fs::symlink("rtk", &sibling)
            .with_context(|| format!("Failed to create '{}'", sibling.display()))?;
    }
    Ok(())
}

fn remove_sibling(target: &Path) {
    let sibling = swap::sibling_path(target);
    if sibling.symlink_metadata().is_ok() {
        let _ = std::fs::remove_file(&sibling);
    }
}

fn purge_user_dirs() {
    let dirs = [
        dirs::config_dir().map(|d| d.join("rtk")),
        dirs::data_local_dir().map(|d| d.join("rtk")),
    ];
    for dir in dirs.into_iter().flatten() {
        if dir.exists() {
            match std::fs::remove_dir_all(&dir) {
                Ok(()) => println!("Removed {}", dir.display()),
                Err(e) => eprintln!("rtk: warning: could not remove {}: {}", dir.display(), e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_require_self_managed() {
        let ok = require_self_managed(
            &InstallMethod::SelfManaged {
                path: PathBuf::from("/home/dev/.local/bin/rtk"),
            },
            "update",
        );
        assert!(ok.is_ok());

        let brew = require_self_managed(
            &InstallMethod::BrewManaged {
                path: PathBuf::from("/opt/homebrew/Cellar/rtk/0.43.0/bin/rtk"),
            },
            "update",
        );
        assert!(brew.unwrap_err().to_string().contains("Homebrew"));

        let system = require_self_managed(
            &InstallMethod::SystemPackage {
                path: PathBuf::from("/usr/bin/rtk"),
            },
            "update",
        );
        assert!(system.unwrap_err().to_string().contains("package manager"));
    }

    #[test]
    fn test_confirm_yes_flag_skips_prompt() {
        assert!(confirm("proceed?", true).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn test_sibling_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("rtk");
        std::fs::write(&target, "bin").unwrap();

        create_sibling(&target).unwrap();
        let sibling = swap::sibling_path(&target);
        assert!(sibling.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read_to_string(&sibling).unwrap(), "bin");

        remove_sibling(&target);
        assert!(sibling.symlink_metadata().is_err());
        assert!(target.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_create_sibling_replaces_legacy_real_file() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("rtk");
        std::fs::write(&target, "bin").unwrap();
        std::fs::write(swap::sibling_path(&target), "legacy-sibling-binary").unwrap();

        create_sibling(&target).unwrap();
        let sibling = swap::sibling_path(&target);
        assert!(sibling.symlink_metadata().unwrap().file_type().is_symlink());
    }
}
