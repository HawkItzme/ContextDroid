# Safety and Diagnostic Correctness Contract

For every optimized command, ContextDroid saves untouched stdout and stderr before lossy
transformation and preserves the original exit code or signal. A failed parse, storage
problem, missing failure evidence, or low confidence returns raw output.

When present, failed Android output must retain the command, working directory, status,
failing task/module/variant, category/severity, exact root error or exception, every
`Caused by` header, source coordinates, dependency/resource/class conflicts, test
expected/actual values, application frames, coroutine causes/suppression, Logcat identity
fields, ANR reason, native reference, run ID, and recovery command.

Compact output lists actual preserved and collapsed counts. Counts must derive from real
transformations. Verbose diagnostic flags remain raw/lossless. Binary and machine-consumed
output is never transformed. Universal hard stops apply even to `rtk-compatible`.

If any invariant cannot be established, the correct result is raw output—not a partial
summary.
