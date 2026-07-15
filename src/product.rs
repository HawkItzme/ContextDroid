use std::path::PathBuf;
use std::{fs, io::Write, path::Path};

use anyhow::{Context, Result};

pub const BINARY_NAME: &str = "contextdroid";
pub const DISPLAY_NAME: &str = "ContextDroid";
pub const DATA_DIR: &str = "contextdroid";
pub const ANALYTICS_DB: &str = "analytics.db";
pub const ENV_PREFIX: &str = "CONTEXTDROID";
pub const DEFAULT_PROFILE: &str = "contextdroid-safe";

pub fn data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|base| base.join(DATA_DIR))
}

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|base| base.join(DATA_DIR))
}

pub fn cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|base| base.join(DATA_DIR))
}

/// Replace a product-owned text/config file without following a destination symlink.
/// Existing content is restored if the final rename fails.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("atomic write path has no parent")?;
    fs::create_dir_all(parent)?;

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                anyhow::bail!("refusing to replace symlink: {}", path.display());
            }
            Some(metadata)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };

    let nonce = format!(
        "{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let temp = path.with_extension(format!("contextdroid.{nonce}.tmp"));
    let backup = path.with_extension(format!("contextdroid.{nonce}.bak"));

    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temp)
        .with_context(|| format!("failed to create temporary file for {}", path.display()))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    if let Some(metadata) = &metadata {
        fs::set_permissions(&temp, metadata.permissions())?;
    }

    if metadata.is_some() {
        fs::rename(path, &backup)?;
        if let Err(error) = fs::rename(&temp, path) {
            let _ = fs::rename(&backup, path);
            let _ = fs::remove_file(&temp);
            return Err(error).with_context(|| format!("failed to replace {}", path.display()));
        }
        fs::remove_file(&backup)?;
    } else {
        fs::rename(&temp, path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_identity_is_contextdroid() {
        assert_eq!(BINARY_NAME, "contextdroid");
        assert_eq!(DISPLAY_NAME, "ContextDroid");
        assert_eq!(ENV_PREFIX, "CONTEXTDROID");
        assert_eq!(DEFAULT_PROFILE, "contextdroid-safe");
    }

    #[test]
    fn test_product_paths_do_not_use_legacy_rtk_directory() {
        for path in [data_dir(), config_dir(), cache_dir()]
            .into_iter()
            .flatten()
        {
            assert_eq!(
                path.file_name().and_then(|name| name.to_str()),
                Some(DATA_DIR)
            );
            assert_ne!(path.file_name().and_then(|name| name.to_str()), Some("rtk"));
        }
    }

    #[test]
    fn atomic_write_replaces_existing_content() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        fs::write(&path, "old").unwrap();

        write_atomic(&path, b"new").unwrap();

        assert_eq!(fs::read_to_string(path).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_refuses_destination_symlink() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("settings.json");
        fs::write(&target, "untouched").unwrap();
        symlink(&target, &link).unwrap();

        assert!(write_atomic(&link, b"replacement").is_err());
        assert_eq!(fs::read_to_string(target).unwrap(), "untouched");
    }
}
