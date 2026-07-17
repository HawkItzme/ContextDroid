# v0.1.0-alpha.1 release checklist

This checklist records the completed evidence for the first public ContextDroid alpha. A local pass
never substitutes for green checks on the exact reviewed commit.

## Product and safety gates

- [x] Apache-2.0 provenance, pinned RTK v0.43.0 commit, notices, and independent-maintenance text.
- [x] Typed Android fixture preservation, raw recovery, omission accounting, never-worse decisions,
  durable run IDs, and exit/signal parity contracts.
- [x] Runtime correctness metrics report unknown unless measured; parser errors force raw fallback.
- [x] ContextDroid-specific contribution, security, installation, support, roadmap, pilot, and release
  documentation replaces inherited product claims.
- [x] Inherited release-please and next-release publication are disabled for the manual first alpha.
- [x] Five canonical archives, DEB, RPM, SBOM, notices, release manifest, and `SHA256SUMS` are present.
- [x] Cargo audit, cargo deny, Semgrep, dangerous-change policy, and CODEOWNERS checks are blocking.
- [x] Public redistributable-project validation and one permissioned private validation passed; no
  private project identity, source, logs, or artifacts are retained in this repository.
- [x] Reproducible fixture benchmarks and honest no-reduction fallback cases are published.

## Reviewed merge and promotion gates

- [x] The readiness work was reviewed and merged as bounded commits in PR #4.
- [x] Raw fmt, all tests, warnings-denied Clippy, and release build passed locally and remotely.
- [x] Documentation, product identity, release, and security contracts passed.
- [x] Linux, Windows, and macOS tests and release builds passed.
- [x] Android API 36 smoke, Claude Linux smoke, and the pinned public Android project passed.
- [x] Cargo audit, cargo deny, and Semgrep passed without an ignored advisory.
- [x] Package dry run installed/uninstalled DEB and RPM, validated five archives, and executed all five
  platform installers.
- [x] The ContextDroid product commit was promoted through `develop` to protected `main`.
- [x] Default branch, archived upstream refs, About/topics, security settings, and branch/tag rulesets
  were reviewed and applied by the maintainer.

## Public release gate

- [x] Cargo, changelog, release title, and annotated tag agree on `v0.1.0-alpha.1`.
- [x] The immutable tag resolves to reviewed commit `f6a00bd2048ba66aa7f55b34c523825134cc5627`.
- [x] Exact tag/main CI, security, package, Android, and integration gates passed.
- [x] All 16 release assets match the reviewed local names, byte lengths, and SHA-256 digests.
- [x] Package and binary subjects have GitHub build-provenance attestation.
- [x] Rollback, private vulnerability reporting, support, and advisory paths are documented.
- [x] The maintainer explicitly authorized the tag, repository administration, and public prerelease.
- [x] The public prerelease is available at
  <https://github.com/HawkItzme/ContextDroid/releases/tag/v0.1.0-alpha.1>.
- [x] Public installer smoke run `29553266017` passed on Linux x86_64, Linux ARM64, macOS Intel,
  macOS ARM64, and Windows x86_64.

## Post-publication recovery evidence

GitHub sanitized `~` in the first uploaded DEB/RPM filenames, so exact asset parity stopped automated
publication while the release was still private. The verified package bytes were renamed with
GitHub-safe semver filenames, the manifest/checksums were regenerated and reverified, and the
permanent workflow fix passed PR #5 and the `develop` to `main` promotion PR #6. The immutable tag and
package bytes never changed.

The first public smoke exposed a verification-only Unix environment-variable typo. The release was
immediately returned to draft, the workflow was corrected, protected CI/package gates passed again,
and all five public installers then passed. There are no remaining alpha publication blockers.

Optional post-alpha work is tracked separately and does not block direct installation: Homebrew,
social-preview artwork, broader OEM/device-lab coverage, and pre-stable Windows/cancellation
hardening.
