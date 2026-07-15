# ContextDroid Product Specification

ContextDroid is an Android-focused downstream product derived from RTK v0.43.0. The alpha
supports conservative Gradle diagnostics, selected text ADB commands, structured Logcat
failure extraction, durable raw recovery, local analytics, and scoped agent integrations.

The default user is an AI coding agent working on Android projects. The product optimizes
human-readable diagnostic output, not arbitrary data transport. Correctness, exit parity,
raw recovery, and transparent omissions take priority over compression.

Required profiles are `contextdroid-safe`, `android-only`, and opt-in `rtk-compatible`.
Required modes are `lossless`, `balanced`, and explicit `aggressive`. Unsupported,
low-confidence, structured, redirected, piped, binary, and unknown operations remain raw.

There is no RTK compatibility alias. Migration is explicit. ContextDroid is independently
maintained and must not imply upstream affiliation or reuse upstream benchmark claims.
