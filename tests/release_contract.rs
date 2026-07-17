use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Manifest {
    schema_version: u32,
    targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
struct Target {
    rust_target: String,
    asset: String,
    archive: String,
    installer: bool,
}

#[test]
fn release_targets_match_workflow_installer_and_documentation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: Manifest = serde_json::from_str(
        &fs::read_to_string(root.join("release/targets.json")).expect("release target manifest"),
    )
    .unwrap();
    let workflow = fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    let installer = fs::read_to_string(root.join("install.sh")).unwrap();
    let windows_installer = fs::read_to_string(root.join("install.ps1")).unwrap();
    let docs = fs::read_to_string(root.join("docs/RELEASE_ARTIFACTS.md")).unwrap();

    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.targets.len(), 5);
    for target in manifest.targets {
        assert!(
            workflow.contains(&target.rust_target),
            "workflow lacks {}",
            target.rust_target
        );
        assert!(
            workflow.contains(&target.asset),
            "workflow lacks {}",
            target.asset
        );
        assert!(docs.contains(&target.asset), "docs lack {}", target.asset);
        assert!(matches!(target.archive.as_str(), "tar.gz" | "zip"));
        if target.installer {
            let platform_installer = if target.archive == "zip" {
                &windows_installer
            } else {
                &installer
            };
            assert!(
                platform_installer.contains(&target.asset),
                "installer lacks {}",
                target.asset
            );
        }
    }
    assert!(workflow.contains("SHA256SUMS"));
    assert!(installer.contains("SHA256SUMS"));
    assert!(workflow.contains("cargo-cyclonedx --version 0.5.9"));
    assert!(workflow.contains("contextdroid.cdx.json"));
    assert!(workflow.contains("THIRD_PARTY_NOTICES.md"));
    assert!(workflow.contains("attest-build-provenance"));
}

#[test]
fn inherited_release_please_is_disabled_for_first_alpha() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join(".release-please-manifest.json")).unwrap();
    let cd = fs::read_to_string(root.join(".github/workflows/cd.yml")).unwrap();

    assert!(manifest.contains("0.1.0-alpha.1"));
    assert!(!cd.contains("release-please-action"));
    assert!(cd.contains("first alpha is manual"));
}

#[test]
fn quick_install_contract_is_cross_platform_and_prerelease_safe() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: Manifest = serde_json::from_str(
        &fs::read_to_string(root.join("release/targets.json")).expect("release target manifest"),
    )
    .unwrap();
    let unix = fs::read_to_string(root.join("install.sh")).expect("Unix installer");
    let windows = fs::read_to_string(root.join("install.ps1")).expect("Windows installer");
    let workflow = fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();

    let windows_target = manifest
        .targets
        .iter()
        .find(|target| target.rust_target == "x86_64-pc-windows-msvc")
        .expect("Windows release target");
    assert!(
        windows_target.installer,
        "Windows must support quick install"
    );
    assert!(windows.contains(&windows_target.asset));

    for installer in [&unix, &windows] {
        assert!(installer.contains("v0.1.0-alpha.1"));
        assert!(installer.contains("CONTEXTDROID_VERSION"));
        assert!(installer.contains("CONTEXTDROID_INSTALL_DIR"));
        assert!(installer.contains("CONTEXTDROID_RELEASE_BASE"));
        assert!(installer.contains("SHA256SUMS"));
        assert!(installer.contains("checksum mismatch"));
    }

    assert!(!unix.contains("releases/latest"));
    assert!(!workflow.contains("macos-13"));
    assert!(!workflow.contains("sha256sum * > SHA256SUMS"));
    assert!(workflow.contains("release-manifest.json"));
    assert!(workflow.contains("install.ps1"));
}

#[test]
fn package_dry_run_gates_every_pull_request_commit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = fs::read_to_string(root.join(".github/workflows/package-dry-run.yml")).unwrap();

    assert!(workflow.contains("pull_request:"));
    assert!(
        !workflow.contains("paths:"),
        "release packaging must not skip commits based on changed paths"
    );
}
