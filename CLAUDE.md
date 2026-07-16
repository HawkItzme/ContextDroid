# CLAUDE.md — ContextDroid repository development

ContextDroid is an independently maintained Android-focused downstream product derived from
RTK v0.43.0. Read `AGENTS.md`, `.agent/EXEC_PLAN.md`, `docs/ARCHITECTURE.md`, and
`docs/SAFETY_CONTRACT.md` before broad changes.

Never use RTK, ContextDroid, or an installed compression hook for this repository's own build,
test, lint, diff, log, or security output. Run these release-blocking commands directly and keep
their output raw:

```bash
cargo fmt --all --check
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

The default `contextdroid-safe` profile rewrites only verified Android commands and narrow
human-readable Git status/log commands. Pipelines, redirects, structured or binary output,
security tools, broad discovery, and unknown commands must pass through unchanged.

Implementation work follows Red-Green-Refactor. Every parser/rewrite change needs semantic
assertions, malformed/unknown fallback, exit parity, raw recovery, and positive/negative routing
tests. Preserve exact failure evidence or return raw output.

Do not modify global Claude/Codex/Cursor/Git/shell settings, publish, tag, merge `develop`, or use
destructive Git operations without explicit user approval. Integration tests use temporary roots.
