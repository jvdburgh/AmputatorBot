//! AMP URL detection.
//!
//! Substring scan over **parsed URL components** (host / path / query) so a
//! whole-URL match like `https://amputeestore.com/...` doesn't false-positive
//! on `/amp` just because of the `//amp` after the scheme.

use url::Url;

/// Scanned against each URL component by [`is_amp_url`].
pub(crate) const AMP_KEYWORDS: &[&str] = &[
    "/amp", "amp/", ".amp", "amp.", "?amp", "amp?", "=amp", "amp=", "&amp", "amp&", "%amp", "amp%",
    "_amp", "amp_",
];

/// Domains that the keyword scan misfires on (band+amp, etc.).
const DENYLISTED_DOMAINS: &[&str] = &[
    "video.twimg.kim",
    "bandcamp.com",
    "progonlymusic.com",
    "redd.it",
    "reddit.com",
    "spotify.com",
    "youtube.com",
    "youtu.be",
];

/// `true` if any of [`AMP_KEYWORDS`] appears as a substring in the URL's
/// host, path, or query — unless the host is on [`DENYLISTED_DOMAINS`].
pub fn is_amp_url(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };

    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    if DENYLISTED_DOMAINS
        .iter()
        .any(|d| host_matches_domain(&host, d))
    {
        return false;
    }

    if has_amp_keyword(&host) {
        return true;
    }

    let path = parsed.path().to_ascii_lowercase();
    if has_amp_keyword(&path) {
        return true;
    }

    if let Some(query) = parsed.query() {
        let query = query.to_ascii_lowercase();
        if has_amp_keyword(&query) {
            return true;
        }
    }

    false
}

fn has_amp_keyword(s: &str) -> bool {
    AMP_KEYWORDS.iter().any(|kw| s.contains(kw))
}

/// `true` when `host` is exactly `domain` or a real subdomain of it.
///
/// Plain `host.ends_with(domain)` would also match unrelated hosts like
/// `notyoutube.com` ↔ `youtube.com` or `notampproject.org` ↔ `ampproject.org`,
/// so denylist / cache-host checks need a dot-boundary on the suffix side.
fn host_matches_domain(host: &str, domain: &str) -> bool {
    host == domain
        || (host.len() > domain.len()
            && host.ends_with(domain)
            && host.as_bytes()[host.len() - domain.len() - 1] == b'.')
}

/// Returns `true` if the URL is hosted on a known AMP cache
/// (Google AMP, Bing AMP, or `ampproject.{net,org}`).
///
/// Ports `praw-python-archive/helpers/checker_utils.py:check_if_cached`. Like
/// [`is_amp_url`], we evaluate against parsed components rather than the
/// raw string — gives the same answers but is harder to fool.
pub fn is_cached_amp(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };

    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    let path = parsed.path().to_ascii_lowercase();

    if host_matches_domain(&host, "ampproject.net") || host_matches_domain(&host, "ampproject.org")
    {
        return true;
    }

    let on_google = host.starts_with("www.google.");
    let on_bing = host.starts_with("www.bing.");
    if (on_google || on_bing) && path.starts_with("/amp/") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_google_amp_cache_url() {
        assert!(is_amp_url(
            "https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3/amp/"
        ));
    }

    #[test]
    fn detects_publisher_amp_path() {
        assert!(is_amp_url(
            "https://www.bbc.com/news/world-europe-12345/amp"
        ));
        assert!(is_amp_url("https://www.fox5atlanta.com/video/foo.amp"));
    }

    #[test]
    fn detects_amp_subdomain() {
        assert!(is_amp_url("https://amp.cnn.com/cnn/2020/some-article"));
    }

    #[test]
    fn detects_amp_query_param() {
        assert!(is_amp_url("https://example.eu/article?amp=1"));
        assert!(is_amp_url("https://example.eu/article?output=amp"));
    }

    #[test]
    fn detects_ampproject_subdomain() {
        assert!(is_amp_url(
            "https://www-cnn-com.cdn.ampproject.org/c/s/www.cnn.com/sample"
        ));
    }

    #[test]
    fn rejects_amputeestore_false_positive() {
        // Canonical false-positive case: a whole-URL substring scan flags
        // `/amp` inside `//amputeestore.com` after the scheme.
        assert!(!is_amp_url(
            "https://amputeestore.com/collections/prosthetic-socks/products/knit-rite-liner-liner-sock?variant=4114742017"
        ));
        assert!(!is_amp_url(
            "https://amputeestore.com/products/tamarack-glidewear-prosthetic-liner-patch"
        ));
        assert!(!is_amp_url(
            "https://amputeestore.com/products/alps-antiperspirant-spray"
        ));
    }

    #[test]
    fn rejects_non_amp_urls() {
        assert!(!is_amp_url("https://www.google.com/search?q=foo"));
        assert!(!is_amp_url("https://news.ycombinator.com/item?id=42"));
        assert!(!is_amp_url("https://en.wikipedia.org/wiki/Wikipedia"));
    }

    #[test]
    fn rejects_denylisted_domains_even_with_amp_in_path() {
        assert!(!is_amp_url("https://www.youtube.com/amp/some-video"));
        assert!(!is_amp_url("https://open.spotify.com/amp/track/123"));
        assert!(!is_amp_url("https://bandcamp.com/amp/foo"));
    }

    #[test]
    fn denylist_does_not_match_unrelated_hosts() {
        // `notyoutube.com` ends with `youtube.com` but isn't a subdomain.
        assert!(is_amp_url("https://notyoutube.com/amp/some-video"));
        assert!(is_amp_url("https://myreddit.com/article/amp"));
    }

    #[test]
    fn cached_does_not_match_unrelated_hosts() {
        // `notampproject.org` ends with `ampproject.org` but is not a real
        // subdomain — must NOT be classified as an AMP cache.
        assert!(!is_cached_amp("https://notampproject.org/some/path"));
        assert!(!is_cached_amp("https://fakeampproject.net/x"));
    }

    #[test]
    fn rejects_malformed_urls() {
        assert!(!is_amp_url("not a url"));
        assert!(!is_amp_url(""));
        assert!(!is_amp_url("amp"));
    }

    #[test]
    fn cached_detects_google_amp() {
        assert!(is_cached_amp(
            "https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3/amp/"
        ));
        assert!(is_cached_amp(
            "https://www.google.co.uk/amp/s/example.eu/article"
        ));
    }

    #[test]
    fn cached_detects_bing_amp() {
        assert!(is_cached_amp(
            "https://www.bing.com/amp/s/example.eu/article"
        ));
    }

    #[test]
    fn cached_detects_ampproject() {
        assert!(is_cached_amp("https://cdn.ampproject.org/c/s/example.eu"));
        assert!(is_cached_amp(
            "https://www-cnn-com.cdn.ampproject.org/c/s/foo"
        ));
        assert!(is_cached_amp("https://example.ampproject.net/some/path"));
    }

    #[test]
    fn cached_rejects_publisher_amp_pages() {
        // A publisher's own /amp page is AMP but not "cached" on a third-party CDN.
        assert!(!is_cached_amp("https://www.bbc.com/news/world-europe/amp"));
        assert!(!is_cached_amp("https://amp.cnn.com/cnn/2020/article"));
    }
}
