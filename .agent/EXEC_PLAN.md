# ContextDroid v0.1 Alpha Execution Plan

## Objective

Transform the repository pinned to RTK v0.43.0 into ContextDroid, an independently
maintained Android-focused command-output optimizer. ContextDroid must reduce agent
context use without hiding evidence required to diagnose Android failures. Correctness,
raw recovery, exit-status parity, and conservative fallback are release gates;
compression percentage is not.

## Approved product decisions

- Product identity: Cargo package and binary `contextdroid`, CLI display name
  `ContextDroid`, initial version `0.1.0-alpha.1`.
- Default profile: `contextdroid-safe`, which automatically handles verified Android
  commands plus explicitly approved human-readable `git status` and `git log` forms.
- `android-only` rewrites only verified Android Gradle, ADB, and Logcat commands.
- `rtk-compatible` is opt-in and retains inherited automatic coverage behind universal
  safety stops and ContextDroid recovery/analytics.
- No `rtk` binary alias. Migration is explicit through
  `contextdroid migrate rtk --dry-run` and `--apply`.
- Analytics are local-only. Remove remote telemetry, consent flows, and the `ureq`
  dependency.
- Output mode defaults to `balanced`; `lossless` and `aggressive` are explicit.
  Verbose diagnostic flags normally select lossless/raw behavior.
- Unknown, malformed, unsafe, structured, piped, redirected, or low-confidence output
  passes through unchanged.
- Raw runs are retained under the platform ContextDroid data directory as
  `runs/YYYY/MM/DD/<RUN_ID>/`. Default retention stops at the first of 7 days, 200 runs,
  or 1 GiB, and remains configurable.
- Claude Code may use tested `PreToolUse` input replacement. Cursor support is gated to
  tested hook versions. Codex uses managed repository instructions because its documented
  hooks cannot transparently replace tool input. Unsupported integration mechanisms must
  never be advertised as interception.

## Non-negotiable runtime contract

1. Classify the requested command with the active profile and universal hard stops.
2. Execute the original command and stream stdout and stderr independently to durable
   raw files without a lossy in-memory cap.
3. Preserve the original exit code or signal and finalize/sync raw artifacts before
   parsing or transformation.
4. Parse into typed diagnostic events, build a preservation inventory, validate required
   evidence, and assign `High`, `Medium`, or `Low` confidence.
5. High confidence returns semantic compact output. Medium confidence adds relevant raw
   context (default five lines before and after evidence). Low confidence, parser errors,
   or failed commands with no diagnostic evidence return raw output unchanged.
6. Every compact result includes actual omission counts, a run ID, and
   `contextdroid show <RUN_ID> --raw`.

Each optimized run stores `metadata.json`, `diagnostics.json`, `summary.txt`,
`stdout.log`, and `stderr.log`. Raw stdout and stderr remain separate; replay preserves
their contents and clearly labels streams when they cannot be faithfully interleaved.

## Public interfaces and data model

- Commands: `show`, `runs prune`, `gradlew`, `adb`, `logcat`, `gain`, `session`,
  `discover`, `quality`, `integrations`, and `migrate`.
- Recovery: `show <RUN_ID>` with `--errors`, `--warnings`, `--causes`, `--json`, and
  `--raw`.
- Android configuration supports `application_ids`, `source_prefixes`, and
  `generated_prefixes`; global configuration selects profile, output mode, raw storage,
  retention, and medium-confidence context size.
- Core typed records: `DiagnosticRun`, `RunMetadata`, `DiagnosticEvent`,
  `DiagnosticKind`, `Severity`, `ParseConfidence`, `Cause`, `ClassifiedFrame`,
  `FrameOwnership`, `OmissionReport`, `DiagnosticFingerprint`, and `ParserIdentity`.
- Run metadata records original command, working directory, timestamps, duration,
  exit code/signal, profile, parser, confidence, fallback, raw recovery, checksums, byte/
  line/token estimates, project identity, and omission totals.
- Diagnostic events retain exact category/severity/message/error type, Gradle task,
  module/variant, causes, source locations, coordinates, conflicts, expected/actual test
  values, frames, Logcat process/thread/tag/priority details, and native references when
  present.
- Product names and platform paths are centralized. User-facing environment variables use
  `CONTEXTDROID_*`; local analytics uses `analytics.db`; installers, formulas, assets,
  badges, hooks, and generated instructions use ContextDroid naming.

## Milestones

### M0 — Baseline and safety characterization

- Record the upstream tag/commit, branch, dirty-worktree exclusions, architecture audit,
  and verification baseline here.
- Add focused failing characterization tests for uncapped separate raw streams,
  exit-code/signal parity, low-confidence raw fallback, unsafe no-rewrite cases, and
  multi-task Gradle classification.
- Capture the expected red results before production changes.

### M1 — Provenance and product identity

- Add `UPSTREAM.md` and `THIRD_PARTY_NOTICES.md` while preserving Apache-2.0 files and
  Git history.
- Centralize identity/path constants; rename Cargo package/binary/version and CLI display
  strings. Keep legacy reading only inside explicit RTK migration.
- Update the execution record and make only identity/characterization tests green.

### M2 — Durable raw run store and execution parity

- Replace capped post-filter tee behavior with pre-transform streaming capture to separate
  stdout/stderr files, atomic metadata finalization, checksums, and crash-tolerant partial
  runs.
- Preserve exit codes and signals on success, failure, interruption, parser failure, and
  storage failure. If storage cannot be established, execute/replay safely and report that
  recovery is unavailable rather than hiding command output.
- Implement retention configuration and `runs prune`, including deterministic limit tests.

### M3 — Diagnostic model, validation, and rendering

- Introduce typed events, parser identity/outcome, evidence inventories, confidence
  validation, deterministic diagnostic deduplication, and omission accounting based only
  on performed transformations.
- Implement lossless, balanced, aggressive, medium-context, and raw-fallback renderers.
- Ensure every failed run preserves the complete diagnostic correctness contract or falls
  back to raw.

### M4 — Raw recovery CLI

- Implement `show` selectors and JSON output against stored artifacts, safe run-ID/path
  validation, corrupt/partial-run handling, and raw recovery analytics.
- Test exact stream recovery, filters, missing artifacts, traversal attempts, and versioned
  metadata compatibility.

### M5 — Android Gradle intelligence

- Classify all requested tasks, not only the last non-flag token. Recognize verified
  assemble/bundle/build/install/test/connected/managed-device/lint/dependency families;
  unknown custom tasks remain conservative.
