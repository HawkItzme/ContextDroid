# Permissioned internal Android pilot record

Copy this template for the release-commit pilot. Do not commit proprietary names, paths, source,
raw logs, device IDs, package names, credentials, or personal data.

- Release commit:
- Validation date and operator:
- Project description (redacted):
- Permission to use results confirmed:
- OS / JDK / Gradle / AGP / Kotlin / SDK / build-tools:
- ContextDroid profile and output mode:
- Application/source prefixes configured:

| Workload | Raw exit/signal | ContextDroid exit/signal | Required evidence preserved | Raw recovery verified | Raw/returned estimated tokens | Latency/overhead | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| assemble/build success | | | | | | | |
| Kotlin or Java compiler failure | | | | | | | |
| resource or Manifest failure | | | | | | | |
| failing unit test | | | | | | | |
| device/application crash | | | | | | | |
| ANR or approved redistributable fixture | | | | | | | |
| unknown/malformed output | | | | | | | |
| verbose/lossless flags | | | | | | | |

Record every raw recovery or rerun. Calculate direct reduction separately from effective
reduction after recoveries/reruns. A correctness failure blocks the release regardless of the
compression result.
