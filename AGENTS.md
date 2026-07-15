# AGENTS.md — ContextDroid

## Product mission

ContextDroid is an independently maintained, Android-focused downstream product
derived from RTK.

Its purpose is to reduce command-output tokens for AI coding agents while
preserving the evidence required to debug Android projects correctly.

ContextDroid must provide Android-aware handling for:

- Gradle and Android Gradle Plugin output
- Kotlin and Java compiler failures
- KSP and KAPT failures
- AAPT2 and resource failures
- Manifest merger failures
- D8 and R8 failures
- unit and instrumentation tests
- ADB commands
- Logcat crashes, ANRs, StrictMode, Binder, and native-crash references

The product must retain compatible RTK functionality where useful, but its
default behavior must be safer and more conservative.

## Repository heritage

This repository is derived from `rtk-ai/rtk` under Apache License 2.0.

Requirements:

- Preserve the upstream license and applicable notices.
- Add and maintain `UPSTREAM.md`.
- Add and maintain `THIRD_PARTY_NOTICES.md`.
- Record the pinned upstream release and commit SHA.
- Clearly state that ContextDroid is independently maintained.
- Do not imply affiliation with or endorsement by `rtk-ai`.
- Do not reuse RTK token-saving claims as ContextDroid benchmarks.
- Do not erase useful upstream Git history.

## Current product branch

The ContextDroid product work starts from:

```text
RTK stable tag: v0.43.0
Product branch: product/contextdroid-v0.1
```

Treat `upstream/develop` as a reference for selective fixes only.

Do not continuously merge the complete upstream `develop` branch.

## Source of truth

Before making broad changes:

1. Confirm the repository root and active branch.
2. Inspect the actual current code.
3. Read relevant architecture and contributing documentation.
4. Prefer current source and tests over outdated documentation.
5. Do not assume file paths or module names without inspecting them.
6. Record major architectural decisions in `.agent/EXEC_PLAN.md`.

## Development safety

While developing ContextDroid itself:

- Run Cargo, Git, grep, diff, test, lint, and build commands with raw output.
- Do not use RTK or ContextDroid to compress this repository's own output.
- Do not allow an installed RTK or ContextDroid hook to hide development output.
- Never hide build failures, test failures, Clippy output, diffs, or security results.
- Do not alter global Claude, Codex, Cursor, Git, or shell settings without explicit permission.
- Do not publish a release, create a tag, or push release assets without explicit permission.
- Do not perform destructive Git operations without explicit permission.

Required raw verification commands:

```bash
cargo fmt --all --check
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Architecture principles

- Reuse mature RTK infrastructure where it is correct and valuable.
- Do not blindly retain all inherited filters or automatic rewrite rules.
- Separate binary command support from automatic hook rewriting.
- Preserve inherited RTK command families where compatibility tests pass.
- Save untouched stdout and stderr before any lossy transformation.
- Preserve original exit codes and signals.
- Unknown or low-confidence output must fall back to raw output.
- Prefer semantic event extraction over line-count or byte-count truncation.
- Every compact result must explain meaningful omissions.
- Every compact result must provide a raw-output recovery path.
- Keep commands, paths, identifiers, error messages, coordinates, file
  locations, and cause headers exact.

## Required profiles

### `contextdroid-safe`

Default profile.

- Enable verified Android semantic filters.
- Enable only explicitly approved inherited filters.
- Do not automatically rewrite discovery, full-diff, structured-output,
  security, binary, pipeline, or unknown commands.
- Use balanced output mode by default.

### `android-only`

- Automatically rewrite only verified Android Gradle, ADB, and Logcat commands.
- General commands run unchanged.
- Inherited RTK commands may still be called explicitly.

### `rtk-compatible`

- Opt-in inherited RTK-style automatic coverage.
- Clearly document that it is more aggressive.
- Preserve ContextDroid raw recovery and analytics where possible.

## Commands not automatically rewritten in safe profiles

Do not automatically rewrite these unless a specific, tested exception exists:

- `grep`
- `rg`
- `find`
- `tree`
- broad file reads or listings
- full `git diff`
- `curl`
- `wget`
- security scanners
- JSON, XML, CSV, protobuf, or other machine-consumed output
- shell pipelines
- commands whose stdout is redirected into another process
- binary output
- unknown commands

## Runtime pipeline

ContextDroid should follow this model:

```text
Agent requests command
    ↓
Agent-specific hook, rule, or instruction
    ↓
Profile-aware command classifier
    ↓
Unsupported or unsafe?
    ├── yes → run original command unchanged
    └── no
         ↓
Execute original command
         ↓
Capture complete stdout and stderr
         ↓
Persist untouched raw output
         ↓
Parse command-specific structured diagnostics
         ↓
Validate parser confidence and preservation invariants
         ↓
High confidence   → compact semantic output
Medium confidence → semantic output plus relevant raw context
Low confidence    → raw output unchanged
         ↓
Record analytics and omissions
         ↓