- Add semantic parsers for Kotlin, Java, KSP, KAPT, Compose, AAPT2/resource merge,
  Manifest merger, dependency resolution, duplicate classes, D8, R8, lint, unit tests,
  instrumentation tests, and common Gradle/AGP exceptions.
- Add stack-frame ownership classification and preserve exception/cause headers,
  application frames, source locations, coroutine causes/suppression, and native
  references. Count collapsed frames by category.

### M6 — ADB support

- Add tested semantic handling for devices, install/uninstall, selected `shell am`,
  `shell pm`, and selected `dumpsys` text commands.
- Pass unknown subcommands and binary/protocol operations (screenshot, bugreport archives,
  APK or pull/push streams) through unchanged.

### M7 — Logcat support

- Add package/PID/time-window filtering and `all`, `crash`, `anr`, `strictmode`, `binder`,
  `native`, and `raw` modes.
- Preserve timestamp, PID/TID, process/package, thread, priority/tag, crash causes,
  application frames, ANR reasons, Binder/process-death context, and tombstone references.

### M8 — Profile-aware rewrite safety

- Separate explicit command support from automatic rewrite eligibility.
- Implement profile matrices and universal hard stops for discovery/full-diff, broad reads,
  structured output, security scanners, downloads, binaries, pipelines, redirects, and
  unknown commands.
- Keep safe Git exceptions narrow, human-readable, and covered by positive and negative
  tests. `rtk-compatible` cannot override universal hard stops.

### M9 — Local analytics and quality

- Replace inherited telemetry with a run-centric local SQLite schema and migrations.
- Record family/parser/profile/project, raw/returned sizes and token estimates, duration,
  status/signal, confidence, fallback/recovery/rerun signals, and omission counts.
- Implement gain scopes/history/daily/graph/JSON and quality reports for confidence,
  fallback, parser errors, recovery, detectable reruns, exit parity, and fixture
  preservation. Distinguish direct output reduction from estimated effective reduction.

### M10 — Agent integrations and RTK migration

- Build idempotent install/status/preview/uninstall flows that preserve unrelated settings
  and enforce the most restrictive applicable permission/profile policy.
- Test Claude input replacement, gate Cursor hook output to explicitly tested versions,
  and generate a clearly bounded managed AGENTS block for Codex without claiming command
  interception.
- Keep other inherited integrations experimental/unadvertised until lifecycle tests pass.
- Implement dry-run/apply RTK migration for safe preferences and compatible local analytics;
  do not import legacy hooks, trusted filters, or telemetry state.

### M11 — Documentation, fixtures, and benchmarks

- Produce the required product, architecture, safety, filter-matrix, benchmark,
  integration, release-checklist, README, and changelog documentation.
- Build synthetic or redistributable fixtures for every required Android/ADB/Logcat family,
  malformed/unknown output, verbosity, and pass-through behavior. Golden output supplements
  semantic assertions and is never the only correctness test.
- Benchmark estimated tokens, reduction, confidence/fallback, critical-field preservation,
  controlled recovery/rerun rate, latency, and memory without reusing upstream claims.

### M12 — Packaging and alpha release readiness

- Update installers, Homebrew formula, workflows, release asset names, URLs, badges, and
  translated documentation to ContextDroid destinations.
- Exercise clean install/uninstall, CLI help, migrations, all integrations, and supported
  platforms in CI. Do not publish, tag, or push release assets without explicit approval.
- Run the complete raw verification gate and document all limitations and remaining risk.

## Test and acceptance matrix

Every parser/rewrite increment uses Red-Green-Refactor and includes representative raw
fixtures, semantic preservation assertions, malformed/unknown output, positive rewrite,
negative no-rewrite, exit parity, and raw recovery. Required fixture families are those
enumerated in `AGENTS.md`, including Gradle success and every compiler/resource/dex/test
failure, supported ADB commands, Java/Kotlin/coroutine crashes, ANR, StrictMode, Binder,
native references, verbose output, and pass-through cases.

Release-blocking acceptance criteria:

- Raw stdout/stderr bytes are persisted before lossy transformation with no inherited
  10 MiB truncation.
- Original exit code or signal is returned for all tested execution paths.
- Failed low-confidence or evidence-incomplete parses replay raw output unchanged.
- High/medium compact failures retain every present required evidence field and provide
  valid recovery commands.
- Omission counts exactly match transformations; no estimated or fabricated counts.
- Safe profiles never rewrite universal hard-stop or unknown commands.
- Integration install/status/uninstall is idempotent and preserves unrelated settings.
- Analytics are local-only and claims are explicitly labeled estimates.
- `cargo fmt --all --check`, `cargo test --all`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `cargo build --release` pass with raw output.

## Commit boundaries

Use reviewable commits after green gates: baseline/provenance/identity; raw store/execution;
diagnostic model/recovery; Gradle parsers; ADB/Logcat; profiles/rewrites; analytics;
integrations/migration; documentation/benchmarks/packaging. Do not stage pre-existing
`AGENTS.md`, `.agents/`, or `.codex/` changes. Pushing is outside the authorized scope.

## Living execution record

### Baseline

- Repository: `D:/ContextDroid/ContextDroid`
- Product branch: `product/contextdroid-v0.1`
- Starting commit: `5a7880d404db8364d602f2ecdc41dd790f64013f`
- Pinned upstream: RTK `v0.43.0` at the same commit (the repository also has a `latest`
  tag at this commit).
- Pre-existing worktree state excluded from product staging: staged `AGENTS.md`; untracked
  `.agents/` and `.codex/`.
- Initial audit: CLI is concentrated in `src/main.rs`; execution/stream/tee behavior is in
  `src/core`; rewrite discovery is in `src/discover`; Gradle classification is in
  `src/cmds/jvm/gradlew_cmd.rs`; parsers and analytics are inherited RTK modules. Current
  stream capture caps each stream at 10 MiB, tee is post-filter/failure-oriented, parser
  passthrough may truncate, Gradle uses last-task classification, and Android ADB/Logcat
  semantic support is absent.

### Progress

- [x] Repository/branch/upstream audit and product decisions approved.
- [x] Execution plan persisted.
- [x] M0 baseline and safety characterization.
- [x] M1 provenance and product identity foundation. The remaining inherited-name sweep is
  tracked under M10-M12 because it overlaps generated integrations and packaging.
