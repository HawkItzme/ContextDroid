# ContextDroid Architecture

## Runtime flow

1. An explicit CLI call or agent-specific integration supplies the original command.
2. The profile-aware classifier applies universal hard stops.
3. The original process executes with stdout and stderr redirected to separate raw files.
4. Raw files are flushed and synchronized before lossy parsing.
5. A command-specific parser creates typed `DiagnosticEvent` records.
6. Preservation rules assign high, medium, or low confidence.
7. High confidence renders semantic output; medium adds raw context; low returns raw.
8. One canonical analytics recorder stores privacy-safe execution metadata for Android and
   inherited general commands. Successful raw staging is deleted by default; failed artifacts
   finalize under configured retention.
9. The original exit code or signal-derived shell status is returned.

## Modules

- `src/core/run_store.rs` and `secure_fs.rs`: validated IDs, private/no-follow staging,
  failure retention, checksums, and guarded purge.
- `src/core/runtime.rs` and `time_window.rs`: resolved profile/output/Android configuration and
  checked `m/h/d/w` query durations.
- `src/core/runner.rs`: execution ordering and raw-first finalization.
- `src/diagnostics/mod.rs`: typed events, confidence, rendering, omissions.
- `src/cmds/android`: Gradle, ADB, Logcat, and stack classification.
- `src/discover/registry.rs`: profile classifier and universal hard stops.
- `src/core/run_analytics.rs`: schema-v2 canonical recorder, idempotent legacy migration,
  gain/session/quality queries, project hashing, and deterministic UTC ordering.
- `src/integrations.rs`: isolated install/status/preview/uninstall lifecycles.
- `src/migration.rs`: conservative RTK preference and legacy analytics migration.

## Key boundaries

Explicit command support and automatic rewrite eligibility are separate. Raw storage is
upstream of every lossy transformation. Analytics failure cannot hide command output or
change process status. Codex instructions are declarative and do not claim interception.

Logcat snapshot uses a finite time and line bound and a multi-incident state machine. Logcat
stream attaches child I/O directly and performs no semantic buffering. Never-worse validation
runs after evidence validation; incomplete or non-smaller semantic output replays raw.

## Decisions

Major decisions, deviations, verification results, and remaining risks live in
`.agent/EXEC_PLAN.md`.
