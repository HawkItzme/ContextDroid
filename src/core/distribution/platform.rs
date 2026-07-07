//! Maps the running build to its release triple
//! (`x86_64-unknown-linux-musl`) used in release asset names.

use anyhow::{bail, Result};

pub struct Target {
    pub triple: &'static str,
}

pub fn detect() -> Result<Target> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Ok(Target {
        triple: "x86_64-unknown-linux-musl",
    });

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Ok(Target {
        triple: "aarch64-unknown-linux-gnu",
    });

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Ok(Target {
        triple: "x86_64-apple-darwin",
    });

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Ok(Target {
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
        assert!(target.triple.contains('-'));
    }
}
