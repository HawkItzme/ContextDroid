# Copilot instructions for ContextDroid

ContextDroid is an independently maintained Android-focused downstream product derived from RTK.
When developing this repository, run Cargo, Git, search, diff, lint, build, test, and diagnostic
commands raw. Never route this repository's own output through RTK or ContextDroid.

The default `contextdroid-safe` profile rewrites only verified Android commands and explicitly
approved human-readable Git forms. It must not rewrite pipelines, redirects, structured output,
security tools, binary output, broad discovery commands, or unknown commands. Preserve complete
stdout/stderr before transformation, exact exit status, required diagnostic evidence, omission
counts, and a `contextdroid show <RUN_ID> --raw` recovery path.

Follow [AGENTS.md](../AGENTS.md) and keep [.agent/EXEC_PLAN.md](../.agent/EXEC_PLAN.md) current.
