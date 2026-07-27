# v0.1.1 stable release evidence

This records the completed promotion and publication gates for the first stable ContextDroid
release. The immutable `v0.1.1` tag identifies release commit
`4f4b4e6dbf4a8345cd57b749ac69fc84a2f69259`; later documentation and workflow improvements do not
change that tag or its verified assets. A local pass never substitutes for protected checks on the
exact reviewed commit.

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
- [x] Package dry run and all native installer smoke jobs passed on the exact release commit.
- [x] The public Windows installer passed under PowerShell 7 and Windows PowerShell 5.1; the 5.1
  job uses the documented raw-URL `irm ... | iex` command.

## Required verification

- [x] `cargo fmt --all --check`
- [x] `cargo test --all --locked`
- [x] `cargo clippy --all-targets --all-features --locked -- -D warnings`
- [x] `cargo build --release --locked`
- [x] `bash scripts/validate-docs.sh`
- [x] Unix and Windows mocked installer suites
- [x] Cross-platform CI, security, Android smoke, public Android validation, and package checks

## Publication gate

- [x] The maintainer separately authorized creation of annotated tag `v0.1.1`.
- [x] The immutable tag resolves to the exact protected release commit.
- [x] Release packaging ran with `publish=true` and `prerelease=false`.
- [x] The private draft contained the exact verified asset set before it became public.
- [x] `/releases/latest` resolves to `v0.1.1`.
- [x] Public installer smoke succeeded from the stable release on Linux x86_64, Linux ARM64,
  macOS Intel, macOS ARM64, and Windows x86_64.

## Immutable evidence

| Gate | Evidence | Result |
| --- | --- | --- |
| Exact release commit | [`4f4b4e6`](https://github.com/HawkItzme/ContextDroid/commit/4f4b4e6dbf4a8345cd57b749ac69fc84a2f69259) | Protected `main` release commit |
| Exact-commit CI | [Run `30212475399`](https://github.com/HawkItzme/ContextDroid/actions/runs/30212475399) | CI, security, Android smoke, public Android validation, and cross-platform tests passed |
| Packaging and publication | [Run `30212504352`](https://github.com/HawkItzme/ContextDroid/actions/runs/30212504352) | Package dry run, native installer checks, provenance, private-draft verification, and publication passed |
| Stable release | [`v0.1.1`](https://github.com/HawkItzme/ContextDroid/releases/tag/v0.1.1) | Public, non-draft, non-prerelease release |
| Latest stable resolution | [`/releases/latest`](https://github.com/HawkItzme/ContextDroid/releases/latest) | Resolves to `v0.1.1` |
| Initial public install matrix | [Run `30212892622`](https://github.com/HawkItzme/ContextDroid/actions/runs/30212892622) | Five supported OS and architecture targets passed |
| PowerShell 5.1 regression proof | [Run `30287093316`](https://github.com/HawkItzme/ContextDroid/actions/runs/30287093316) | Existing matrix plus PowerShell 7 and Windows PowerShell 5.1 passed |

The public release contains the exact 16-asset contract: five platform archives, DEB, RPM, SBOM,
release manifest, checksums, target metadata, both installers, license, upstream notice, and
third-party notices. GitHub build provenance is attached separately by the successful publication
workflow.
