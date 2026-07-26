# Testing ContextDroid

Run development commands directly so failures, warnings, diffs, and security output remain
unfiltered. Do not route this repository's own verification through ContextDroid or another output
compression hook.

## Required local gates

```bash
cargo fmt --all --check
cargo test --all --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo build --release --locked
bash scripts/validate-docs.sh
```

Installer changes also require:

```bash
bash scripts/test-install.sh
pwsh -File scripts/test-install.ps1
```

Parser and rewrite changes require representative redistributable fixtures, semantic preservation
assertions, malformed and unknown-output cases, exit-code parity, raw-recovery coverage, and both
positive rewrite and negative no-rewrite tests. Golden output may supplement these assertions but
must not replace them.

Release candidates must additionally pass the repository's cross-platform build, package dry run,
installer smoke, Android smoke, public Android validation, audit, dependency-policy, and static
security workflows on the exact reviewed commit.
