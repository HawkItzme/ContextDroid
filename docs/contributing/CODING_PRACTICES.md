# ContextDroid coding practices

Preserve exact failure evidence, original exit status, and raw recovery. Prefer semantic parsing
with fail-closed confidence checks over truncation. New parser or rewrite work requires fixtures,
preservation assertions, malformed/unknown fallbacks, exit-code parity, and negative no-rewrite
tests. Run the raw verification commands in [TESTING.md](TESTING.md) before handoff.