- [x] M2 durable raw run store and execution parity.
- [x] M3 diagnostic model, validation, and rendering.
- [x] M4 raw recovery CLI.
- [x] M5 Android Gradle intelligence alpha parser.
- [x] M6 ADB support alpha classifier/parser.
- [x] M7 Logcat support alpha classifier/parser.
- [x] M8 profile-aware rewrite safety.
- [x] M9 local analytics and quality.
- [x] M10 integrations and migration.
- [x] M11 documentation, fixtures, and benchmarks.
- [x] M12 packaging implementation and local alpha readiness audit. Publishing remains
  intentionally blocked on explicit approval and the release risks below.

### Decisions and discoveries

- 2026-07-15: User approved the plan and authorized milestone-by-milestone execution,
  excluding publishing, tags, global agent configuration, and destructive Git operations.
- 2026-07-15: Repository commands must use raw output; installed RTK/ContextDroid wrappers
  are forbidden for this repository's own verification.
- 2026-07-15: Rust verification uses pinned toolchain 1.91.0 inside the Visual Studio 2022
  Build Tools developer environment. This was necessary because the host initially lacked
  a working MSVC linker; installation was explicitly approved.
- 2026-07-15: Raw run IDs are validated newtypes and run storage is rooted by
  `CONTEXTDROID_RUNS_DIR` for isolated tests. Capture writes separate stdout/stderr files
  before parsing and finalizes checksummed metadata/artifacts after command completion.
- 2026-07-15: Lossless and low-confidence diagnostic output is exact raw output. Medium
  confidence adds bounded evidence context; high confidence renders typed diagnostics and
  measured omission counts with a raw-recovery command.
- 2026-07-15: Safe automatic rewriting is fail-closed. Shell operators, pipelines,
  redirects (including descriptor duplication), substitutions, structured/full output,
  binaries, unknown commands, and environment-prefixed commands pass through unchanged.
  Plain verified Android commands and narrow human-readable Git status/log forms are the
  only current safe-profile automatic candidates.
- 2026-07-15: Normal Android Gradle execution uses typed semantic diagnostics; the
  inherited parser remains reachable only under `rtk-compatible`. Multi-task families are
  classified conservatively, and mixed/unknown families do not receive aggressive output.
- 2026-07-15: ADB and Logcat support is explicit and fail-closed. Binary/protocol ADB
  operations and unsupported subcommands bypass transformation.
- 2026-07-15: Run-centric analytics are stored only in local `analytics.db`. Remote
  telemetry code, consent flows, documentation, and the direct `ureq` dependency were
  removed. Direct estimated reduction is reported separately from effective reduction
  after raw-recovery requests.
- 2026-07-15: Claude and Cursor lifecycle writers preserve unrelated JSON, reject
  unverified Cursor schemas, refuse destination symlinks, and replace files with a
  restore-on-failure atomic write. Codex uses a bounded managed `AGENTS.md` block and
  explicitly states that transparent interception is unavailable.
- 2026-07-15: The inherited `init` command is quarantined and returns a directed error.
  Its embedded RTK instructions and extra agent installers are not supported in the
  alpha; the tested `integrations` command is the only advertised lifecycle.
- 2026-07-15: Explicit migration is dry-run by default and copies only safe display,
  limit, and tracking-retention preferences. Legacy hooks, trust state, telemetry,
  database path overrides, and automatic rewrite state are skipped; compatible legacy
  analytics are archived in a separately labeled table.
- 2026-07-15: Automatic release triggers and inherited downstream Discord/Homebrew-tap
  publishing jobs were removed. Release workflows are manual inputs and retain only
  ContextDroid asset production; no workflow was run.

### Deviations

- Inherited internal field/type names such as `rtk_cmd` remain where changing them would
  only churn the compatibility implementation or migration schema. Product-facing paths,
  commands, active warnings, assets, URLs, and supported integration content use
  ContextDroid naming; RTK appears publicly only as provenance, explicit migration, or
  opt-in compatibility labeling.
- Durable optimized execution currently completes raw capture before returning transformed
  output instead of live-streaming transformed lines. This ensures raw bytes are durable
  before lossy rendering; streaming UX and interruption tests remain a release-readiness
  risk.
- The first full Windows test run exposed inherited Unix-command assumptions and tests that
  wrote user configuration paths. The tests now use platform shells and temporary roots;
  no global user configuration was changed.
- The static Homebrew formula intentionally contains non-release SHA placeholders. The
  former automatic upstream tap update was removed because no downstream tap destination
  or publication authority has been approved.
- The benchmark document defines and exercises the alpha fixture methodology but does not
  claim production compression percentages. Correctness gates remain release blockers.

### Verification results

- M0 red characterization captured compile failures for missing profile/run-store APIs,
  a 10 MiB stream cap, low-confidence truncation, and last-task Gradle classification.
- Final raw `cargo test --all`: green. Main unit suite ran 2,280 tests with 2,272 passed,
  0 failed, and 8 ignored; Android fixture contract 2/2; guard integration 6/6; remaining
  integration crates contained zero platform-applicable tests.
- Final raw `cargo fmt --all --check`: green.
- Final raw `cargo clippy --all-targets --all-features -- -D warnings`: green.
- Final raw `cargo build --release`: green in 4m30s after the last product-prefix edit.
- Release-binary smoke checks: `contextdroid 0.1.0-alpha.1`; help reports
  `contextdroid-safe`, `android-only`, and `rtk-compatible`; Claude integration preview
  produced the expected `contextdroid hook claude` JSON without writing; legacy `init`
  failed safely with exit 1 and a `contextdroid:` diagnostic.
- Verified behaviors include uncapped separate stream capture, status/signal mapping,
  exact low-confidence fallback, retention limits, traversal-safe recovery selectors,
  typed diagnostics and measured omissions, required Gradle/ADB/Logcat families, stack
  ownership, universal rewrite hard stops, local analytics/quality filters, integration
  idempotence/preservation, migration exclusions, and the complete synthetic fixture list.

### Milestone completion reports

- M0-M1 — Changed Cargo identity, product constants, provenance/notices, baseline tests,
  and this record. Established `contextdroid`/`0.1.0-alpha.1`, pinned RTK heritage, no
  alias, raw command discipline, and MSVC/Rust 1.91 verification environment.
- M2-M4 — Added run store, typed run metadata/diagnostics, rendering, retention, and
  `show`/`runs prune`. Raw stdout/stderr is saved before parsing, low confidence returns
  raw, exact exit status is retained, selectors reject traversal, and recovery is local.
