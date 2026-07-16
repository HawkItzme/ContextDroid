# Filter Matrix

| Family | Explicit command | Safe auto-rewrite | Android-only | Notes |
|---|---:|---:|---:|---|
| Verified Gradle Android tasks | Yes | Yes | Yes | All tasks verified; clean neutral; unknown raw; mixed capped balanced |
| ADB devices/install/install-multiple/uninstall | Yes | Yes | Yes | Text only |
| ADB shell am start/startservice/broadcast/force-stop | Yes | Yes | Yes | Other actions raw |
| ADB shell pm list packages/path/resolve-activity | Yes | Yes | Yes | Other actions raw |
| ADB dumpsys activity/package/meminfo | Yes | Yes | Yes | Requires a narrowing argument |
| Logcat bounded snapshot | Yes | Yes | Yes | Default 10m, 20,000 lines; package/PID/mode filters |
| Logcat stream | Yes | Yes | Yes | Direct pass-through; no semantic buffer or raw retention |
| Git status/log human output | Inherited | Narrow forms | No | No structured/full output |
| Other inherited RTK families | Explicit | No | No | Automatic only in rtk-compatible |
| grep/rg/find/tree/broad reads | Inherited | No | No | Raw by default |
| full git diff | Inherited | No | No | Raw by default |
| curl/wget/security scanners | Inherited | No | No | Raw by default |
| JSON/XML/CSV/protobuf | Inherited | No | No | Machine output raw |
| pipelines/redirects/substitution | N/A | No | No | Universal hard stop |
| binary/protocol output | N/A | No | No | Universal hard stop |
| unknown commands/output | N/A | No | No | Raw fallback |
