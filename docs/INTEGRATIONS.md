# Agent Integrations

`contextdroid setup` detects and manages bounded ContextDroid entries while preserving unrelated
settings.

Claude Code on Linux is supported. It uses `PreToolUse` updated input with
`contextdroid hook claude`. Cursor hooks schema version 1 is experimental and excluded unless
`--include-experimental` is supplied; unknown schema versions fail closed.

Codex is guidance-only and has no claimed transparent command interception. It adds a delimited
managed block to a project `AGENTS.md` explaining explicit Android commands, raw exclusions,
and recovery. Uninstall removes only that block.

Detect, preview, and status do not write. Apply preflights all selected adapters, creates backups,
and rolls the selected set back if any write fails. Actual installation changes user or project
configuration and requires an interactive confirmation or `--yes`.

If a recognized RTK hook is present, install fails closed and status reports the conflict.
Preview does not propose a coexisting hook. Use `contextdroid migrate rtk --apply` for the only
supported backed-up replacement flow; see [MIGRATION.md](MIGRATION.md).

Detect and preview the exact managed change:

```text
contextdroid setup detect
contextdroid setup preview
```

Apply after review, then inspect or uninstall:

```text
contextdroid setup apply --yes
contextdroid setup status
contextdroid setup uninstall --yes
```

Use repeatable `--only` flags to select agents. Cursor additionally requires
`--include-experimental`. The agent-specific `contextdroid integrations` lifecycle is retained
throughout 0.1.x and uses the same underlying engine; removal will not occur before 0.2.0.
