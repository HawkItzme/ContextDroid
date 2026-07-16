use anyhow::{bail, Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn ensure_private_dir(path: &Path) -> Result<()> {
    reject_reparse_components(path)?;
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create private directory: {}", path.display()))?;
    reject_reparse_components(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

pub fn create_private_new(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent)?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    options
        .open(path)
        .with_context(|| format!("failed to securely create {}", path.display()))
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("secure write target has no parent")?;
    ensure_private_dir(parent)?;
    let nonce = format!(
        "{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let temp = parent.join(format!(".contextdroid-{nonce}.tmp"));
    let mut file = create_private_new(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    let backup = parent.join(format!(".contextdroid-{nonce}.backup"));
    let had_destination = path.exists();
    if had_destination {
        fs::rename(path, &backup)
            .with_context(|| format!("failed to stage replacement for {}", path.display()))?;
    }
    if let Err(error) = fs::rename(&temp, path) {
        if had_destination {
            let _ = fs::rename(&backup, path);
        }
        let _ = fs::remove_file(&temp);
        return Err(error)
            .with_context(|| format!("failed to atomically replace {}", path.display()));
    }
    if had_destination {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

pub fn reject_store_inside_repository(path: &Path) -> Result<()> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut cursor: Option<&Path> = Some(&absolute);
    while let Some(current) = cursor {
        if current.join(".git").exists() {
            bail!(
                "raw run storage must not be located inside a source repository: {}",
                path.display()
            );
        }
        cursor = current.parent();
    }
    Ok(())
}

pub fn reject_reparse_components(path: &Path) -> Result<()> {
    let mut built = PathBuf::new();
    for component in path.components() {
        built.push(component);
        let Ok(metadata) = fs::symlink_metadata(&built) else {
            continue;
        };
        if metadata.file_type().is_symlink() || is_windows_reparse(&metadata) {
            bail!(
                "refusing symlink or reparse-point path component: {}",
                built.display()
            );
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_windows_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_windows_reparse(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_create_is_create_new_and_atomic_write_replaces() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("private").join("value");
        create_private_new(&path).unwrap();
        assert!(create_private_new(&path).is_err());
        fs::remove_file(&path).unwrap();
        atomic_write(&path, b"one").unwrap();
        atomic_write(&path, b"two").unwrap();
        assert_eq!(fs::read(path).unwrap(), b"two");
    }

    #[test]
    fn repository_storage_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();
        assert!(reject_store_inside_repository(&temp.path().join("runs")).is_err());
    }
}
