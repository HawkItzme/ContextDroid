//! Atomic binary replacement engine for self-managed installs.
//!
//! Verify-before-touch: the tarball is downloaded to a temp file in the
//! target's own directory (same filesystem → atomic `rename`), its sha256
//! checked against the channel's expected value, and only then is the live
//! binary stashed to `<target>.bak` and replaced. Any failure leaves the
//! live binary untouched.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::core::utils::resolved_command;

const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
const BINARY_NAMES: &[&str] = &["rtk", "rtk-plus"];

pub fn backup_path(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "rtk".to_string());
    target.with_file_name(format!("{}.bak", name))
}

pub fn sibling_path(target: &Path) -> PathBuf {
    target.with_file_name("rtk-plus")
}

/// Download, verify and atomically install a release tarball over `target`.
pub fn download_and_swap(url: &str, expected_sha256: Option<&str>, target: &Path) -> Result<()> {
    let dir = target
        .parent()
        .ok_or_else(|| anyhow!("Target path '{}' has no parent directory", target.display()))?;
    fs::create_dir_all(dir)
        .with_context(|| format!("Failed to create install directory '{}'", dir.display()))?;

    let staging = Staging::new(dir)?;
    download(url, &staging.tarball)?;

    match expected_sha256 {
        Some(expected) => {
            let actual = sha256_file(&staging.tarball)?;
            if actual != expected.to_lowercase() {
                bail!(
                    "Checksum mismatch for {} (expected {}, got {}) — a proxy may have served a stale file; retry",
                    url,
                    expected,
                    actual
                );
            }
        }
        None => eprintln!("rtk: warning: no checksum available — installing unverified download"),
    }

    let binary = extract_binary(&staging)?;
    swap_file(&binary, target)
}

/// Restore the previous binary stashed by the last swap.
pub fn restore_backup(target: &Path) -> Result<()> {
    let bak = backup_path(target);
    if !bak.exists() {
        bail!("No backup found at '{}'", bak.display());
    }
    fs::rename(&bak, target)
        .with_context(|| format!("Failed to restore '{}' from backup", target.display()))
}

fn swap_file(new_binary: &Path, target: &Path) -> Result<()> {
    set_executable(new_binary)?;
    if target.exists() {
        fs::copy(target, backup_path(target))
            .with_context(|| format!("Failed to back up '{}'", target.display()))?;
    }
    fs::rename(new_binary, target)
        .with_context(|| format!("Failed to install to '{}'", target.display()))
}

fn download(url: &str, dest: &Path) -> Result<()> {
    let response = ureq::get(url)
        .set("Cache-Control", "no-cache")
        .set("User-Agent", "rtk-distribution")
        .timeout(DOWNLOAD_TIMEOUT)
        .call()
        .with_context(|| format!("Download failed: {}", url))?;
    let mut file = fs::File::create(dest)
        .with_context(|| format!("Failed to create temp file '{}'", dest.display()))?;
    std::io::copy(&mut response.into_reader(), &mut file)
        .with_context(|| format!("Failed to write download to '{}'", dest.display()))?;
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file =
        fs::File::open(path).with_context(|| format!("Failed to open '{}'", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).context("Read failed during hashing")?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect())
}

fn extract_binary(staging: &Staging) -> Result<PathBuf> {
    let status = resolved_command("tar")
        .arg("-xzf")
        .arg(&staging.tarball)
        .arg("-C")
        .arg(&staging.extract_dir)
        .status()
        .context("Failed to run tar")?;
    if !status.success() {
        bail!("tar extraction failed (corrupt archive?)");
    }
    find_binary(&staging.extract_dir)
        .ok_or_else(|| anyhow!("No rtk binary found in downloaded archive"))
}

fn find_binary(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let name = entry.file_name();
            if BINARY_NAMES.iter().any(|b| name == *b) {
                return Some(path);
            }
        } else if path.is_dir() {
            subdirs.push(path);
        }
    }
    subdirs.iter().find_map(|sub| find_binary(sub))
}