- M5-M7 — Added Android Gradle, ADB, Logcat, and stack modules plus semantic tests.
  Supported text families compact only with sufficient evidence; unknown tasks,
  unsupported ADB, binary protocols, and unrelated Logcat pass through unchanged.
- M8-M9 — Added profile-aware classification, universal hard stops, output-mode behavior,
  local run analytics, `gain`, and `quality`; removed remote telemetry. Compatibility is
  opt-in and cannot bypass pipeline/redirect/structured/binary/security exclusions.
- M10 — Added tested Claude/Cursor/Codex lifecycle management and explicit RTK migration.
  Writes are bounded, atomic, symlink-refusing, idempotent, and preserve unrelated state;
  legacy `init` is disabled and unverified agents are not advertised.
- M11 — Added all required root documentation, replaced misleading inherited manuals,
  and added 30 synthetic Android fixture groups plus semantic fixture-contract tests.
  Benchmark language uses estimates and no inherited percentage claims.
- M12 — Updated installer, formula, package metadata, translated README stubs, CI/CD,
  release assets, and downstream URLs. Automatic publishing side effects were removed.
  Windows raw gates are green; no release, tag, push, global configuration, or destructive
  Git operation was performed.

### Remaining risks

- Unix/macOS CI and actual Android SDK/device execution have not run in this Windows task;
  cross-platform release artifacts, Unix signal behavior, real Gradle/AGP versions, OEM
  Logcat variants, and live ADB behavior still need CI/device evidence.
- Android fixture coverage is complete by required family but synthetic. Additional
  legally redistributable real-world samples should expand parser confidence before a
  stable release.
- Inherited explicit command filters are broad and retain internal RTK-derived naming.
  They are compatibility code, not safe-profile rewrite authority, and need continued
  review when touched.
- The Homebrew formula cannot be published until real release checksums exist and a
  downstream tap/update process is explicitly selected. No release assets currently exist.
- Claude/Cursor schemas and Codex capabilities can change. Cursor is pinned to verified
  schema version 1 and all integrations should be revalidated before publishing.
- Interruption/crash recovery uses durable partial artifacts, but kill-during-finalize and
  long-running live-stream UX need broader platform stress testing.
- Publishing, tags, release assets, pushes, and global integration installation remain
  outside this execution and require explicit user approval.

### Post-implementation documentation clarification

- 2026-07-15: Installation documentation now separates source binary builds, explicit agent
  integration lifecycle, and future published packages. Preview remains the recommended first
  integration action, but concrete install/status/uninstall commands are now shown.
- 2026-07-15: Release-readiness documentation now distinguishes cross-platform/live-Android
  validation blockers, optional Homebrew distribution work, and explicit publication approval.
  Homebrew is not required for a direct-archive alpha; publishing approval is an authorization
  gate rather than a technical blocker.

## Stabilization phase — approved 2026-07-15

### Scope and branch

- Branch: `fix/contextdroid-v0.1-stabilization`, created from
  `product/contextdroid-v0.1` at `3f27260f75dddafe56f4e7c53ea808470e6c0ceb`.
- Upstream remains RTK `v0.43.0` at
  `5a7880d404db8364d602f2ecdc41dd790f64013f`; upstream is reference/heritage only
  and is not a build or runtime dependency.
- Do not merge `develop`, publish, tag, push, change global user configuration, or use
  destructive Git operations. Repository verification output stays raw.
- Preserve the working Linux Claude Code integration and compact `git status` behavior
  as blocking regressions.

### Approved decisions

- Analytics has one local canonical recorder and store for Android and inherited general
  commands. Legacy rows are sanitized/imported without double counting; old tables become
  read-only compatibility inputs.
- Canonical time is UTC epoch milliseconds. Lower bounds are inclusive and equivalent
  durations select identical rows. `--daily`, `--weekly`, and `--monthly` are rolling 24-hour,
  7-day, and 30-day windows.
- `--last` is applied after every other filter and is validated in `1..=1000`.
- Logcat semantic support is a bounded snapshot. Stabilization stream mode is direct
  pass-through with native cancellation and no internal semantic buffer.
- Integration installation fails closed when recognized RTK hooks exist. Only
  `migrate rtk --apply` owns backed-up, atomic RTK-hook replacement.
- Output mode precedence is CLI, `CONTEXTDROID_OUTPUT_MODE`, config, then balanced.
- Raw capture uses secure staging before parsing. Completed successful artifacts are deleted
  by default; failure retention defaults to 7 days/200 runs/1 GiB, while Logcat failure
  retention defaults to 24 hours/50 runs/256 MiB.

### Stabilization milestones

- [x] S0 — Baseline and red canonical-analytics contract. Add an isolated database test that
  records one general and one Android execution and demonstrates that bare gain, rolling
  filters, and scopes currently disagree or duplicate. Re-establish all raw gates.
- [x] S1 — Canonical analytics schema and migration. Add validated duration/count newtypes,
  `ExecutionRecord`/builder/recorder/query, schema v2, migration checkpoints, deterministic
  ordering, privacy-safe project/session IDs, and idempotent imports from `contextdroid_runs`,
  `commands`, and `legacy_rtk_commands`. Stop all legacy writes and deduplicate the Android
  durable/tracker pair.
- [x] S2 — Canonical analytics CLI. Make gain/session/quality share the v2 store. Implement
  orthogonal scope/command/project/profile/parser filters, typed `m/h/d/w` durations,
  rolling presets, post-filter `--last`, all/history compatibility, stable text/JSON/CSV,
  and measured-versus-unknown quality sections.
- [x] S3 — Runtime context, frame ownership, and output modes. Resolve profile/output/config
  once, propagate Android ownership into every parser, preserve unknown source-looking frames,
  positively classify third-party frames, and force verbose Gradle requests to lossless/raw.
- [x] S4 — Never-worse rendering and measured execution evidence. Validate semantic evidence
  before size comparison, replay raw whenever semantic output is incomplete or not smaller,
  record the decision/reason and exact emitted metrics, and stop fabricating exit-parity or
  fixture-preservation success.
- [x] S5 — Bounded Logcat snapshot and incident state machine. Add `snapshot`/`stream`, keep
  the old flat form as a deprecated snapshot alias, execute snapshots with a finite time/line
  bound, segment all incidents, filter each incident independently, preserve crash/ANR/native
  evidence, and keep stream zero-buffer pass-through.
