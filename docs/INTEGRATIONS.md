# Agent Integrations

`contextdroid integrations <agent> preview|install|status|uninstall` manages only bounded
ContextDroid entries and preserves unrelated settings.

Claude uses `PreToolUse` updated input with `contextdroid hook claude`. Cursor uses
`hooks.json` `preToolUse` only for verified schema version 1 and invokes
`contextdroid hook cursor`. Unknown Cursor schema versions fail closed.

Codex has no claimed transparent command interception. The integration adds a delimited
managed block to a project `AGENTS.md` explaining explicit Android commands, raw exclusions,
and recovery. Uninstall removes only that block.

Preview and status do not write. Lifecycle tests use temporary roots. Actual global
installation changes user configuration and must be initiated explicitly by the user.