fn set_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .with_context(|| format!("Failed to chmod '{}'", path.display()))?;
    }
    Ok(())
}

/// Temp paths inside the target directory, removed on drop so failures
/// never leave half-written files next to the live binary.
struct Staging {
    tarball: PathBuf,
    extract_dir: PathBuf,
}

impl Staging {
    fn new(dir: &Path) -> Result<Self> {
        let pid = std::process::id();
        let extract_dir = dir.join(format!(".rtk-dist-{}", pid));
        fs::create_dir_all(&extract_dir)
            .with_context(|| format!("Failed to create staging dir in '{}'", dir.display()))?;
        Ok(Self {
            tarball: dir.join(format!(".rtk-dist-{}.tar.gz", pid)),
            extract_dir,
        })
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.tarball);
        let _ = fs::remove_dir_all(&self.extract_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn make_tarball(dir: &Path, binary_name: &str, content: &str) -> PathBuf {
        let src = dir.join("pack");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join(binary_name), content).unwrap();
        let tar = dir.join("release.tar.gz");
        let status = Command::new("tar")
            .arg("-czf")
            .arg(&tar)
            .arg("-C")
            .arg(&src)
            .arg(binary_name)
            .status()
            .unwrap();
        assert!(status.success());
        tar
    }

    fn serve_tarball(tar: &Path) -> (String, String) {
        let bytes = fs::read(tar).unwrap();
        let sha = sha256_file(tar).unwrap();
        let base =
            super::super::test_http::serve(vec![("/release.tar.gz".to_string(), 200, bytes)]);
        (format!("{}/release.tar.gz", base), sha)
    }

    #[test]
    fn test_download_and_swap_success_with_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("bin").join("rtk");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "old-binary").unwrap();

        let tar = make_tarball(tmp.path(), "rtk", "new-binary");
        let (url, sha) = serve_tarball(&tar);

        download_and_swap(&url, Some(&sha), &target).unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "new-binary");
        assert_eq!(
            fs::read_to_string(backup_path(&target)).unwrap(),
            "old-binary"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&target).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }
    }

    #[test]
    fn test_checksum_mismatch_leaves_target_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("rtk");
        fs::write(&target, "old-binary").unwrap();

        let tar = make_tarball(tmp.path(), "rtk", "evil-binary");
        let (url, _) = serve_tarball(&tar);

        let err = download_and_swap(&url, Some(&"0".repeat(64)), &target).unwrap_err();
        assert!(err.to_string().contains("Checksum mismatch"));
        assert_eq!(fs::read_to_string(&target).unwrap(), "old-binary");
        assert!(!backup_path(&target).exists());
        // Staging temp files must be cleaned up.
        let leftovers: Vec<_> = fs::read_dir(tmp.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(".rtk-dist"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn test_swap_accepts_suffixed_binary_name() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("rtk");
        let tar = make_tarball(tmp.path(), "rtk-plus", "plus-binary");
        let (url, sha) = serve_tarball(&tar);

        download_and_swap(&url, Some(&sha), &target).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "plus-binary");
    }

    #[test]
    fn test_corrupt_archive_fails_cleanly() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("rtk");
        fs::write(&target, "old-binary").unwrap();

        let base = super::super::test_http::serve(vec![(
            "/release.tar.gz".to_string(),
            200,
            b"not a tarball".to_vec(),
        )]);
        let url = format!("{}/release.tar.gz", base);

        assert!(download_and_swap(&url, None, &target).is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "old-binary");
    }

    #[test]
    fn test_restore_backup_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("rtk");
        fs::write(&target, "current").unwrap();
        fs::write(backup_path(&target), "previous").unwrap();

        restore_backup(&target).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "previous");
        assert!(!backup_path(&target).exists());
    }

    #[test]
    fn test_restore_backup_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("rtk");
        assert!(restore_backup(&target).is_err());
    }
}