- [x] S6 — Shared Android command classification and rewrite safety. Parse every Gradle task
  and option value, treat `clean` as neutral only alongside verified tasks, fail closed on an
  unknown task, align supported ADB auto-rewrites, and replace only a true executable `rtk`
  token in compatibility mode without touching quoted data or paths.
- [x] S7 — Integration conflict migration. Add RTK-hook conflict status/preview, fail-closed
  install, retained user-only backups, atomic recognized-entry replacement in explicit RTK
  migration, and temporary-HOME lifecycle tests. Preserve unrelated JSON and the Linux Claude
  compact `git status` smoke behavior.
- [x] S8 — Fixture preservation contracts. Replace existence checks with a data-driven
  manifest that exercises parser, structured assertions, renderer, secure run store, raw
  recovery, exit parity, malformed fallback, confidence, omissions, and every required
  Android/ADB/Logcat diagnostic family.
- [x] S9 — Privacy and secure storage. Add user-only/no-follow atomic filesystem operations,
  reject symlink/reparse/traversal and raw stores inside source repositories, bound parser
  memory without truncating durable raw capture, enforce failure-only retention, and add
  `privacy status`, analytics export/reset, runs list/purge, and guarded data purge.
- [x] S10 — CI and real smoke coverage. Add `develop` PR coverage; deterministic docs/help
  validation; required raw fmt, clippy, test, and release-build jobs on Linux/Windows/macOS;
  a pinned AGP 9.3/Gradle 9.5/JDK 17/API 37 sample; bounded fake-ADB Logcat smoke; and Linux
  temporary-HOME Claude lifecycle plus exact compact `git status` smoke.
- [x] S11 — Documentation and merge-readiness audit. Update README, architecture, safety,
  filter matrix, benchmarks, integrations, release checklist, product spec, changelog,
  `CLAUDE.md`, and new `docs/MIGRATION.md`; remove unsupported RTK-facing claims; run every
  raw gate and document remaining platform/device limitations.

### Canonical analytics design

- Schema version 2 uses `contextdroid_executions` with an autoincrement execution ID,
  stable unique import key/origin, UTC `started_at_ms`, privacy-safe session/project IDs,
  scope, command family, operation, parser, profile, output mode, execution source,
  raw/returned bytes/lines/token estimates, duration, exit/signal, confidence, fallback,
  parser/recovery/rerun signals, optional run ID, and non-sensitive omission JSON.
- Full commands, arguments, paths, package names, device serials, file/error text, and Logcat
  contents are not stored in analytics. Per-install salt protects project/session hashing.
- `analytics_migrations` stores import/checkpoint state separately from resettable analytics.
  Import keys are `run-v1:<run_id>`, `legacy-local:<id>`, and `legacy-rtk:<id>`.
- Android v1/legacy duplicates match on sanitized family/operation/project/metrics and a
  completion-time tolerance of five seconds. Stable keys and high-water rescans make imports
  idempotent while compatibility tables exist.
- Query order is `started_at_ms DESC, execution_id DESC`; filters run before limiting.
  Bare gain is all-time/all-scope. History aliases `--last 20`. Presets conflict with each
  other and `--since`; `--all` conflicts with time filters.
- Session groups canonical opaque session IDs and reports unattributed executions separately.
  Quality reports observed confidence/fallback/parser/recovery/never-worse metrics and labels
  exit-parity/fixture status unknown unless versioned test evidence exists.

### Safety architecture

- Use validated newtypes for durations/counts/identifiers, builders for execution records,
  RAII for secure staging and cleanup, and an explicit state machine for Logcat incidents.
- Use `std::process::Command` argument arrays only; never execute reconstructed shell strings.
  Add injection tests for metacharacters, substitutions, quotes, newlines, malformed/binary
  output, oversized input, traversal, symlink, and Windows reparse cases.
- Snapshot Logcat defaults to a rolling 10-minute dump and a 20,000-line cap. Semantic
  snapshots reject binary/file/rotation/clear/unbounded flags. Automatic `adb logcat` maps to
  snapshot. Streaming analytics store only observable completion metadata and unknown sizes.
- A shared classifier covers verified Gradle tasks and safe ADB text commands. All tasks must
  be verified; `clean` alone and unknown custom tasks pass through. ADB auto-rewrite is limited
  to devices, install/install-multiple/uninstall, selected `am` actions, selected `pm` queries,
  narrowed activity/package/meminfo dumpsys, and Logcat snapshot.
- Secure storage uses 0700 directories/0600 files plus no-follow/create-new/fsync/atomic rename
  on Unix and explicit reparse-point rejection/current-user-only access on Windows.

### Per-milestone evidence and commit boundaries

- Every production change follows Red-Green-Refactor: capture the focused failing test, make
  only that contract green, then run the relevant module/integration suite and raw fmt/clippy.
- Reviewable commit subjects after green gates:
  `test(stabilization): characterize analytics and safety gaps`,
  `fix(analytics): unify general and Android execution records`,
  `fix(android): preserve application frames and propagate output mode`,
  `fix(runner): enforce semantic never-worse guarantees`,
  `fix(logcat): add bounded snapshots and incident segmentation`,
  `fix(rewrite): align Android routing and compatibility rewriting`,
  `fix(integrations): detect and migrate RTK hooks safely`,
  `test(android): enforce parser preservation contracts`,
  `fix(privacy): harden analytics and raw run storage`,
  `ci: add develop gates and Android/Linux smoke coverage`, and
  `docs: align product behavior and release evidence`.
- Each milestone records changed files, behavior, tests/fixtures, raw commands, safety/fallback,
  compatibility, docs, discoveries/deviations, and remaining risk in this file before moving on.

### Stabilization verification log

- 2026-07-15: Clean baseline confirmed at `3f27260`; branch created. Prior planning-turn fresh
  evidence was `cargo fmt --all --check` green. The earlier alpha record reports all raw gates
  green, but S0 must independently re-establish them for this branch.

### Stabilization risks at phase start (resolved or superseded below)

- The current v1 durable table and inherited `commands` table share `analytics.db`; Android
  diagnostic execution writes both, while general inherited filters primarily write the
  legacy table. Current gain flags route between incompatible readers.
- Current Logcat semantic execution can be unbounded and its parser selects only one loosely
  matched incident. Current Android ownership configuration is not propagated to Gradle stack
  collapse, causing application frames to be classified as removable third-party frames.
- Current raw run store retains every optimized success, writes predictable files without the
  full no-follow/user-only contract, and stores full commands/paths in local metadata/analytics.
- Real Android SDK/device and Linux Claude evidence are absent on this Windows host until S10
  runs in CI or an equivalent Linux/Android environment.

