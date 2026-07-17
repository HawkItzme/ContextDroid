# Contributing to ContextDroid

ContextDroid is an independently maintained Android-focused downstream of RTK. Contributions
are welcome when they strengthen Android diagnostic correctness, conservative command routing,
raw-output recovery, or compatible inherited functionality.

## Contribution terms

ContextDroid uses an Apache-2.0 inbound-equals-outbound policy. By submitting a contribution,
you agree that it is provided under the repository's [Apache License 2.0](LICENSE). You must have
the right to submit the work. ContextDroid has no CLA and does not require a DCO sign-off.

Do not submit proprietary build logs, secrets, personal data, non-redistributable fixtures, or
code copied from sources with incompatible licenses.

## Before opening a pull request

1. Open an issue for changes that alter public behavior, safety policy, storage, integrations,
   or release packaging.
2. Branch from `develop` after the ContextDroid branch migration is complete. During alpha
   readiness, use the product branch named in [AGENTS.md](AGENTS.md).
3. Keep the change focused and use Conventional Commit subjects.
4. Add tests before implementation for parser, classifier, rewrite, or bug-fix work.
5. Update `.agent/EXEC_PLAN.md` for architectural decisions or release-gate work.

Pull requests must target `develop`. Promotion to `main` and release tagging are maintainer-only
operations performed from an exact reviewed commit.

## Correctness and safety rules

- Preserve untouched stdout and stderr before any lossy transformation.
- Preserve the child exit code or signal.
- Unknown or incomplete diagnostics must fall back to raw output.
- Keep exact tasks, paths, identifiers, messages, coordinates, locations, causes, assertion
  values, application frames, and Android incident identity.
- Do not automatically rewrite pipelines, redirects, structured output, binary output, security
  tools, broad discovery commands, or unknown commands in safe profiles.
- Omission counts must describe transformations that actually occurred.
- Never use ContextDroid or RTK to compress this repository's own build, test, lint, diff,
  security, or diagnostic output.

See [docs/SAFETY_CONTRACT.md](docs/SAFETY_CONTRACT.md),
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), and
[docs/FILTER_MATRIX.md](docs/FILTER_MATRIX.md).

## Test expectations

Parser and rewrite changes require representative redistributable fixtures, structured evidence
assertions, malformed/unknown cases, positive and negative routing tests, raw-recovery tests, and
exit/signal parity coverage. Snapshots may supplement these checks but cannot replace them.

Run the release gates with raw output:

```text
cargo fmt --all --check
cargo test --all --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo build --release --locked
bash scripts/validate-docs.sh
bash scripts/check-stale-brand.sh
```

Security-sensitive changes must also cover shell metacharacters, redirects, substitutions,
malformed input, path traversal, symlinks/reparse points, and fail-closed behavior.

## Documentation and benchmarks

Document only implemented and tested behavior. Label token counts as estimates unless an exact
tokenizer is used. Keep direct command-output reduction separate from effective reduction after
recoveries or reruns. Do not reuse upstream RTK percentage claims as ContextDroid results.

## Reviews and releases

Maintainers review correctness, safety, licensing, compatibility, and documentation. No
contributor or automation should publish packages, create tags, upload release assets, change
repository settings, or modify user-global agent configuration as part of an ordinary pull
request.

Report vulnerabilities through the private process in [SECURITY.md](SECURITY.md), not a public
issue.
