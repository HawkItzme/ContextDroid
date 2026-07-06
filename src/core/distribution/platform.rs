//! Maps the running build to the two release naming schemes:
//! blob targets (`linux-amd64`) and release triples
//! (`x86_64-unknown-linux-musl`).

use anyhow::{bail, Result};

pub struct Target {
    pub blob: &'static str,
    pub triple: &'static str,
}

pub fn detect() -> Result<Target> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Ok(Target {
        blob: "linux-amd64",
        triple: "x86_64-unknown-linux-musl",
    });

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Ok(Target {
        blob: "linux-arm64",
        triple: "aarch64-unknown-linux-gnu",
    });

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Ok(Target {
        blob: "darwin-amd64",
        triple: "x86_64-apple-darwin",
    });

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Ok(Target {
        blob: "darwin-arm64",
        triple: "aarch64-apple-darwin",
    });

    #[allow(unreachable_code)]
    {
        bail!(
            "unsupported platform for self-managed install flows ({}-{})",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_current_platform() {
        let target = detect().expect("supported platform");
        assert!(target.blob.contains('-'));
        assert!(target.triple.contains('-'));
    }
}
