# Alpha Release Checklist

- [ ] Provenance, Apache-2.0 license, upstream SHA, and notices reviewed.
- [ ] All product-facing RTK names and URLs migrated or explicitly labeled legacy.
- [ ] Synthetic fixture matrix complete and provenance documented.
- [ ] Exit parity, raw recovery, parser preservation, and profile hard stops green.
- [ ] Claude/Cursor/Codex install, status, preview, and uninstall green on supported hosts.
- [ ] Clean install/uninstall verified for every supported package.
- [ ] Windows, Linux, and macOS CI green.
- [ ] `cargo fmt --all --check` green with raw output.
- [ ] `cargo test --all` green with raw output.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` green with raw output.
- [ ] `cargo build --release` green with raw output.
- [ ] README limitations and benchmark measurements match evidence.
- [ ] No remote telemetry or ContextDroid network analytics dependency exists.
- [ ] No release, tag, asset upload, or push occurs without explicit approval.
