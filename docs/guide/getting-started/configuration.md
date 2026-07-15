# Configuration

The default profile is `contextdroid-safe`; `android-only` and the opt-in aggressive
`rtk-compatible` profile are also available. `balanced` is the default output mode, while
`aggressive` must be selected explicitly.

ContextDroid configuration, cache, data, analytics, and audit paths use the `contextdroid`
product namespace. Legacy RTK settings are imported only by explicit `contextdroid migrate rtk`
and hooks, trust records, telemetry settings, and custom database paths are not copied.