Return original exit code or signal
```

Raw output must be saved before any lossy transformation.

## Diagnostic correctness contract

For failed Android operations, preserve when present:

- original command
- working directory
- exit code or signal
- failing Gradle task
- module
- variant, flavor, or build type
- diagnostic category and severity
- exact exception or error type
- exact root message
- every `Caused by` header
- file, line, and column
- dependency coordinates
- conflicting resources, manifests, classes, or artifacts
- expected and actual test values
- application-owned stack frames
- coroutine cause and suppression information
- Logcat timestamp
- PID and TID
- process and package
- thread
- priority and tag
- ANR reason
- native tombstone or crash references
- run ID
- raw-output retrieval command

If a parser cannot preserve the required failure evidence, it must return raw output.

## Output modes

### `lossless`

May:

- remove ANSI/control sequences
- normalize line endings
- collapse byte-identical consecutive lines with exact counts
- reorganize information without deleting unique facts

### `balanced`

Default mode.

May:

- remove successful Gradle task chatter
- collapse classified framework frames
- deduplicate repeated diagnostics
- group equivalent information
- preserve all failure evidence
- report omissions and raw recovery

### `aggressive`

- Must be explicitly selected.
- May produce a highly compact actionable result.
- Must retain raw output and omission accounting.
- Must not be selected automatically for low- or medium-confidence failures.

Explicit verbose flags such as these should normally trigger lossless or raw
behavior:

```text
--stacktrace
--full-stacktrace
--info
--debug
--scan
```

## Raw run storage

Every optimized run should store:

```text
metadata.json
diagnostics.json
summary.txt
stdout.log
stderr.log
```

Required commands or equivalent:

```bash
contextdroid show <RUN_ID>
contextdroid show <RUN_ID> --errors
contextdroid show <RUN_ID> --warnings
contextdroid show <RUN_ID> --causes
contextdroid show <RUN_ID> --json
contextdroid show <RUN_ID> --raw
```

Retention and cleanup must be configurable.

## Android Gradle scope

Initial candidate command families:

```text
./gradlew assemble*
./gradlew bundle*
./gradlew build
./gradlew install*
./gradlew test*
./gradlew connected*
./gradlew managedDevice*
./gradlew lint*
./gradlew dependencies
./gradlew dependencyInsight
```

Unknown custom tasks must not be aggressively classified.

Required alpha diagnostic families:

- Kotlin compiler errors
- Java compiler errors
- KSP failures
- KAPT failures
- Compose compiler diagnostics when present
- AAPT2 and resource-linking failures
- resource merge failures
- Manifest merger failures
- dependency-resolution failures
- duplicate-class failures
- D8 failures
- R8 failures
- lint findings
- unit-test failures
- instrumentation-test failures
- common Gradle and Android Gradle Plugin exceptions

## Stack-trace intelligence

Classify frames as:

- application or source-owned
- generated
- third-party
- Android framework
- Kotlin coroutine
- Gradle or plugin
- native
- unknown

Support configuration for:

```toml
[android]
application_ids = ["com.example.app", "com.example.app.debug"]
source_prefixes = ["com.example"]
generated_prefixes = ["dagger.", "hilt_", "com.example.databinding"]
```

Never remove:

- exception headers
- `Caused by` headers
- source-owned frames
- source locations
- coroutine suppression or cause information
- native tombstone references

Collapsed frames must be counted by category.

## ADB and Logcat scope

Required candidate support:

- `adb devices`
- `adb install`
- `adb uninstall`
- `adb shell am ...`
- `adb shell pm ...`
- selected tested `adb shell dumpsys ...`
- `adb logcat`

Unsupported ADB subcommands must pass through unchanged.

Never transform binary output such as:

- screenshots
- bugreport archives
- APK payloads
- pull or push byte streams
- non-text protocol output

Logcat support should include:

- package-aware filtering
- PID-aware filtering where practical
- crash mode
- ANR mode
- StrictMode mode
- Binder and process-death context
- native crash and tombstone references
- raw mode
- time-window filtering

## Omission transparency

Compact output must report meaningful omissions.

Example:

```text
Preserved:
- 1 root error
- 2 caused-by exceptions
- 4 source-owned frames
- 1 source location

Collapsed:
- 38 successful Gradle tasks
- 27 Android or Gradle framework frames
- 6 duplicate warnings

