# Safety and Diagnostic Correctness Contract

For every optimized command, ContextDroid securely stages untouched stdout and stderr before
lossy transformation and preserves the original exit code or signal. Successful staging is
deleted by default; failed staging is retained for recovery. A failed parse, storage
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

Semantic output must also be smaller than faithful raw output. Otherwise ContextDroid replays
raw and records a never-worse fallback. Analytics never stores full commands, arguments, paths,
package names, device serials, error text, or Logcat contents; project/session selectors use
local pseudonymous identifiers. No remote analytics client or endpoint exists.
