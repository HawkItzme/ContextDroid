# Installation

Binary installation and agent integration installation are separate operations.

## Install the latest stable release

Linux and macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/HawkItzme/ContextDroid/main/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/HawkItzme/ContextDroid/main/install.ps1 | iex
```

The installers verify the selected archive against `SHA256SUMS` and validate its version before
installation. Download and inspect the scripts first if remote pipe-to-shell installation does
not match your security policy.

## Build from source now

Install Rust 1.91 or later, clone this repository, and run:

```text
cargo build --release
```

The resulting binary is `target/release/contextdroid` on Linux/macOS and
`target/release/contextdroid.exe` on Windows. Put it on `PATH` if desired, then verify:

```text
contextdroid --version
contextdroid --help
```

## Configure an agent after installation

Preview is non-mutating. Install writes only the bounded ContextDroid entry:

```text
contextdroid setup detect
contextdroid setup preview
contextdroid setup apply --yes
contextdroid setup status
```

Use `contextdroid setup uninstall --yes` to remove only managed entries. Claude may target user
configuration. Codex modifies only its bounded managed block in the selected project `AGENTS.md`.
Cursor is experimental and requires explicit opt-in.

## Other packages

Direct archives, DEB, and RPM packages are attached to the GitHub release. Homebrew requires a
selected downstream tap and separate install/uninstall validation, so it is deferred.

Homebrew is excluded throughout v0.1.x without blocking direct GitHub installation.

See the repository [README](../../../README.md), [integrations guide](../../INTEGRATIONS.md),
and [release checklist](../../RELEASE_CHECKLIST.md).