### Exact first implementation task

Add a temp-database integration contract that records one inherited general Git execution and
one Android diagnostic execution, then asserts bare gain, rolling weekly gain, and Android/general
scopes all read the same source and return exactly two non-duplicated executions. Run it raw and
record the expected failure before changing production analytics.

### Stabilization completion record — 2026-07-16

All S0-S11 implementation milestones are complete on
`fix/contextdroid-v0.1-stabilization`. No commit, merge, push, tag, release, global user
configuration change, or destructive repository operation was performed.

#### Milestone evidence

- S0 captured the split-store failure: the red canonical contract observed one execution
  through one reader and two through the combined expectation. The same contract is green.
- S1-S2 added schema-v2 `contextdroid_executions`, OS-random install salt, salted project IDs,
  deterministic UTC ordering, idempotent legacy imports, completion-time Android deduplication,
  canonical `gain`/`session`/`quality`, rolling presets, typed `m/h/d/w`, orthogonal filters,
  post-filter `--last`, history, graph, and text/JSON/CSV output. Pass-through sizes are unknown,
  not fabricated zeroes. Windows project paths are normalized before hashing.
- S3-S4 propagate runtime profile/output/Android ownership, preserve unknown source-looking
  frames, positively classify collapsible frames, honor verbose Gradle raw behavior, and apply
  the never-worse evidence/size guard before semantic output. Quality no longer claims unmeasured
  exit parity or fixture success.
- S5-S6 add bounded 10-minute/20,000-line Logcat snapshots, zero-buffer pass-through streams,
  independent multi-incident parsing, package/PID-safe filtering, shared Gradle/ADB classification,
  fail-closed unknown tasks, and token-aware RTK executable migration. Automatic Logcat rewriting
  accepts only plain `adb logcat`; custom/dump forms stay unchanged.
- S7 adds fail-closed recognized RTK hook conflicts and explicit backed-up migration while
  preserving unrelated integration JSON. Temporary-root lifecycle tests and smoke pass.
- S8 replaces existence-only fixtures with a 30-entry synthetic contract covering every required
  Android family from parsing through semantic assertions, rendering, durable finalization, and
  byte-exact raw recovery.
- S9 adds no-follow/reparse/traversal defenses, private Unix modes, secure staged replacement,
  failure-only default run retention, repository-root rejection, storage lifecycle commands,
  privacy status, and canonical analytics export/reset. Analytics has no network client.
- S10 adds the three-OS test/release matrix, deterministic docs/help gate, pinned AGP 9.3 /
  Gradle 9.5 / JDK 17 / API 37 sample, Android failure smoke, and Linux temporary-home Claude plus
  compact Git-status smoke. The inherited secret-backed AI review is disabled.
- S11 updates README, architecture, safety, filter matrix, benchmarks, integrations, migration,
  release checklist, product spec, changelog, and contributor instructions.

#### Final raw verification

- `cargo fmt --all --check`: green.
- `cargo test --all`: green; main suite 2,281 passed, 0 failed, 8 ignored; fixture manifest 1/1;
  guard integration 6/6; all other applicable integration targets green.
- `cargo clippy --all-targets --all-features -- -D warnings`: green.
- `cargo build --release`: green in 3m34s after the final source change.
- `git diff --check`: green. Source scan found no analytics/network client. `getrandom 0.4.2`
  was already present through `tempfile` and is now a direct dependency for the privacy salt.
- Documentation contract: green through an equivalent raw PowerShell check. The Bash script could
  not run through the Windows `bash` alias because this host has WSL enabled without an installed
  distribution; Git Bash subsequently ran the real script successfully and validated all smoke
  script syntax, including the bounded fake-ADB fixture.
- Release help: green for `--profile`, post-filter `--last`, rolling daily, bounded snapshot,
  pass-through stream, and typed `--since` surfaces.

#### Smoke results

- Windows isolated Claude lifecycle: preview/install/status/uninstall green with no global writes.
- Windows isolated compact Git status: green and preserved exact ` M tracked.txt` status evidence.
- Windows isolated canonical analytics: green for bare/general Git recording, project/profile/time
  filters, post-filter last, daily/weekly/monthly, graph, quality, and session from one database.
- Linux Claude script and real Android AGP sample are implemented as required CI jobs but were not
  executable locally. This Windows host has neither a WSL distribution nor the required Android
  SDK/Gradle environment; CI results are therefore still pending.

#### Decisions, deviations, and compatibility

- To provide the required `gain --profile` and `quality --profile` filters without a Clap name
  collision, the rewrite profile remains `contextdroid --profile <PROFILE> <COMMAND>` and must
  precede the subcommand; analytics owns `--profile` after `gain`/`quality`.
- Production execution writes only the canonical recorder. A `cfg(test)` legacy tracker write
  remains solely to keep inherited tracker unit contracts isolated; it cannot affect production.
- Successful diagnostic runs securely stage raw output and then delete it after analytics unless
  `CONTEXTDROID_RETAIN_SUCCESSES=1`; failed runs retain exact recovery artifacts.
- Windows secure storage rejects reparse points and inherits the current user's protected local
  data-directory ACL. Explicit DACL replacement was not added because it can corrupt managed or
  enterprise inheritance; this remains a pre-stable hardening item.

#### Remaining risks

- New Linux/macOS/Android/Claude CI jobs have not run for this unpushed branch. Merge readiness is
  conditional on their green results, including `cargo audit` and Semgrep; `cargo-audit` is not
  installed on this host.
- Live devices, OEM Logcat variants, instrumentation tests, and cancellation under real ADB still
  need device-lab evidence. The sample currently covers Gradle success, Kotlin, AAPT, and unit-test
  failures in CI.
- Canonical session grouping supports opaque session IDs, but direct and current integration paths
  are reported as `unattributed` until agent-provided session identity is wired through execution.
- Windows current-user DACL assertion, kill-during-finalize stress, and broader legally
  redistributable real-world Android fixtures remain pre-stable hardening work.
- The branch is intentionally uncommitted. Review and CI must precede merging; publication remains
  separately gated by explicit approval.

## Approved final release-readiness plan — 2026-07-17

