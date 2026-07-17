# Release artifacts

`release/targets.json` is the canonical target-to-asset contract. CI tests this table against the
release workflow, installer, and this document.

| Platform | Rust target | Asset | Installation |
| --- | --- | --- | --- |
| Linux x86_64 | `x86_64-unknown-linux-musl` | `contextdroid-x86_64-unknown-linux-musl.tar.gz` | installer or manual |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | `contextdroid-aarch64-unknown-linux-gnu.tar.gz` | installer or manual |
| macOS Intel | `x86_64-apple-darwin` | `contextdroid-x86_64-apple-darwin.tar.gz` | installer or manual |
| macOS Apple Silicon | `aarch64-apple-darwin` | `contextdroid-aarch64-apple-darwin.tar.gz` | installer or manual |
| Windows x86_64 | `x86_64-pc-windows-msvc` | `contextdroid-x86_64-pc-windows-msvc.zip` | PowerShell installer or manual |

Every archive contains one binary named `contextdroid` or `contextdroid.exe` plus `LICENSE`,
`UPSTREAM.md`, and `THIRD_PARTY_NOTICES.md`. Published release assets include `SHA256SUMS`, the
canonical target manifest, release manifest, installers, notices, a CycloneDX JSON SBOM, and
GitHub build provenance attestation.
DEB and RPM packages are additional Linux convenience artifacts and do not replace the target
archives.