Run: <RUN_ID>
Raw: contextdroid show <RUN_ID> --raw
```

Omission counts must come from actual transformations.

## Analytics

Retain and extend RTK-style analytics.

Required commands or equivalent:

```bash
contextdroid gain
contextdroid gain --history
contextdroid gain --daily
contextdroid gain --graph
contextdroid gain --scope android
contextdroid gain --command gradle
contextdroid gain --project .
contextdroid gain --since 7d
contextdroid gain --all --format json
contextdroid session
contextdroid discover
contextdroid quality
```

Track:

- command family
- parser
- profile
- project
- raw and returned bytes
- raw and returned lines
- estimated raw and returned tokens
- direct estimated savings
- duration
- exit code or signal
- confidence
- raw fallback
- raw recovery request
- omission counts

Token counts must be clearly labeled as estimates unless an exact tokenizer is
used.

`contextdroid quality` should report available correctness proxies:

- confidence distribution
- fallback rate
- parser error rate
- raw recovery frequency
- raw rerun frequency where detectable
- exit-code parity test status
- fixture preservation status

Distinguish:

1. direct command-output reduction
2. estimated effective savings after raw recoveries and reruns

Do not claim either equals complete model-session billing savings.

## Agent integrations

Implement and document each integration according to the platform's actual
capabilities.

### Claude Code

- `PreToolUse` hook where supported
- profile selection
- idempotent install
- status or show
- dry-run or preview where feasible
- safe uninstall
- no duplicate hook entries
- preserve unrelated user settings

### Cursor

- use actual supported hooks
- test generated configuration
- support status and uninstall

### Codex

- be honest about the integration mechanism
- use `AGENTS.md` or generated instruction files when transparent interception
  is unavailable
- do not claim hook-based interception unless implemented and tested
- preserve direct CLI usage

No integration is supported until install, status, and uninstall behavior are
tested.

## CLI and naming migration

The implementation plan must identify and migrate:

- Cargo package name
- binary name
- `rtk` command literals
- config paths
- cache paths
- data paths
- analytics database names
- environment variables
- hook commands
- generated instruction filenames
- installer URLs
- release asset names
- Homebrew formula
- badges
- translated READMEs
- web links

Preferred command:

```bash
contextdroid
```

Decide explicitly whether to provide:

- an `rtk` compatibility alias
- a migration wrapper
- no alias

Centralize product naming and paths where practical.

## Test requirements

Every parser or rewrite change requires:

- representative raw fixtures
- semantic preservation assertions
- golden output where useful
- malformed and unknown-output tests
- exit-code parity tests
- raw-output recovery tests
- positive rewrite tests
- negative no-rewrite tests

Golden snapshots cannot be the only correctness test.

Required fixture groups:

- Gradle success
- Kotlin compiler failures
- Java compiler failures
- KSP failures
- KAPT failures
- Compose compiler diagnostics
- AAPT2 and resource failures
- Manifest failures
- dependency failures
- duplicate classes
- D8 failures
- R8 failures
- lint
- unit tests
- instrumentation tests
- ADB devices
- ADB install and uninstall
- selected ADB shell commands
- Java crashes
- Kotlin crashes
- coroutine crashes
- ANRs
- StrictMode
- Binder and process death
- native crash references
- malformed output
- unknown output
- verbose and pass-through behavior

Fixtures must be synthetic, generated, or legally redistributable and must not
contain secrets, personal data, or proprietary identifiers.

## Benchmarks

Build a ContextDroid-specific benchmark corpus.

Measure:

- raw estimated tokens
- optimized estimated tokens
- reduction
- confidence
- fallback rate
- critical-field preservation on fixtures
- raw recovery or rerun rate in controlled tests
- latency
- memory overhead

Do not copy RTK's percentage claims.

Correctness gates are release blockers. Compression percentage is not.

## Documentation requirements

Required:

```text
README.md
UPSTREAM.md
THIRD_PARTY_NOTICES.md
CHANGELOG.md
docs/ARCHITECTURE.md
docs/CONTEXTDROID_PRODUCT_SPEC.md
docs/SAFETY_CONTRACT.md
docs/FILTER_MATRIX.md
docs/BENCHMARKS.md
docs/INTEGRATIONS.md
docs/RELEASE_CHECKLIST.md
.agent/EXEC_PLAN.md
```

README must cover:

- product purpose
- Android problem
- RTK heritage
- major differences
- installation
- direct usage
- profiles
- output modes
- Gradle usage
- ADB usage
- Logcat usage
- `gain`
- `session`
- `quality`
- run recovery
- Claude Code integration
- Codex integration
- Cursor integration
- safe defaults and exclusions
- benchmark methodology and measured results
- limitations
- troubleshooting
- uninstall
- contributing
- license and attribution

## Planning and implementation protocol

For the initial ContextDroid product build:

1. Work in Codex Plan mode first.
2. Inspect the repository before proposing target paths.
3. Produce a complete repository-grounded implementation plan.
4. Wait for explicit plan approval.
5. Persist the approved plan to `.agent/EXEC_PLAN.md`.
6. Keep the plan updated as a living execution record with:
   - progress
   - decisions
   - discoveries
   - deviations
   - test results
   - remaining risks
7. Implement milestone by milestone.
8. Continue between ordinary milestones without requesting repeated approval.
9. Stop only for:
   - destructive operations
   - legal or license uncertainty
   - missing credentials
   - publishing
   - modifying global user configuration
   - a requirement that cannot be safely inferred
10. Do not publish a release or push a tag without explicit permission.

## Completion report

For every milestone, report:

1. changed files
2. implemented behavior
3. fixtures and tests added
4. raw verification commands run
5. safety and fallback behavior
6. compatibility impact
7. documentation updated
8. remaining limitations and risks
