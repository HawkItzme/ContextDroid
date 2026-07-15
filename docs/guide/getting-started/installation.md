# Installation

No alpha release has been published. Binary installation and agent integration installation
are separate operations.

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

## Configure an agent after building

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

## Published packages later

`install.sh` and `Formula/contextdroid.rb` are packaging inputs, not proof that downloadable
artifacts exist. The shell installer becomes usable only after versioned release archives and
`checksums.txt` are published. Homebrew additionally requires real SHA-256 values, a selected
downstream tap, and install/uninstall testing on supported macOS architectures.

Homebrew may be excluded from the first alpha without blocking direct GitHub archives. Publishing
any tag, release, or asset requires explicit approval.

See the repository [README](../../../README.md), [integrations guide](../../INTEGRATIONS.md),
and [release checklist](../../RELEASE_CHECKLIST.md).
