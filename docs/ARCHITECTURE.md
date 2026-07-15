# ContextDroid Architecture

## Runtime flow

1. An explicit CLI call or agent-specific integration supplies the original command.
2. The profile-aware classifier applies universal hard stops.
3. The original process executes with stdout and stderr redirected to separate raw files.
4. Raw files are flushed and synchronized before lossy parsing.
5. A command-specific parser creates typed `DiagnosticEvent` records.
6. Preservation rules assign high, medium, or low confidence.
7. High confidence renders semantic output; medium adds raw context; low returns raw.
8. Metadata, diagnostics, summary, checksums, and local analytics finalize.
9. The original exit code or signal-derived shell status is returned.

## Modules

- `src/core/run_store.rs`: validated run IDs, raw artifacts, retention, checksums, status.
- `src/core/runner.rs`: execution ordering and raw-first finalization.
- `src/diagnostics/mod.rs`: typed events, confidence, rendering, omissions.
- `src/cmds/android`: Gradle, ADB, Logcat, and stack classification.
- `src/discover/registry.rs`: profile classifier and universal hard stops.
- `src/core/run_analytics.rs`: local run schema, gain, and quality proxies.
- `src/integrations.rs`: isolated install/status/preview/uninstall lifecycles.
- `src/migration.rs`: conservative RTK preference and legacy analytics migration.

## Key boundaries

Explicit command support and automatic rewrite eligibility are separate. Raw storage is
upstream of every lossy transformation. Analytics failure cannot hide command output or
change process status. Codex instructions are declarative and do not claim interception.

## Decisions

Major decisions, deviations, verification results, and remaining risks live in
`.agent/EXEC_PLAN.md`.
