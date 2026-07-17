# Post-alpha Android adoption pilot

After the public alpha is available, the adoption pilot should contain 3–5 Android developers
across at least two project shapes. It is opt-in, time-bounded, and starts with `android-only` or
`contextdroid-safe`; aggressive mode and global agent configuration are out of scope. Pilot results
gate the next alpha or stable release, not installation of `v0.1.0-alpha.1`.

## Entry criteria

- The exact candidate commit passes cross-platform CI and packaging dry run.
- Participants have permission to use the selected projects and logs.
- Raw-output retention, sensitive-data handling, and uninstall are understood.
- A maintainer owns incident response and rollback.

## Procedure

1. Record raw baselines for agreed Gradle, test, ADB, and Logcat workloads.
2. Preview and explicitly install only the selected agent integration.
3. Record exit/signal parity, required evidence, fallback, recovery/rerun, latency, and estimated
   raw/returned tokens.
4. Stop immediately for missing evidence, altered exit status, unsafe routing, or unexpected
   configuration changes.
5. Uninstall the integration, purge or retain local runs per policy, and confirm rollback.

## Success and stop criteria

No correctness or security failure is acceptable. Compression is informational. A high fallback
rate is an alpha finding, not a reason to hide raw evidence. Any incident is documented with
redacted reproduction data and blocks expansion until resolved.
