//! Self-managed install lifecycle: `rtk update` / `rtk uninstall`.
//!
//! One engine drives both flows: resolve the latest GitHub release, verify
//! its checksum, atomically swap the binary at its own path. The `rtk` name
//! always has exactly one owner — mutating verbs refuse on brew-managed or
//! system-package installs instead of creating a second copy on PATH
//! (Homebrew-aware flows arrive in a later phase).

pub mod channel;
pub mod detect;
pub mod platform;
pub mod swap;
#[cfg(test)]
pub(crate) mod test_http;
pub mod version;

use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use detect::InstallMethod;

pub fn update(current_version: &str, check: bool, rollback: bool, yes: bool) -> Result<i32> {
    if check {
        return status_check(current_version);
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
    let release = channel::latest(&platform::detect()?)?;
    if !version::is_newer(&release.version, current_version) {
        println!(
            "Already up to date (v{})",
            version::numeric(current_version)
        );
        return Ok(0);
    }

    confirm(
        &format!(
            "Update rtk v{} → v{}?",
            version::numeric(current_version),
            release.version
        ),
        yes,
    )?;
    swap::download_and_swap(&release.tarball_url, release.sha256.as_deref(), &target)?;
    println!(
        "Updated to rtk v{} (previous binary kept at {})",
        release.version,
        swap::backup_path(&target).display()
    );
    Ok(0)
}

pub fn uninstall(yes: bool, purge: bool) -> Result<i32> {
    let target = require_self_managed(&detect::detect()?, "uninstall")?;
    confirm(
        &format!(
            "Remove rtk from {}?{}",
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
    let _ = std::fs::remove_file(swap::backup_path(&target));
    println!("Removed {}", target.display());

    if purge {
        purge_user_dirs();
    }
    Ok(0)
}

fn status_check(current_version: &str) -> Result<i32> {
    let method = detect::detect()?;
    let target = platform::detect()?;
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
    match channel::latest(&target) {
        Ok(release) => {
            println!("latest:     {}", release.version);
            if version::is_newer(&release.version, current_version) {
                println!("update available: v{} — run 'rtk update'", release.version);
            }
        }
        Err(e) => println!("latest:     unavailable ({:#})", e),
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
}