The final `v0.1.0-alpha.1` readiness plan was approved for implementation on
`fix/contextdroid-v0.1-release-readiness` at `e5f80c2cfd87c5e448d3e2b952319ea38cd6a424`.
This section supersedes earlier claims that CI had not run: PR-head runs `29465606705` and
`29472266156` completed and failed. Linux, macOS, and Windows tests exposed a timezone-dependent
Logcat assertion; macOS also exposed `/var` temporary-root symlink handling; Android smoke requested
an unavailable stable-channel API 37 package. Fresh local Windows gates on the approved starting
commit were green with 2,284 passed and 8 ignored in the main suite, not the stale recorded 2,281.

### Locked policy decisions

- Contribution policy is Apache-2.0 inbound=outbound. ContextDroid has no CLA or DCO gate.
- The first alpha is a manually approved GitHub prerelease. Inherited release-please state and
  automatic publishing are disabled; ContextDroid-specific automation is deferred until after alpha.
- Real validation uses the Apache-2.0 `android/architecture-samples` project plus one permissioned
  internal organization Android project. Only redacted aggregate evidence may be committed.
- Stable Android CI uses API 36 with AGP 9.3, Gradle 9.5, Build Tools 36, and JDK 17.
- Direct archives, DEB, and RPM are alpha channels. Homebrew publication is deferred.
- Claude Linux is the only hook integration eligible for supported status. Cursor schema v1 remains
  experimental; Codex is managed `AGENTS.md` guidance and never transparent interception.
- No push, PR creation, merge, tag, release, asset upload, default-branch/ref mutation, global user
  configuration change, or package publication is authorized by implementation approval alone.

### Implementation milestones and commit boundaries

1. `fix(ci): restore deterministic cross-platform readiness gates`
   - Make Logcat time construction testable with a fixed offset while production uses local time.
   - Canonicalize macOS test temporary roots without weakening managed-root symlink rejection.
   - Use stable API 36 in the Android smoke project and workflow.
2. `test(android): prove semantic evidence preservation`
   - Replace raw-fixture substring assertions with typed event, cause, location, application-frame,
     dependency/test field, renderer, omission, confidence, never-worse, exit/signal, run-ID, and
     byte-exact stdout/stderr recovery assertions.
   - Add schema-v2 diagnostic fields and evidence-inventory validation wherever the new red contracts
     prove the current parser or renderer incomplete.
3. `fix(quality): represent unmeasured runtime correctness as unknown`
   - Make fixture preservation and independent exit parity nullable, migrate v1 asserted truth to
     unknown, make parser failures observable, and test text/JSON `quality` output.
4. `refactor(integrations): remove unsupported inherited agent surfaces`
   - Remove legacy `init`, unsupported hook commands/assets, and OpenClaw from the alpha surface while
     preserving Claude/Cursor/Codex lifecycle and explicit RTK backup/rollback.
5. `docs: replace inherited RTK legal and public identity`
   - Replace contribution, security, install, disclaimer, changelog, nested docs, generated agent
     instructions, scripts, source READMEs, user strings, links, contacts, telemetry statements, and
     unsupported claims. Add a blocking deterministic stale-brand policy.
6. `chore(release): disable inherited release-please for first alpha`
   - Remove active release-please manifests/workflows and align Cargo, changelog, tag contract, and
     release-title version state at `0.1.0-alpha.1`.
7. `fix(installer): unify target and release asset contracts`
   - Add canonical target metadata for Linux x86_64 musl, Linux ARM64 GNU, macOS Intel/ARM, and Windows
     x86_64. Execute the real installer/uninstaller against local artifacts and reject unsafe archives.
8. `ci(release): add locked packaging dry run and protected publication`
   - Run raw locked gates; pin packaging tools; validate tag/version/main SHA; build and test the full
     artifact set; generate checksums, manifest, notices, SBOM, and provenance; publish only a complete
     verified draft as a prerelease using least-privilege `GITHUB_TOKEN`.
9. `ci(security): make dependency and dangerous-change policy blocking`
   - Make cargo-audit, cargo-deny, Semgrep, shell/network/unsafe/dependency policy, code-owner review,
     and PR-base comparisons blocking and self-tested.
10. `test(android): record redistributable and permissioned workload validation`
    - Record redacted Gradle, compiler/processor, resource/manifest/dex/test, ADB, Logcat, fallback,
      exit-parity, and raw-recovery evidence from the exact candidate commit.
11. `docs(launch): publish benchmarks, pilot guide, issue forms, and release notes`
    - Publish reproducible direct/effective reduction, preservation, fallback, latency, and memory
      evidence plus launch examples, limitations, privacy guidance, and the opt-in 3–5 developer pilot.
12. `chore(branches): migrate stable references from master to main`
    - Update repository text only during implementation. Actual GitHub ref/default migration remains a
      separate explicitly approved administration operation.

### Merge, branch, and release gates

- The readiness branch targets `product/contextdroid-v0.1`, the current product integration line.
  Current `origin/develop` is inherited post-v0.43 RTK development and must never be merged wholesale.
- After a green reviewed product commit `R` and separate branch-administration approval, preserve
  inherited refs under `archive/upstream-master-v0.43.0` and
  `archive/upstream-develop-20260716`, create ContextDroid `develop` and `main` at `R`, set `main` as
  default, and rerun every protected gate on exactly `R`.
- Merge requires green raw fmt, locked tests, locked Clippy, locked release build, docs/help/brand,
  Android and Claude smoke, security, and packaging dry run with resolved code-owner review.
- Release additionally requires the real Android/device matrix, truthful integration support labels,
  reviewed release notes/checksums/manifest/notices/SBOM/provenance, complete artifacts, protected
  exact-main CI, and explicit authorization to create/push `v0.1.0-alpha.1` and publish assets.

### Rollback contract

- Revert the smallest milestone and rerun all gates; never move a published tag.
- Failed publication may leave only a private draft and must never expose partial assets.
- Installer rollback uses a pinned prior version or uninstall; integration rollback restores the
  retained RTK backup; branch rollback uses archived refs only with explicit administration approval.
- A confirmed missing root cause or raw-recovery failure pauses release/pilot and forces the affected
  command family to raw behavior.

### Exact first implementation task

Use the existing failed CI evidence as RED. Add a fixed-offset Logcat cutoff contract that the current
`DateTime<Local>` helper cannot accept, make the helper timezone-generic while production still passes
`Local`, canonicalize temporary test roots on macOS, move the smoke sample to stable API 36, and rerun
the previously failed focused tests before beginning parser-preservation work.

### Implementation record — 2026-07-17

Milestones 1–9 and the repository implementation portions of 10–12 are complete in the working
tree; no commit, push, tag, release, asset upload, repository setting, or global agent configuration
change was performed.

