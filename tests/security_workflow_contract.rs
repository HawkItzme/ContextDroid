use std::fs;
use std::path::Path;

#[test]
fn security_policy_is_blocking_and_uses_the_actual_pr_base() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ci = fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    let owners = fs::read_to_string(root.join(".github/CODEOWNERS")).unwrap();
    let deny = fs::read_to_string(root.join("deny.toml")).unwrap();

    assert!(ci.contains("cargo audit --deny warnings"));
    assert!(ci.contains("cargo deny check advisories bans licenses sources"));
    assert!(ci.contains("github.event.pull_request.base.sha"));
    assert!(!ci.contains("origin/master"));
    assert!(ci.contains("semgrep scan --config .semgrep.yml"));
    assert!(ci.contains("--error"));
    assert!(ci.contains("Dangerous code patterns require security review"));
    assert!(owners.contains("src/core/runner.rs"));
    assert!(deny.contains("unknown-git = \"deny\""));
}

#[test]
fn exact_commit_ci_has_safe_comparison_bases_and_pinned_actions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ci = fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();

    assert!(ci.contains("workflow_dispatch:"));
    assert!(ci.contains("push:"));
    assert!(ci.contains("COMPARISON_BASE"));
    assert!(ci.contains("github.event.before"));
    assert!(!ci.contains("uses: actions/checkout@v4"));
    assert!(!ci.contains("uses: dtolnay/rust-toolchain@1.91.0"));
    assert!(!ci.contains("uses: Swatinem/rust-cache@v2"));
    assert!(!ci.contains("runs-on: macos-latest"));
}
