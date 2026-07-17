# ContextDroid

ContextDroid is an independently maintained, Android-focused command-output optimizer
derived from RTK. It extracts useful Android diagnostics for AI coding agents while securely
staging complete stdout and stderr before parsing. Failed optimized runs are retained for exact
recovery; successful staging is deleted by default.

ContextDroid is not affiliated with or endorsed by `rtk-ai`. The upstream provenance and
pinned commit are recorded in [UPSTREAM.md](UPSTREAM.md); third-party notices are in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Why Android needs a conservative tool

Gradle, AGP, Kotlin, Java, KSP, KAPT, AAPT2, manifest merger, D8/R8, test, ADB, and
Logcat output can be extremely verbose. Blind truncation is dangerous because a useful
root cause may be separated from the failing task, source location, or `Caused by` chain.
ContextDroid captures raw output first, extracts typed diagnostics second, and returns raw
output unchanged whenever parser confidence is low.

## Installation and availability

Install the published alpha without Rust or a local build.

Linux and macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/HawkItzme/ContextDroid/main/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/HawkItzme/ContextDroid/main/install.ps1 | iex
```

Both installers select the current alpha, verify `SHA256SUMS`, reject unsafe archives, and
verify the binary version before replacing an existing installation. Pin a release with
`CONTEXTDROID_VERSION`, or download and inspect the installer before running it if preferred.

Agent integration is a separate explicit operation:

```text
contextdroid integrations claude preview
contextdroid integrations cursor preview --cursor-schema-version 1
contextdroid integrations codex preview --root .

contextdroid integrations claude install
contextdroid integrations cursor install --cursor-schema-version 1
contextdroid integrations codex install --root .
```

Use the corresponding `status` or `uninstall` action to inspect or remove only the managed
ContextDroid entry. Installation modifies agent or project configuration; preview and status
do not write.

Direct archives, DEB, and RPM packages are also attached to the GitHub prerelease. Homebrew is
deferred. See [INSTALL.md](INSTALL.md) for pinned, manual, rollback, and source-build options.

## Direct usage

```text
contextdroid gradlew assembleDebug
contextdroid gradlew testDebugUnitTest
contextdroid adb devices
contextdroid adb install app-debug.apk
contextdroid logcat snapshot --mode crash --package com.example.app --since 10m
contextdroid logcat stream --package com.example.app
```

Unsupported commands are not automatically transformed. In safe profiles, call inherited
compatibility commands explicitly only when you understand their output behavior.

## Profiles

- `contextdroid-safe` is the default. It automatically considers verified Android commands
  and narrow human-readable Git status/log forms.
- `android-only` automatically considers only verified Gradle, ADB, and Logcat commands.
- `rtk-compatible` opts into inherited coverage, but cannot bypass universal hard stops.

Select the rewrite profile before the subcommand, for example
`contextdroid --profile android-only rewrite "./gradlew assembleDebug"`. The `gain` and
`quality` subcommands use their own `--profile` execution filter. Select output with `--output-mode`,
`CONTEXTDROID_OUTPUT_MODE`, or `[output].mode` (in that precedence). Pipelines, redirects,
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

Every failed optimized run stores `metadata.json`, `diagnostics.json`, `summary.txt`,
`stdout.log`, and `stderr.log`. Successful raw staging is deleted unless
`CONTEXTDROID_RETAIN_SUCCESSES=1` is explicitly set. Compact failures include a run ID.

```text
contextdroid show <RUN_ID>
contextdroid show <RUN_ID> --errors
contextdroid show <RUN_ID> --warnings
contextdroid show <RUN_ID> --causes
contextdroid show <RUN_ID> --json
contextdroid show <RUN_ID> --raw
contextdroid runs prune
contextdroid runs list
contextdroid runs purge --yes
```

Stdout and stderr are stored separately. Raw replay labels streams because their original
cross-stream interleaving cannot be reconstructed reliably after process completion.

## Analytics

Analytics are local-only in `analytics.db`; ContextDroid contains no remote telemetry
client or consent flow.

```text
contextdroid gain
contextdroid gain --scope android
contextdroid gain --command gradle --project . --profile contextdroid-safe --parser android-gradle --since 2h
contextdroid gain --weekly --last 20 --format json
contextdroid quality
contextdroid quality --scope android --format json
contextdroid session
contextdroid discover
contextdroid analytics export --format csv
contextdroid analytics reset --yes
contextdroid privacy status
```

Token figures are estimates. Direct command-output reduction and effective reduction after
raw recoveries are reported separately; neither is a claim about complete model-session
billing.

## Agent integrations

The integration commands are also listed here for reference. Preview first:

```text
contextdroid integrations claude preview
contextdroid integrations cursor preview --cursor-schema-version 1
contextdroid integrations codex preview --root .
```

Install explicitly after reviewing the preview:

```text
contextdroid integrations claude install
contextdroid integrations cursor install --cursor-schema-version 1
contextdroid integrations codex install --root .
```

Replace `install` with `status` or `uninstall` as needed. Claude Code on Linux is the
supported alpha candidate and uses `PreToolUse` input replacement. Cursor schema version 1
is experimental until cross-platform release-commit smoke is recorded. Codex receives a
bounded managed `AGENTS.md` instruction block; ContextDroid does not claim transparent Codex
command interception. Lifecycle tests require unrelated settings to be preserved and operations
to be idempotent.

## RTK migration

There is no `rtk` binary alias. Migration is explicit and dry-run by default:

```text
contextdroid migrate rtk --dry-run
contextdroid migrate rtk --apply
```

Only safe preferences and compatible local analytics are imported. Trust state, telemetry
state, and database path overrides are never imported. Recognized RTK hooks make ordinary
integration install fail closed; explicit apply backs up and replaces only recognized entries.
See [docs/MIGRATION.md](docs/MIGRATION.md).

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
- Release archives are verified on their native Linux, macOS, and Windows CI runners.
- Android parser coverage combines redistributable fixtures, an Android Gradle smoke project,
  and a pinned public validation project; device and OEM formats will continue to expand.
- Homebrew remains deferred and is not required for direct alpha installation.

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
