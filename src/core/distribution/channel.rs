//! Release channels: where each edition gets its binaries.
//!
//! `Github` reads the latest version from the `releases/latest` 302
//! redirect (no API rate limit) and checksums from `checksums.txt`.
//! `Blob` reads `latest.json` on a public blob, whose checksums are nested
//! under `"sha256s"` (deployed shape; a flat map is tolerated).
//!
//! Env overrides: `RTK_BLOB_BASE`, `RTK_OSS_BASE` (also how tests point at
//! a fixture server), `RTK_VERSION` (pin an exact version).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};

use super::platform::Target;

pub const DEFAULT_BLOB_BASE: &str = "https://releases.example.com";
pub const DEFAULT_OSS_BASE: &str = "https://github.com/rtk-ai/rtk";

const FETCH_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct Release {
    pub version: String,
    pub sha256: Option<String>,
    pub tarball_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Github,
    Blob,
}

impl Channel {
    pub fn latest(&self, target: &Target) -> Result<Release> {
        let pinned = std::env::var("RTK_VERSION").ok();
        match self {
            Channel::Blob => {
                let base = std::env::var("RTK_BLOB_BASE")
                    .unwrap_or_else(|_| DEFAULT_BLOB_BASE.to_string());
                blob_latest(&base, pinned.as_deref(), target.blob)
            }
            Channel::Github => {
                let base =
                    std::env::var("RTK_OSS_BASE").unwrap_or_else(|_| DEFAULT_OSS_BASE.to_string());
                github_latest(&base, pinned.as_deref(), target.triple)
            }
        }
    }

    pub fn describe(&self) -> &'static str {
        match self {
            Channel::Github => "GitHub releases",
            Channel::Blob => "release blob",
        }
    }
}

fn blob_latest(base: &str, pinned: Option<&str>, blob_target: &str) -> Result<Release> {
    let url = format!("{}/latest.json?{}", base, cache_bust());
    let body = http_get_string(&url)
        .with_context(|| format!("Failed to fetch release index ({}/latest.json)", base))?;
    let (latest_version, sha_for_target) = parse_latest_json(&body, blob_target)?;

    let version = match pinned {
        Some(pin) => super::version::numeric(pin),
        None => latest_version.clone(),
    };
    // Checksums are only published for the latest version; a pinned older
    // version downloads without verification (caller warns).
    let sha256 = if version == latest_version {
        sha_for_target
    } else {
        None
    };

    let tarball_url = format!(
        "{}/v{}/rtk-plus-{}.tar.gz?{}",
        base,
        version,
        blob_target,
        cache_bust()
    );
    Ok(Release {
        version,
        sha256,
        tarball_url,
    })
}

fn parse_latest_json(body: &str, blob_target: &str) -> Result<(String, Option<String>)> {
    let json: serde_json::Value =
        serde_json::from_str(body).context("Invalid JSON in latest.json")?;
    let version = json
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("latest.json has no \"version\" field"))?
        .to_string();
    let sha = json
        .get("sha256s")
        .and_then(|m| m.get(blob_target))
        .or_else(|| json.get(blob_target))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    Ok((version, sha))
}

fn github_latest(base: &str, pinned: Option<&str>, triple: &str) -> Result<Release> {
    let tag = match pinned {
        Some(pin) => format!("v{}", super::version::numeric(pin)),
        None => {
            let url = format!("{}/releases/latest", base);
            resolve_latest_tag(&url)
                .with_context(|| format!("Failed to resolve latest OSS release ({})", url))?
        }
    };
    let version = super::version::numeric(&tag);
    if version.is_empty() {
        bail!("Could not parse a version from OSS release tag '{}'", tag);
    }

    let tarball_name = format!("rtk-{}.tar.gz", triple);
    let checksums_url = format!("{}/releases/download/{}/checksums.txt", base, tag);
    let sha256 = match http_get_string(&checksums_url) {
        Ok(body) => parse_checksums(&body, &tarball_name),
        Err(e) => {
            eprintln!(
                "rtk: warning: could not fetch checksums.txt ({:#}) — proceeding without verification",
                e
            );
            None
        }
    };

    let tarball_url = format!("{}/releases/download/{}/{}", base, tag, tarball_name);
    Ok(Release {
        version,
        sha256,
        tarball_url,
    })
}

