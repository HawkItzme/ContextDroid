# ContextDroid Security Policy

## Reporting a vulnerability

Use GitHub's private vulnerability reporting for
[HawkItzme/ContextDroid](https://github.com/HawkItzme/ContextDroid/security/advisories/new).
Do not include exploit details, raw logs, tokens, private paths, or device identifiers in a
public issue.

If private reporting is unavailable, open a public issue containing only a request for private
contact. ContextDroid does not currently publish a dedicated security mailbox.

We aim to acknowledge a report within 7 calendar days and provide an initial assessment within
14 days. These are response targets, not guarantees. Disclosure timing is coordinated with the
reporter after a fix or mitigation is available.

## Supported versions

Before the first public alpha, only the current head of the active product branch is maintained.
After `v0.1.0-alpha.1`, the latest published prerelease is supported on a best-effort basis.
Older prereleases may be asked to upgrade before investigation. This policy will be revised when
a stable release line exists.

## Security model

ContextDroid launches developer commands, captures stdout and stderr, writes local recovery
artifacts, and can install bounded agent-integration configuration after an explicit command.
High-risk areas include:

- `src/core/runner.rs`, `src/core/stream.rs`, and command construction;
- `src/core/run_store.rs`, `src/core/secure_fs.rs`, and local raw-output retention;
- `src/hooks/`, `src/integrations.rs`, and `src/migration.rs`;
- installers, release workflows, package metadata, and checksums;
- Android parsers handling malformed or attacker-controlled build and Logcat text.

Safe profiles reject pipelines, redirects, substitutions, binary streams, structured output,
security scanners, and unknown commands from automatic rewriting. Parser failure and low
confidence return raw output. Raw recovery logs may contain secrets or personal information from
the original command; protect the local data directory and redact logs before sharing them.

## Required review

Security-sensitive changes require tests for command injection, shell quoting, substitutions,
redirects, traversal, symlink/reparse handling, malformed input, permission preservation,
unrelated-settings preservation, and failure fallback. CI security policy is defined by the
checked-in workflows; this document does not claim checks that are absent there.

## Disclosure and advisories

Validated vulnerabilities are fixed on a private branch when practical, assigned severity,
tested on supported platforms, and disclosed through a GitHub security advisory with affected
versions and remediation. Credit is provided when requested and safe to do so.

Upstream RTK vulnerabilities are evaluated for applicability to the pinned downstream code.
ContextDroid is independently maintained and is not affiliated with or endorsed by RTK or its
maintainers.
