# RTK migration

ContextDroid reads no RTK state automatically. Preview the explicit migration first:

```text
contextdroid migrate rtk --dry-run
contextdroid migrate rtk --apply
```

The migration copies only safe display, limit, and retention preferences; archives compatible
local analytics; sanitizes them into the canonical schema; and skips telemetry, trust, database
overrides, and automatic rewrite state. Stable import keys and completion-time matching prevent
the Android durable/legacy pair from being counted twice.

Recognized Claude RTK hooks are conflicts during ordinary integration installation. Only
`migrate rtk --apply` removes recognized RTK entries, deduplicates the ContextDroid entry,
preserves unrelated settings, and writes a timestamped retained backup before atomic replacement.
Dry-run reports conflicts without writing. Uninstall removes only ContextDroid-owned entries.

ContextDroid provides no `rtk` binary alias. Use the opt-in `rtk-compatible` profile only when
inherited automatic coverage is intentionally desired; universal safety exclusions still apply.

Rollback uses the reported integration backup and the pre-migration analytics database backup.
Do not delete those backups until the canonical gain/session/quality views have been verified.
