# Technical contribution guide

The current technical source of truth is [Architecture](../ARCHITECTURE.md), together with the
[Safety contract](../SAFETY_CONTRACT.md) and [Filter matrix](../FILTER_MATRIX.md).

Repository builds, tests, diffs, logs, lint, and diagnostics must be run raw. Every parser change
requires semantic preservation assertions, malformed or unknown-output coverage, exit-code parity,
and raw recovery coverage.
