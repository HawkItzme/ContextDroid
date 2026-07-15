# Filter Matrix

| Family | Explicit command | Safe auto-rewrite | Android-only | Notes |
|---|---:|---:|---:|---|
| Verified Gradle Android tasks | Yes | Yes | Yes | Mixed/unknown tasks conservative |
| ADB devices/install/uninstall | Yes | Yes | Yes | Text only |
| ADB shell am/pm/selected dumpsys | Yes | Yes | Yes | Unknown subcommands raw |
| Logcat text modes | Yes | Yes | Yes | Package/PID/time filters supported |
| Git status/log human output | Inherited | Narrow forms | No | No structured/full output |
| Other inherited RTK families | Explicit | No | No | Automatic only in rtk-compatible |
| grep/rg/find/tree/broad reads | Inherited | No | No | Raw by default |
| full git diff | Inherited | No | No | Raw by default |
| curl/wget/security scanners | Inherited | No | No | Raw by default |
| JSON/XML/CSV/protobuf | Inherited | No | No | Machine output raw |
| pipelines/redirects/substitution | N/A | No | No | Universal hard stop |
| binary/protocol output | N/A | No | No | Universal hard stop |
| unknown commands/output | N/A | No | No | Raw fallback |
