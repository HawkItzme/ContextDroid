# v0.1.0-alpha.1 release checklist

Unchecked items are real blockers. A local pass never substitutes for green checks on the exact
release commit.

## Implemented release gates

- [x] Apache-2.0 provenance, pinned RTK v0.43.0 commit, notices, and independent-maintenance text.
- [x] Typed fixture preservation, cause/location/frame/dependency/test assertions, raw recovery,
  durable IDs, omission counts, never-worse decisions, and exit/signal contract tests.
- [x] Runtime fixture preservation and exit parity are unknown unless measured; observed parser
  errors force raw fallback.
- [x] ContextDroid contribution, security, installation, disclaimer, support, roadmap, pilot, and
  draft release documentation replaces inherited product claims.
- [x] Inherited release-please and next-release automation disabled for the manual first alpha.
- [x] Canonical five-target archive manifest, `SHA256SUMS`, local installer test, DEB/RPM tooling,
  and non-publishing package dry run implemented.
- [x] Cargo audit, cargo-deny source/license policy, Semgrep, dangerous-change scan, and CODEOWNERS
  are checked in as blocking policy.
- [x] Stable branch references use `main`; ordinary contributions target `develop`.
- [x] Public redistributable-project validation and one-time private validation are complete; no
  private project identity, source, or output is retained in this repository.
- [x] Reproducible fixture benchmark and honest no-reduction fallback cases are published.

## Exact merge gate: readiness branch → develop

- [ ] Review this change as bounded commits or an equivalent reviewable series.
- [ ] `cargo fmt --all --check` green.
- [ ] `cargo test --all --locked` green.
- [ ] `cargo clippy --all-targets --all-features --locked -- -D warnings` green.
- [ ] `cargo build --release --locked` green.
- [ ] Documentation/stale-brand, release-contract, and security-contract tests green.
- [ ] Linux, Windows, and macOS test and release-build jobs green.
- [ ] Android API 36 smoke, Claude Linux smoke, and public Android project validation green.
- [ ] Cargo audit, cargo deny, and Semgrep green with no expired or unexplained exception.
- [ ] Maintainer review confirms no destructive operation, global configuration change, tag,
  asset upload, or publication occurred.

## Exact promotion gate: develop → main

- [ ] The merge commit on `develop` is identified and all required checks are green on that exact
  commit.
- [ ] Real or redistributable evidence covers build success, compiler/resource/Manifest/test
  failure, crash/ANR, unknown/malformed, and verbose/lossless behavior.
- [ ] Direct and effective benchmark results include every recovery/rerun and release-build
  general-command timing.
- [ ] Claude Linux support, Cursor experimental status, and Codex guidance-only status match the
  evidence.
- [ ] Package dry run validates every canonical archive and native Unix/Windows installer.
- [ ] Repository About/default branch/branch protection settings are reviewed by a maintainer.
- [ ] The exact reviewed commit is promoted without unrelated changes.

## Exact release gate: main commit → v0.1.0-alpha.1

- [ ] `Cargo.toml`, `.release-please-manifest.json`, changelog, tag, and release title all say
  `0.1.0-alpha.1` / `v0.1.0-alpha.1`.
- [ ] All required checks are green on the exact `main` commit.
- [ ] The existing annotated tag points to that exact commit and contains the prerelease suffix.
- [ ] Draft release notes and final artifact/checksum manifest are reviewed.
- [ ] Rollback owner and advisory/support path are confirmed.
- [x] The maintainer authorized the exact gated `main` commit, annotated
  `v0.1.0-alpha.1` tag, and public prerelease publication.

Only after every item above is satisfied may the release workflow be dispatched with
`publish=true`. Use the built-in `GITHUB_TOKEN`; no separate app credentials are required.

## Current blockers as of 2026-07-17

- Cross-platform and new packaging/security workflows have not run on this working tree.
- Clean JDK 17 public-project CI is pending on the exact release commit.
- GitHub default-branch, protection, About/topics/social preview, and inherited-tag presentation
  require explicit repository-admin action.
- The authorized tag and publication remain gated on exact-commit CI and packaging.

Do not create or push a tag, publish a release, upload assets, or change repository settings while
these blockers remain.