/// GitHub answers `releases/latest` with a 302 whose Location ends in
/// `/tag/<tag>`; fixture servers may answer 200 with the tag as body.
fn resolve_latest_tag(url: &str) -> Result<String> {
    let agent = ureq::builder().redirects(0).build();
    let response = match agent.get(url).timeout(FETCH_TIMEOUT).call() {
        Ok(resp) => resp,
        Err(ureq::Error::Status(_, resp)) => resp,
        Err(e) => return Err(e).context("Request failed"),
    };

    if (300..400).contains(&response.status()) {
        let location = response
            .header("location")
            .ok_or_else(|| anyhow!("Redirect without Location header"))?;
        return tag_from_location(location)
            .ok_or_else(|| anyhow!("Could not parse tag from redirect '{}'", location));
    }
    if response.status() == 200 {
        let tag = response
            .into_string()
            .context("Failed to read response body")?
            .trim()
            .to_string();
        if tag.is_empty() {
            bail!("Empty response for latest release tag");
        }
        return Ok(tag);
    }
    bail!("Unexpected HTTP status {}", response.status());
}

fn tag_from_location(location: &str) -> Option<String> {
    let tag = location.rsplit("/tag/").next()?;
    let tag = tag.split(['?', '#']).next().unwrap_or(tag).trim();
    if tag.is_empty() || tag == location {
        None
    } else {
        Some(tag.to_string())
    }
}

fn parse_checksums(body: &str, file_name: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?;
        // sha256sum prefixes binary-mode entries with '*'.
        if name.trim_start_matches('*') == file_name && hash.len() == 64 {
            Some(hash.to_string())
        } else {
            None
        }
    })
}

fn http_get_string(url: &str) -> Result<String> {
    let response = ureq::get(url)
        .set("Cache-Control", "no-cache")
        .set("User-Agent", "rtk-distribution")
        .timeout(FETCH_TIMEOUT)
        .call()
        .with_context(|| format!("GET {} failed", url))?;
    response.into_string().context("Failed to read response")
}

