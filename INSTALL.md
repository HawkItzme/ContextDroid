# Installing ContextDroid

The supported alpha distribution is the GitHub prerelease `v0.1.0-alpha.1`. Branch artifacts and
inherited RTK packages are not ContextDroid releases.

## Quick install

Linux and macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/HawkItzme/ContextDroid/main/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/HawkItzme/ContextDroid/main/install.ps1 | iex
```

The Windows installer uses `%LOCALAPPDATA%\ContextDroid\bin` and adds that directory to the
current-user `PATH` once. Set `CONTEXTDROID_NO_PATH_UPDATE=1` to opt out. Both installers support
`CONTEXTDROID_VERSION`, `CONTEXTDROID_INSTALL_DIR`, and `CONTEXTDROID_RELEASE_BASE`, verify the
archive checksum and binary version, and leave an existing installation intact on failure.

## Build from source

Requirements: Rust 1.91 or newer and Git.

```text
git clone https://github.com/HawkItzme/ContextDroid.git
cd ContextDroid
cargo build --release --locked
```

The binary is `target/release/contextdroid` on Linux/macOS and
`target\release\contextdroid.exe` on Windows. Copy it to a directory on `PATH`, then verify:

```text
contextdroid --version
contextdroid --help
```

## Versioned release archives

After publication, download only assets attached to the matching GitHub release. The intended
archive families are Linux x86_64/ARM64, macOS Intel/Apple Silicon, and Windows x86_64. Exact
filenames are defined by `release/targets.json`; if a release does not contain a listed asset,
that platform is not available for that release.

Download the archive and `SHA256SUMS`, then verify before extraction:

```text
sha256sum --check SHA256SUMS --ignore-missing
```

On macOS, use `shasum -a 256 <archive>` and compare it with `SHA256SUMS`. On Windows, use
`Get-FileHash <archive> -Algorithm SHA256`. Extract the archive and copy only the
`contextdroid`/`contextdroid.exe` binary to a user-controlled directory on `PATH`.

Remote pipe-to-shell installation is optional. Downloading, inspecting, and executing the matching
release installer locally provides the same checksum-verifying behavior.

## Agent integrations

The binary must already be on `PATH`. Preview every mutation:

```text
contextdroid integrations claude preview
contextdroid integrations cursor preview --cursor-schema-version 1
contextdroid integrations codex preview --root .
```

Then install, inspect, or remove the selected integration:

```text
contextdroid integrations <claude|cursor|codex> install
contextdroid integrations <claude|cursor|codex> status
contextdroid integrations <claude|cursor|codex> uninstall
```

Claude Code on Linux is the supported alpha candidate. Cursor schema v1 is experimental until
its cross-platform lifecycle smoke is green on the release commit. Codex installation adds a
bounded `AGENTS.md` guidance block only; it does not intercept shell commands transparently.
All lifecycle operations are designed to preserve unrelated settings and be idempotent.

## Migrating from RTK

ContextDroid has no `rtk` alias and reads no RTK state automatically. Preview explicit migration:

```text
contextdroid migrate rtk --dry-run
contextdroid migrate rtk --apply
```

Apply creates backups before replacing recognized conflicts. It imports only documented safe
preferences and compatible local analytics. See [docs/MIGRATION.md](docs/MIGRATION.md).

## Rollback and uninstall

Remove an integration first, then delete the installed binary. Restore a migration backup if the
migration report says one was created. ContextDroid local state is under the platform data/config
directories shown by `contextdroid privacy status`.

To remove local analytics and raw runs while preserving integrations:

```text
contextdroid privacy purge --yes
```

Then run each installed integration's `uninstall` command and remove remaining ContextDroid
config/data directories manually if a complete purge is required. Review raw logs before sharing
or deletion because they can contain sensitive command output.

## Current limitations

- Homebrew is deferred for the first alpha.
- Cursor remains experimental until its stated release gates pass.
- Codex is guidance-only.
- Android device validation requires a locally configured SDK/device and is not implied by
  installing the binary.
