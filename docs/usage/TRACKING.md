# Local analytics

ContextDroid records run analytics locally in `analytics.db`. It has no remote telemetry client.

Use `contextdroid gain`, `contextdroid session`, and `contextdroid quality` for local reports.
Direct estimates, effective estimates after raw recovery, confidence, fallback rate, and raw
recovery frequency are intentionally distinguished. See [Benchmarks](../BENCHMARKS.md) for the
measurement contract.
