# v0.1.0 stable release checklist

This is the promotion checklist for the first stable ContextDroid release. A local pass never
substitutes for protected checks on the exact reviewed commit.

## Product and repository gates

- [x] Android diagnostic correctness, raw recovery, exit parity, conservative fallback, and
  omission accounting remain covered by fixtures and contract tests.
- [x] Public Android validation and the permissioned internal pilot are complete.
- [x] Internal agent workspaces, private pilot notes, and maintainer-only artifacts are absent from
  the public tree and guarded by deterministic tests.
- [x] Apache-2.0 provenance, upstream pinning, third-party notices, and independent-maintenance
  language remain intact.
- [x] Generic `contextdroid setup` detection, preview, status, transactional apply, and bounded
  uninstall are tested; Cursor remains explicit and experimental.

## Distribution gates

- [x] Unix and Windows installers discover the latest stable release when no version is supplied.
- [x] Explicit stable and prerelease pins remain supported.
- [x] Custom release bases require an explicit version.
- [x] HTTPS/redirect, checksum, archive-entry, binary-version, transactional replacement, and PATH
  rollback contracts are tested.
- [x] Five canonical archives, DEB, RPM, SBOM, manifest, checksums, notices, and provenance remain
  release outputs.
- [ ] Package dry run and all native installer smoke jobs pass on the exact release commit.

## Required verification

- [ ] `cargo fmt --all --check`
- [ ] `cargo test --all --locked`
- [ ] `cargo clippy --all-targets --all-features --locked -- -D warnings`
- [ ] `cargo build --release --locked`
- [ ] `bash scripts/validate-docs.sh`
- [ ] Unix and Windows mocked installer suites
- [ ] Cross-platform CI, security, Android smoke, public Android validation, and package checks

## Publication gate

- [ ] Maintainer separately authorizes creation of annotated tag `v0.1.0`.
- [ ] The immutable tag resolves to the exact protected `main` commit.
- [ ] Release packaging runs with `publish=true` and `prerelease=false`.
- [ ] The private draft contains the exact verified asset set before it becomes public.
- [ ] `/releases/latest` resolves to `v0.1.0`.
- [ ] Public installer smoke succeeds from the stable release on all five supported targets.

Tag creation, release publication, branch deletion, and repository-setting changes are not part of
ordinary implementation and require separate authorization.
