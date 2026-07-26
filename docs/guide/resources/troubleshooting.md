# Troubleshooting

- Use `contextdroid --version` to confirm the product binary.
- Use `contextdroid setup status` (optionally with repeatable `--only` flags) to inspect integrations.
- Use `contextdroid show <RUN_ID> --raw` to recover untouched output.
- Unknown, structured, binary, piped, redirected, security-sensitive, or low-confidence output
  should remain raw; this is expected safe-profile behavior.

See the [README](../../../README.md) for current limitations.
