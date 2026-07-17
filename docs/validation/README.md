# Android validation evidence

Release validation has three evidence levels:

1. the checked-in synthetic fixture contract, which proves typed preservation and raw recovery;
2. the pinned Apache-2.0 `android/architecture-samples` workload, which exercises a real Gradle,
   AGP, Kotlin, and Android project;
3. one permissioned internal Android project pilot, recorded with the redacted template in this
   directory.

The public workload is pinned to commit
`ee66e1526b84c026615df032c705842b7d2a521f`. CI injects a temporary unresolved Kotlin symbol,
runs the original Gradle command and ContextDroid separately, and verifies the same nonzero exit
code and exact root identifier. It records raw/returned bytes, estimated tokens, lines, latency,
and recovery artifacts. The injected file is removed after the run.

Required alpha matrix:

| Workload | Evidence source | Required result |
| --- | --- | --- |
| Gradle success | checked-in smoke + public sample | exit 0, no false failure |
| Kotlin failure | public sample + fixture | task/location/root preserved; exit parity |
| Java/KSP/KAPT/Compose/resources/Manifest/D8/R8/lint | fixture contract | typed fields or raw fallback |
| Unit/instrumentation test failure | fixture + internal pilot | test identity and values when present |
| Java/Kotlin/coroutine crash | redistributable Logcat fixtures + internal pilot | incident identity, causes, app frames |
| ANR/StrictMode/Binder/native | redistributable fixtures | reason/reference and Logcat identity |
| Unknown/malformed | fixture contract | byte-identical raw fallback |
| Verbose flags | fixture and routing tests | lossless/raw behavior |

The local Windows attempt on 2026-07-17 was inconclusive: the public sample dependency/bootstrap
run produced no output before it was stopped, the installed `sdkmanager` is legacy and fails under
JDK 23, and no JDK 17 is installed. This is not recorded as a pass. The clean CI job uses JDK 17
and a configured Android SDK. Public release remains blocked until that job and the internal pilot
record are green on the exact release commit.
