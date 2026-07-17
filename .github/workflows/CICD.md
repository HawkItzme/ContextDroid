# ContextDroid CI and release flow

Pull requests to the product branch, `develop`, or `main` run formatting, cross-platform tests,
Clippy, release builds, documentation/help contracts, Android smoke, Claude Linux smoke,
Semgrep, dependency audit, and source/license policy checks.

Ordinary contributions target `develop`. The exact reviewed `develop` commit is promoted to
`main` only after the merge gate in `docs/RELEASE_CHECKLIST.md` passes. Branch protection must
require the checked-in blocking jobs and CODEOWNER review for high-risk paths.

`.github/workflows/package-dry-run.yml` invokes the reusable release workflow with publishing
disabled. The release workflow validates the Cargo version, packages every canonical target,
checks archive contents, generates `SHA256SUMS`, and tests the Linux installer locally.

For publication, a maintainer must explicitly dispatch the release workflow from `main` with an
existing tag that points to the checked-out commit. Publishing remains prohibited until explicit
approval. The inherited release-please and next-release workflows are disabled for the first
alpha.
