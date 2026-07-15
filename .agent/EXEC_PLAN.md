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
