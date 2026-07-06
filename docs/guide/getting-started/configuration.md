---
title: Configuration
description: Customize RTK behavior via config.toml, environment variables, and per-project filters
sidebar:
  order: 4
---

# Configuration

## Config file location

| Platform | Path |
|----------|------|
| Linux | `~/.config/rtk/config.toml` |
| macOS | `~/Library/Application Support/rtk/config.toml` |

```bash
rtk config            # show current configuration
rtk config --create   # create config file with defaults
```

## Full config structure

```toml
[tracking]
enabled = true              # enable/disable token tracking
history_days = 90           # retention in days (auto-cleanup)
database_path = "/custom/path/history.db"   # optional override

[display]
colors = true               # colored output
emoji = true                # use emojis in output
max_width = 120             # maximum output width

[filters]
# These apply to file-reading commands (ls, find, grep, cat/rtk read).
# Paths matching these patterns are excluded from output, keeping noise low.
ignore_dirs = [".git", "node_modules", "target", "__pycache__", ".venv", "vendor"]
ignore_files = ["*.lock", "*.min.js", "*.min.css"]

[retriever]
mode = "sqlite"             # sqlite (default) | tee (legacy files) | disabled
max_entry_bytes = 10485760  # sqlite: 10 MiB per entry
max_entries = 200           # sqlite: FIFO cap
retention_days = 30         # sqlite: 0 disables age eviction
compression = true          # sqlite: gzip blobs (lossless)
# database_path = "/custom/recall.db"
tee_max_files = 20          # tee mode: rotation
tee_max_file_size = 1048576 # tee mode: per-file cap
# tee_directory = "/custom/tee/dir"

[telemetry]
enabled = true              # anonymous daily ping — see Telemetry & Privacy for full details

[hooks]
exclude_commands = []       # commands to never auto-rewrite
```

For full details on what is collected, opt-out options, and GDPR rights, see [Telemetry & Privacy](../resources/telemetry.md).

## Environment variables

| Variable | Description |
|----------|-------------|
| `RTK_DISABLED=1` | Disable RTK for a single command (`RTK_DISABLED=1 git status`) |
| `RTK_RECALL=0` | Disable the recall store for a single command |
| `RTK_RECALL_DB` | Override the recall database path |
| `RTK_TELEMETRY_DISABLED=1` | Disable telemetry |
| `RTK_HOOK_AUDIT=1` | Enable hook audit logging |
| `SKIP_ENV_VALIDATION=1` | Skip env validation (useful with Next.js) |

## Recall system

When a command fails — or a filter trims a long list — RTK persists the full output to an embedded database and prints a recall hint:

```
FAILED: 2/15 tests
[full output: rtk recall 36365b69eda6]
```

Your AI assistant runs `rtk recall <hash>` to get back exactly what was elided (or just the hidden tail of a trimmed list), without re-running the command. Other forms: `rtk recall <hash> --full | --from N | --lines N | --grep PAT`, `rtk recall --command "cargo test"`, and `rtk recall --list`. Storage is byte-faithful (`BLOB` + lossless gzip).

| Setting | Default | Description |
|---------|---------|-------------|
| `retriever.mode` | `"sqlite"` | `sqlite` (default), `tee` (legacy files), `disabled` |
| `retriever.max_entry_bytes` | `10485760` | Per-entry storage cap (10 MiB) |
| `retriever.max_entries` | `200` | FIFO cap on retained entries |
| `retriever.retention_days` | `30` | Age eviction in days (0 = off) |
| `retriever.compression` | `true` | gzip stored blobs (lossless) |
| Max file size | 1 MB | Truncated above this |

## Excluding commands from auto-rewrite

Prevent specific commands from being rewritten by the hook:

```toml
[hooks]
exclude_commands = ["git rebase", "git cherry-pick", "docker exec"]
```

Patterns match against the full command after stripping env prefixes (`sudo`, `VAR=val`), so `"psql"` excludes both `psql -h localhost` and `PGPASSWORD=x psql -h localhost`.

Subcommand patterns work too: `"git push"` excludes `git push origin main` but not `git status`.

Patterns starting with `^` are treated as regex:

```toml
[hooks]
exclude_commands = ["^curl", "^wget", "git rebase"]
```

Invalid regex patterns fall back to prefix matching.

Or for a single invocation:

```bash
RTK_DISABLED=1 git rebase main
```

## Telemetry

RTK sends one anonymous ping per day (23h interval). No personal data, no file paths, no command content.

Data sent: device hash, version, OS, architecture, command count/24h, top commands, savings %.

To opt out:

```bash
# Via environment variable
export RTK_TELEMETRY_DISABLED=1

# Via config.toml
[telemetry]
enabled = false
```

## Per-project filters

Create `.rtk/filters.toml` in your project root to add custom filters or override built-ins. See [`src/filters/README.md`](https://github.com/rtk-ai/rtk/blob/master/src/filters/README.md) for the full TOML DSL reference.