- Cross-platform regressions: the Logcat cutoff helper accepts fixed offsets, production still uses
  local time, test temporary roots are canonicalized for the macOS `/var` system symlink, and Android
  smoke uses stable API/build-tools 36. Focused tests and the first full locked suite were green.
- Preservation: the 30-family manifest now asserts typed root messages, every declared cause,
  locations, application frames, task/module/variant, dependency coordinates, test assertion values,
  details, omissions, confidence, never-worse decisions, durable IDs, exit/signal behavior, and exact
  stdout/stderr recovery. KAPT failed-task banners are no longer misclassified as root diagnostics.
- Quality: run metadata schema v2 uses nullable fixture preservation and exit parity. Schema-v1
  asserted values normalize to unknown. Live parsers return `Result`; an observed parser error records
  low confidence and returns raw output. Ordinary runtime paths never claim fixture success.
- Integrations/public identity: disabled legacy `init`, its source/tests, inherited deployable hook
  assets, and OpenClaw were removed. Internal Claude/Cursor hook processors remain hidden implementation
  entry points for the verified lifecycle. Contribution policy is Apache-2.0 inbound=outbound; security
  uses GitHub private reporting; install/disclaimer/support text is ContextDroid-specific.
- Release: inherited release-please/next-release flows are disabled and the version manifest is
  `0.1.0-alpha.1`. `release/targets.json` contract-tests five archives against workflow, installer,
  and documentation. Packaging uses locked Rust gates, pinned cargo-deb 3.7.0 and cargo-generate-rpm
  0.21.0, Rust 1.91 in Fedora, `SHA256SUMS`, local installer execution, built-in `GITHUB_TOKEN`, and
  exact tag/version/main/SHA validation before the conditional publish job.
- Security/branches: cargo-audit, cargo-deny, Semgrep, dangerous additions, PR-base comparison, and
  CODEOWNERS policy are checked in. Stable documentation/workflow references use `main`; actual branch
  administration remains unauthorized and undone.
- Validation/launch: the public Apache-2.0 `android/architecture-samples` workload is pinned at
  `ee66e1526b84c026615df032c705842b7d2a521f` in CI. A redacted internal pilot template, pilot guide,
  issue forms, support/roadmap, draft release notes, and reproducible fixture benchmark are present.

Local Android discovery found SDK platforms/build-tools 35 and 36, but only JDK 23 and an obsolete
`sdkmanager` that fails with missing JAXB. A raw public-project Gradle attempt emitted no output before
being stopped; it is explicitly recorded as inconclusive, not a pass. Clean JDK 17 CI and the
permissioned internal pilot remain release blockers.

Verification completed during implementation:

- initial post-regression `cargo fmt --all --check`: green;
- initial post-regression `cargo test --all --locked`: 2,284 passed, 8 ignored in the main suite plus
  all integration targets green;
- initial post-regression locked Clippy with warnings denied: green;
- focused semantic contract, metadata, parser-error, release-contract, and security-contract tests:
  green;
- all-target compilation after removing legacy init/hook assets: green;
- workflow YAML parse: green for every `.github/workflows/*.yml` file.

Final raw fmt/test/Clippy/release-build gates must be rerun after the complete diff. External blockers
remain exactly those listed in `docs/RELEASE_CHECKLIST.md`: new cross-platform CI and package dry run,
clean public-project CI, permissioned internal validation, repository administration, and explicit
publication approval.

#### Final local gate result after dependency security remediation

The blocking local audit found `RUSTSEC-2026-0190`, `RUSTSEC-2026-0204`,
`RUSTSEC-2026-0194`, and `RUSTSEC-2026-0195`. No ignore was added. `anyhow` was upgraded to
1.0.103, `crossbeam-epoch` to 0.9.20, and `quick-xml` to 0.41.0; the two small .NET XML call sites
were migrated to the non-deprecated 0.41 APIs and all focused .NET tests passed.

Final raw gates on the remediated lockfile:

- `cargo fmt --all --check`: green;
- `cargo test --all --locked`: green; main suite 2,131 passed, 9 ignored, plus fixture manifest
  1/1, guard integration 6/6, release contract 2/2, and security contract 1/1;
- `cargo clippy --all-targets --all-features --locked -- -D warnings`: green;
- `cargo build --release --locked`: green in 3m55s;
- `cargo audit --deny warnings`: green after remediation;
- `cargo deny check advisories bans licenses sources`: green (duplicate-version notices are warnings
  under the checked-in policy);
- Git Bash documentation/stale-brand contract and shell syntax checks: green;
- workflow YAML parse and release help contract: green;
- `git diff --check`: green.

The lower main-suite count is intentional: the disabled inherited `init` module and its large test
surface were removed; one ignored reproducible benchmark emitter was added. This is not presented as
equivalent functional coverage for the removed unsupported integrations.

### Authorized distribution completion — 2026-07-17

The maintainer authorized full repository administration and publication of the exact gated
`v0.1.0-alpha.1` GitHub prerelease. The install experience is a checksum-verifying binary installer
followed by one explicit Claude, Cursor, or Codex integration command. Alpha branch governance is
CI-gated without mandatory human approval until a second collaborator exists.

A one-time permissioned Android-project validation ran outside the tracked repository. The release
binary completed unit-test/APK workloads and preserved the exact failing task, Kotlin location,
identifier, compiler cause, raw fallback, and nonzero exit for a controlled compiler failure. No
project name, URL, commit, source, logs, fixture, evidence artifact, or future CI dependency is
retained in ContextDroid.

Distribution completion adds a transactional Windows PowerShell installer, makes the Unix installer
prerelease-aware, verifies checksum/path/version failure modes, uses current native release runners,
generates a release manifest without self-referential checksums, tests DEB/RPM installation, and
keeps GitHub publication private until every draft asset matches the verified output set. CI now
supports exact-commit push/manual gates with immutable action pins and safe comparison bases.

Local distribution-completion gates are green: `cargo fmt --all --check`; `cargo test --all
--locked` with 2,132 passed and 9 intentionally ignored in the main suite plus all integration
targets; warnings-denied all-feature Clippy; the locked optimized release build; three release
contracts; two security-workflow contracts; Git Bash documentation and shell syntax; PowerShell
installer syntax, success, checksum-tamper, and traversal tests; workflow YAML parsing; RustSec
audit; cargo-deny advisories/bans/licenses/sources; and `git diff --check`. Semgrep is not installed
on this host and remains a blocking pinned CI job.
