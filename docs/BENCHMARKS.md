# ContextDroid Benchmarks

## Method

The ContextDroid corpus is synthetic or redistributable and contains no secrets, personal
data, or proprietary identifiers. Each case records raw/returned bytes and lines, estimated
tokens (ceil bytes/4), parser confidence, fallback, critical-field preservation, raw
recovery/rerun behavior, duration, and memory where the runner supports it.

Direct estimated reduction is `raw - returned`. Effective estimated reduction also counts
raw recovery output and detectable reruns. Neither metric represents complete model-session
billing. RTK benchmark percentages are not copied or used as ContextDroid claims.

## Current alpha evidence

The data-driven `contract.json` corpus currently covers 30 required fixture families through
parser, semantic assertions, renderer, durable store, exit status, and exact raw recovery.
Focused tests also cover raw-first persistence, exact stream recovery,
low-confidence fallback, omissions, exit/signal representation, safe no-rewrite behavior,
and local analytics. The fixture corpus is still being expanded and no representative
compression percentage is published yet.

Correctness gates are release blockers. Reduction percentage is not.