/// Corporate proxies cache blob responses aggressively — bust them.
fn cache_bust() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("nc={}", ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NESTED: &str = r#"{
        "version": "0.42.4",
        "sha256s": {
            "linux-amd64": "56a49da54b9a4f3f75779656d40f1f53647052becb70415143158c8672895634",
            "linux-arm64": "17ddd8217957d1423463d754f406b07b244c690435f3c2d88a6df9c5451cdf82"
        }
    }"#;

    #[test]
    fn test_parse_latest_json_nested_deployed_shape() {
        let (version, sha) = parse_latest_json(NESTED, "linux-amd64").unwrap();
        assert_eq!(version, "0.42.4");
        assert_eq!(
            sha.as_deref(),
            Some("56a49da54b9a4f3f75779656d40f1f53647052becb70415143158c8672895634")
        );
    }

    #[test]
    fn test_parse_latest_json_missing_target() {
        let (version, sha) = parse_latest_json(NESTED, "darwin-arm64").unwrap();
        assert_eq!(version, "0.42.4");
        assert!(sha.is_none());
    }

    #[test]
    fn test_parse_latest_json_flat_fallback_shape() {
        let flat = r#"{"version": "1.2.3", "linux-amd64": "abc123"}"#;
        let (version, sha) = parse_latest_json(flat, "linux-amd64").unwrap();
        assert_eq!(version, "1.2.3");
        assert_eq!(sha.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_parse_latest_json_errors() {
        assert!(parse_latest_json(r#"{"sha256s": {}}"#, "linux-amd64").is_err());
        assert!(parse_latest_json("<html>404</html>", "linux-amd64").is_err());
    }

    #[test]
    fn test_tag_from_location() {
        assert_eq!(
            tag_from_location("https://github.com/rtk-ai/rtk/releases/tag/v0.43.0").as_deref(),
            Some("v0.43.0")
        );
        assert_eq!(
            tag_from_location("/rtk-ai/rtk/releases/tag/v1.0.0?foo=1").as_deref(),
            Some("v1.0.0")
        );
        assert!(tag_from_location("https://github.com/").is_none());
    }

    #[test]
    fn test_parse_checksums() {
        let body = "abc\n\
            56a49da54b9a4f3f75779656d40f1f53647052becb70415143158c8672895634  rtk-x86_64-unknown-linux-musl.tar.gz\n\
            17ddd8217957d1423463d754f406b07b244c690435f3c2d88a6df9c5451cdf82 *rtk-aarch64-unknown-linux-gnu.tar.gz\n";
        assert_eq!(
            parse_checksums(body, "rtk-x86_64-unknown-linux-musl.tar.gz").as_deref(),
            Some("56a49da54b9a4f3f75779656d40f1f53647052becb70415143158c8672895634")
        );
        assert_eq!(
            parse_checksums(body, "rtk-aarch64-unknown-linux-gnu.tar.gz").as_deref(),
            Some("17ddd8217957d1423463d754f406b07b244c690435f3c2d88a6df9c5451cdf82")
        );
        assert!(parse_checksums(body, "rtk-missing.tar.gz").is_none());
    }

    #[test]
    fn test_blob_latest_via_fixture_server() {
        let base = super::super::test_http::serve(vec![(
            "/latest.json".to_string(),
            200,
            NESTED.as_bytes().to_vec(),
        )]);
        let release = blob_latest(&base, None, "linux-amd64").unwrap();
        assert_eq!(release.version, "0.42.4");
        assert!(release.sha256.is_some());
        assert!(release
            .tarball_url
            .starts_with(&format!("{}/v0.42.4/rtk-plus-linux-amd64.tar.gz", base)));
    }

    #[test]
    fn test_blob_latest_pinned_older_version_no_checksum() {
        let base = super::super::test_http::serve(vec![(
            "/latest.json".to_string(),
            200,
            NESTED.as_bytes().to_vec(),
        )]);
        let release = blob_latest(&base, Some("v0.40.0"), "linux-amd64").unwrap();
        assert_eq!(release.version, "0.40.0");
        assert!(release.sha256.is_none());
    }

    #[test]
    fn test_github_latest_via_fixture_200_body() {
        let checksums =
            "aaaa000000000000000000000000000000000000000000000000000000000000  rtk-x86_64-unknown-linux-musl.tar.gz\n";
        let base = super::super::test_http::serve(vec![
            ("/releases/latest".to_string(), 200, b"v9.9.9".to_vec()),
            (
                "/releases/download/v9.9.9/checksums.txt".to_string(),
                200,
                checksums.as_bytes().to_vec(),
            ),
        ]);
        let release = github_latest(&base, None, "x86_64-unknown-linux-musl").unwrap();
        assert_eq!(release.version, "9.9.9");
        assert_eq!(
            release.sha256.as_deref(),
            Some("aaaa000000000000000000000000000000000000000000000000000000000000")
        );
        assert_eq!(
            release.tarball_url,
            format!(
                "{}/releases/download/v9.9.9/rtk-x86_64-unknown-linux-musl.tar.gz",
                base
            )
        );
    }

    #[test]
    fn test_github_latest_via_fixture_302_redirect() {
        let base = super::super::test_http::serve_with_headers(vec![(
            "/releases/latest".to_string(),
            302,
            vec![(
                "Location".to_string(),
                "https://example.com/rtk-ai/rtk/releases/tag/v1.2.3".to_string(),
            )],
            Vec::new(),
        )]);
        let release = github_latest(&base, None, "x86_64-unknown-linux-musl").unwrap();
        assert_eq!(release.version, "1.2.3");
        assert!(release.sha256.is_none());
    }
}
