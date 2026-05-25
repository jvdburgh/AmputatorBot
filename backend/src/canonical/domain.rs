//! Domain extraction — given a URL, return just the registrable name
//! (e.g. `"electrek"` from `"https://www.electrek.co.uk/article"`).
//!
//! Uses the [`psl`] crate, which bundles Mozilla's Public Suffix List into
//! the binary. The PSL is the only reliable way to know where a domain ends
//! and a TLD begins (TLDs aren't always one segment: `.co.uk`, `.com.br`,
//! `.github.io` etc.).
//!
//! Ports `archive/helpers/canonical_methods.py:113` —
//! `tldextract.extract(url).domain`.

use url::Url;

/// Extract the second-level domain ("registrable name") from a URL.
///
/// Examples:
/// - `https://www.electrek.co/article`      → `Some("electrek")`
/// - `https://www.google.co.uk/search?q=x`  → `Some("google")`
/// - `https://amp.scmp.com/asia/article-x`  → `Some("scmp")`
/// - `https://news.ycombinator.com/`        → `Some("ycombinator")`
///
/// Returns `None` when:
/// - The URL doesn't parse.
/// - The URL has no host (e.g. `mailto:`, `data:`).
/// - The host has no recognized public suffix (the PSL only knows real-world
///   TLDs and registrable suffixes, so `http://localhost/` or
///   `http://192.168.0.1/` will return None).
pub fn extract_domain(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host_str()?;

    // The `psl` crate gives us the registrable apex (e.g. "electrek.co.uk")
    // and the public suffix separately. The SLD ("electrek") is everything
    // in the apex before the suffix.
    let apex = psl::domain_str(host)?;
    let suffix = psl::suffix_str(host)?;

    // Strip ".<suffix>" off the end of the apex.
    let sld = apex
        .strip_suffix(suffix)
        .and_then(|s| s.strip_suffix('.'))
        .unwrap_or(apex);

    if sld.is_empty() {
        None
    } else {
        Some(sld.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_simple_com() {
        assert_eq!(
            extract_domain("https://www.example.eu/page").as_deref(),
            Some("example")
        );
    }

    #[test]
    fn extracts_under_multi_part_tld() {
        // `.co.uk`, `.com.br` — TLDs that aren't a single segment.
        assert_eq!(
            extract_domain("https://www.google.co.uk/search?q=foo").as_deref(),
            Some("google")
        );
        assert_eq!(
            extract_domain("https://www.uol.com.br/article").as_deref(),
            Some("uol")
        );
    }

    #[test]
    fn ignores_subdomains() {
        assert_eq!(
            extract_domain("https://amp.cnn.com/cnn/2020/article").as_deref(),
            Some("cnn")
        );
        assert_eq!(
            extract_domain("https://amp.scmp.com/news/asia/article").as_deref(),
            Some("scmp")
        );
        assert_eq!(
            extract_domain("https://www-cnn-com.cdn.ampproject.org/path").as_deref(),
            Some("ampproject")
        );
    }

    #[test]
    fn handles_no_subdomain() {
        assert_eq!(
            extract_domain("https://electrek.co/article").as_deref(),
            Some("electrek")
        );
    }

    #[test]
    fn returns_none_for_malformed_url() {
        assert!(extract_domain("not a url").is_none());
        assert!(extract_domain("").is_none());
    }

    #[test]
    fn returns_none_for_url_without_recognized_tld() {
        // localhost isn't in the PSL → returns None. IPs we don't bother
        // asserting on; their `psl` behavior is implementation-defined and
        // we never feed canonical-finding raw IPs anyway.
        assert!(extract_domain("http://localhost:8080/path").is_none());
    }
}
