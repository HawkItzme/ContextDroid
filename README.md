# ContextDroid

ContextDroid is an independently maintained, Android-focused command-output optimizer
derived from RTK. It extracts useful Android diagnostics for AI coding agents while saving
the complete stdout and stderr of every optimized run for exact recovery.

ContextDroid is not affiliated with or endorsed by `rtk-ai`. The upstream provenance and
pinned commit are recorded in [UPSTREAM.md](UPSTREAM.md); third-party notices are in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Why Android needs a conservative tool

Gradle, AGP, Kotlin, Java, KSP, KAPT, AAPT2, manifest merger, D8/R8, test, ADB, and
Logcat output can be extremely verbose. Blind truncation is dangerous because a useful
root cause may be separated from the failing task, source location, or `Caused by` chain.
ContextDroid captures raw output first, extracts typed diagnostics second, and returns raw
output unchanged whenever parser confidence is low.

## Install

This alpha is built from source:

```text
cargo build --release
```

The binary is `target/release/contextdroid` (or `contextdroid.exe` on Windows). Packaging
installers are not release-ready yet; see [docs/RELEASE_CHECKLIST.md](docs/RELEASE_CHECKLIST.md).

## Direct usage

```text
contextdroid gradlew assembleDebug
contextdroid gradlew testDebugUnitTest
contextdroid adb devices
contextdroid adb install app-debug.apk
contextdroid logcat --mode crash --package com.example.app
```

Unsupported commands are not automatically transformed. In safe profiles, call inherited
compatibility commands explicitly only when you understand their output behavior.

## Profiles

- `contextdroid-safe` is the default. It automatically considers verified Android commands
  and narrow human-readable Git status/log forms.
- `android-only` automatically considers only verified Gradle, ADB, and Logcat commands.
- `rtk-compatible` opts into inherited coverage, but cannot bypass universal hard stops.

Select a profile with `--profile` or `CONTEXTDROID_PROFILE`. Pipelines, redirects,
substitutions, structured/full output, security tools, downloads, binary protocols,
unknown commands, and broad discovery/read operations pass through unchanged.

## Output modes

- `lossless` removes no unique diagnostic facts.
- `balanced` is the default and collapses classified chatter while retaining failure
  evidence.
- `aggressive` must be selected explicitly and is never automatic for low- or
  medium-confidence failures.

Verbose flags such as `--stacktrace`, `--full-stacktrace`, `--info`, `--debug`, and
`--scan` select raw/lossless behavior.

## Raw recovery

Every optimized run stores `metadata.json`, `diagnostics.json`, `summary.txt`,
`stdout.log`, and `stderr.log`. Compact output includes a run ID.

```text
contextdroid show <RUN_ID>
contextdroid show <RUN_ID> --errors
contextdroid show <RUN_ID> --warnings
contextdroid show <RUN_ID> --causes
contextdroid show <RUN_ID> --json
contextdroid show <RUN_ID> --raw
contextdroid runs prune
```

Stdout and stderr are stored separately. Raw replay labels streams because their original
cross-stream interleaving cannot be reconstructed reliably after process completion.

## Analytics

Analytics are local-only in `analytics.db`; ContextDroid contains no remote telemetry
client or consent flow.

```text
contextdroid gain
contextdroid gain --scope android
contextdroid gain --command gradle --project . --since 7d
contextdroid gain --format json
contextdroid quality
contextdroid quality --scope android --format json
contextdroid session
contextdroid discover
```

Token figures are estimates. Direct command-output reduction and effective reduction after
raw recoveries are reported separately; neither is a claim about complete model-session
billing.

## Agent integrations

Preview before installing:

```text
contextdroid integrations claude preview
contextdroid integrations cursor preview --cursor-schema-version 1
contextdroid integrations codex preview --root .
```

Replace `preview` with `install`, `status`, or `uninstall`. Claude uses tested
`PreToolUse` input replacement. Cursor is limited to verified hooks schema version 1.
Codex receives a bounded managed `AGENTS.md` instruction block; ContextDroid does not
claim transparent Codex command interception. Integrations preserve unrelated settings
and are idempotent.

## RTK migration

There is no `rtk` binary alias. Migration is explicit and dry-run by default:

```text
contextdroid migrate rtk --dry-run
contextdroid migrate rtk --apply
```

Only safe preferences and separately labeled legacy local analytics are imported. Hooks,
trust state, telemetry state, and database path overrides are never imported.

## Safe defaults and exclusions

ContextDroid never automatically rewrites broad `grep`, `rg`, `find`, `tree`, file reads,
full `git diff`, `curl`, `wget`, security scans, machine-consumed JSON/XML/CSV/protobuf,
pipelines, redirected commands, or binary output. Unknown or malformed output is raw.

## Benchmarks

ContextDroid does not reuse RTK percentage claims. The alpha corpus measures estimated raw
and returned tokens, preservation, confidence, fallback, recovery/rerun behavior, latency,
and memory. Current results and the methodology are in
[docs/BENCHMARKS.md](docs/BENCHMARKS.md). Correctness gates block release; compression
percentage does not.

## Limitations

- Android diagnostic formats vary across Gradle, AGP, Kotlin, devices, and OEMs.
- The alpha parser corpus is synthetic and must expand with redistributable real-world
  samples.
- Durable optimized output is returned after raw capture completes; live transformed
  streaming remains future work.
- Windows is the current local verification platform. Unix CI is required before release.
- Packaging and translated READMEs still contain inherited material until the final naming
  and release-readiness sweep is complete.

## Troubleshooting and uninstall

Use `contextdroid show <RUN_ID> --raw` whenever a summary appears incomplete. Select
`android-only` to disable general inherited automatic coverage, or invoke the original
command directly. Remove agent integration state with `contextdroid integrations <agent>
uninstall`. Delete the binary and the platform ContextDroid data directory only after
retaining any raw runs you need.

## Contributing

Read [AGENTS.md](AGENTS.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), and
[docs/SAFETY_CONTRACT.md](docs/SAFETY_CONTRACT.md). Repository builds, tests, diffs, logs,
and diagnostics must be run raw—never through RTK or ContextDroid. Every parser/rewrite
change needs raw fixtures, semantic assertions, malformed/unknown cases, exit parity,
recovery, and positive/negative rewrite tests.

## License

Apache License 2.0. Existing upstream copyright and license notices are preserved.
