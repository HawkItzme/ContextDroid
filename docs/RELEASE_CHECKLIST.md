# Alpha Release Checklist

## Completed local gates

- [ ] Provenance, Apache-2.0 license, upstream SHA, and notices reviewed.
- [ ] All product-facing RTK names and URLs migrated or explicitly labeled legacy.
- [x] Synthetic fixture matrix complete and provenance documented.
- [x] Exit parity, raw recovery, parser preservation, and profile hard stops green.
- [x] Stabilization `cargo fmt --all --check` green with raw output.
- [x] Stabilization `cargo test --all` green with raw output (2,281 passed, 8 ignored).
- [x] Stabilization `cargo clippy --all-targets --all-features -- -D warnings` green with raw output.
- [x] Stabilization `cargo build --release` green with raw output.
- [x] README limitations and benchmark measurements match current local evidence.
- [x] No remote telemetry or ContextDroid network analytics dependency exists.

## Validation blockers

- [ ] Claude/Cursor/Codex install, status, preview, and uninstall green on supported hosts.
- [ ] Clean install/uninstall verified for every supported package.
- [ ] Windows, Linux, and macOS CI green.
- [ ] Representative live Android Gradle/AGP builds and failure families validated.
- [ ] Emulator/device ADB and Logcat scenarios validate evidence, exit parity, and raw recovery.

CI now requires Ubuntu, Windows, and macOS tests/release builds, deterministic docs/help checks,
a pinned AGP 9.3/Gradle 9.5/JDK 17/API 37 Android sample, and Linux temporary-HOME Claude plus
compact `git status` smoke. Those new jobs have not yet run against this branch; local evidence
remains Windows-only.

## Optional distribution work

- [ ] Versioned GitHub archives and `checksums.txt` tested with `install.sh`.
- [ ] If Homebrew is included in this alpha: downstream tap selected, real SHA-256 values inserted,
  and Intel/Apple Silicon install/uninstall tested.

Homebrew may be deferred without blocking a direct-archive alpha release.

## Publication gate

- [ ] Explicit approval received for the exact version/tag and prerelease status.
- [ ] Release notes and final artifact manifest reviewed.

Do not create or push a tag, publish a GitHub release, upload assets, or update a package channel
before this gate is satisfied. Publication approval is an authorization gate, not a software defect.
