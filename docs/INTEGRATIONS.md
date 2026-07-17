# Agent Integrations

`contextdroid integrations <agent> preview|install|status|uninstall` manages only bounded
ContextDroid entries and preserves unrelated settings.

Claude Code on Linux is the supported alpha candidate. It uses `PreToolUse` updated input with
`contextdroid hook claude`. Cursor hooks schema version 1 is experimental until cross-platform
release-commit smoke is recorded; unknown schema versions fail closed.

Codex is guidance-only and has no claimed transparent command interception. It adds a delimited
managed block to a project `AGENTS.md` explaining explicit Android commands, raw exclusions,
and recovery. Uninstall removes only that block.

Preview and status do not write. Lifecycle tests use temporary roots. Actual global installation
changes user configuration and must be initiated explicitly by the user.

If a recognized RTK hook is present, install fails closed and status reports the conflict.
Preview does not propose a coexisting hook. Use `contextdroid migrate rtk --apply` for the only
supported backed-up replacement flow; see [MIGRATION.md](MIGRATION.md).

Preview the exact managed change:

```text
contextdroid integrations claude preview
contextdroid integrations cursor preview --cursor-schema-version 1
contextdroid integrations codex preview --root .
```

Install after review:

```text
contextdroid integrations claude install
contextdroid integrations cursor install --cursor-schema-version 1
contextdroid integrations codex install --root .
```

Replace `install` with `status` or `uninstall` to inspect or remove the managed entry.
