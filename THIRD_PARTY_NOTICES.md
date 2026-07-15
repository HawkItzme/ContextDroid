# Third-party notices

ContextDroid includes and builds on open-source software. This file supplements, and does
not replace, the license terms supplied with each component.

## RTK

ContextDroid is derived from RTK `v0.43.0`, commit
`5a7880d404db8364d602f2ecdc41dd790f64013f`.

- Project: RTK
- Upstream: <https://github.com/rtk-ai/rtk>
- License: Apache License 2.0
- Copyright: retained in the upstream source and Git history

ContextDroid is independently maintained and is not affiliated with or endorsed by RTK
or its maintainers. See [UPSTREAM.md](UPSTREAM.md) for the pinned source and maintenance
policy, and [LICENSE](LICENSE) for the Apache License 2.0 text.

## Rust dependencies

ContextDroid links Rust crates declared in `Cargo.toml` and resolved in `Cargo.lock`.
Those components remain under their respective licenses and copyrights. Release
artifacts must include a generated, lockfile-specific dependency license inventory; the
release checklist treats missing, unknown, or incompatible licenses as a blocker.

No dependency license is superseded by this notice. Source distributions retain the
dependency names and exact versions in `Cargo.lock` so recipients can obtain the
corresponding license texts from each crate's published source package.
