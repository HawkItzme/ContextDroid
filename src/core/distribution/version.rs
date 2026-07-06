//! Version parsing and comparison for self-update.
//!
//! Compares versions without a semver dependency: release tags (`v0.42.4`)
//! and pre-releases (`dev-0.44.0-rc.3`). Only the numeric dotted core is
//! compared; any suffix is ignored.

pub fn parse(version: &str) -> Vec<u64> {
    let start = match version.find(|c: char| c.is_ascii_digit()) {
        Some(i) => i,
        None => return Vec::new(),
    };
    version[start..]
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()
        .unwrap_or("")
        .split('.')
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

pub fn is_newer(remote: &str, local: &str) -> bool {
    let r = parse(remote);
    let l = parse(local);
    if r.is_empty() || l.is_empty() {
        // Unparseable versions must never trigger a self-replace.
        return false;
    }
    r > l
}

/// Plain `X.Y.Z` form as used in download URLs.
pub fn numeric(version: &str) -> String {
    parse(version)
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_forms() {
        assert_eq!(parse("0.42.4"), vec![0, 42, 4]);
        assert_eq!(parse("v0.42.4"), vec![0, 42, 4]);
        assert_eq!(parse("1.2.3-rc.4"), vec![1, 2, 3]);
        assert_eq!(parse("dev-0.44.0-rc.309"), vec![0, 44, 0]);
        assert!(parse("not-a-version").is_empty());
        assert!(parse("").is_empty());
    }

    #[test]
    fn test_is_newer() {
        assert!(is_newer("0.43.0", "0.42.4"));
        assert!(!is_newer("0.42.4", "0.43.0"));
        assert!(!is_newer("0.42.4", "0.42.4"));
        assert!(is_newer("0.100.0", "0.99.9"));
    }

    #[test]
    fn test_is_newer_ignores_suffix() {
        assert!(is_newer("0.42.4", "0.40.0-rc.1"));
        assert!(!is_newer("0.40.0-rc.1", "0.42.4"));
    }

    #[test]
    fn test_is_newer_unparseable_never_updates() {
        assert!(!is_newer("garbage", "0.42.4"));
        assert!(!is_newer("0.43.0", "garbage"));
    }

    #[test]
    fn test_numeric() {
        assert_eq!(numeric("v0.43.0"), "0.43.0");
        assert_eq!(numeric("1.2.3-rc.4"), "1.2.3");
    }
}
