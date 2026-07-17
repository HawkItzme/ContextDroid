# Installation

Binary installation and agent integration installation are separate operations.

## Install the published alpha

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
contextdroid integrations claude preview
contextdroid integrations claude install

contextdroid integrations cursor preview --cursor-schema-version 1
contextdroid integrations cursor install --cursor-schema-version 1

contextdroid integrations codex preview --root .
contextdroid integrations codex install --root .
```

Use `status` to inspect an integration and `uninstall` to remove it. Claude and Cursor may
target user configuration when no root override is provided. Codex modifies only its bounded
managed block in the selected project `AGENTS.md`.

## Other packages

Direct archives, DEB, and RPM packages are attached to the GitHub prerelease. Homebrew requires a
selected downstream tap and separate install/uninstall validation, so it is deferred.

Homebrew is excluded from the first alpha without blocking direct GitHub installation.

See the repository [README](../../../README.md), [integrations guide](../../INTEGRATIONS.md),
and [release checklist](../../RELEASE_CHECKLIST.md).
