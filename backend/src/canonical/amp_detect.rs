//! AMP URL detection.
//!
//! Ports `archive/helpers/checker_utils.py` (`check_if_amp`, `check_if_cached`)
//! and the keyword/denylist lists from `archive/static/static.py`.
//!
//! ## Improvements over the legacy implementation
//!
//! The Python implementation does a substring scan over the **whole URL string**
//! with 14 short keywords. That produces false positives like
//! `https://amputeestore.com/products/...` matching `/amp` (because of `//amp`
//! after the scheme), even though the URL is plainly not an AMP page.
//!
//! Our port keeps the same 14 keywords and the same denylist, but applies the
//! keyword scan to **parsed URL components** (host, path, query) separately.
//! That eliminates the `amputeestore.com`-class false positives without losing
//! any of the legitimate matches (`/amp/`, `amp.`, `amp=1`, etc.). The Python
//! check on the whole string would match `/amp` in `https://amputeestore.com`
//! because it sees `//a` then `mp` somewhere later — our version requires the
//! match to land entirely within a single component.

use url::Url;

/// 14 substring patterns from `archive/static/static.py:8-9`.
///
/// Applied against each URL component (host, path, query) by
/// [`is_amp_url`]. The order is preserved from the legacy bot, though
/// short-circuit evaluation means it doesn't strictly matter.
///
/// Visible to the rest of `canonical::` because GUESS_AND_CHECK mutates
/// URLs by removing each keyword in turn — see `methods::guess_and_check`.
pub(crate) const AMP_KEYWORDS: &[&str] = &[
    "/amp", "amp/", ".amp", "amp.", "?amp", "amp?", "=amp", "amp=", "&amp", "amp&", "%amp", "amp%",
    "_amp", "amp_",
];

/// Domains hard-excluded from AMP detection regardless of URL shape.
///
/// Ports `archive/static/static.py:10`. These are domains where the substring
/// match historically misfired, so the legacy bot just bailed.
const DENYLISTED_DOMAINS: &[&str] = &[
    "bandcamp.com",
    "progonlymusic.com",
    "spotify.com",
    "youtube.com",
];

/// Returns `true` if the URL appears to be an AMP URL.
///
/// Rules:
/// 1. The URL must parse.
/// 2. The host is checked against [`DENYLISTED_DOMAINS`] — denied → not AMP.
/// 3. The host, path, and query string are each scanned for any of the
///    [`AMP_KEYWORDS`]. A match in any single component → AMP.
///
/// Component-scoping is the key fix vs. the legacy whole-string scan.
pub fn is_amp_url(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };

    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    if DENYLISTED_DOMAINS.iter().any(|d| host.ends_with(d)) {
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

/// Returns `true` if the URL is hosted on a known AMP cache
/// (Google AMP, Bing AMP, or `ampproject.{net,org}`).
///
/// Ports `archive/helpers/checker_utils.py:check_if_cached`. Like
/// [`is_amp_url`], we evaluate against parsed components rather than the
/// raw string — gives the same answers but is harder to fool.
pub fn is_cached_amp(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };

    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    let path = parsed.path().to_ascii_lowercase();

    if host.ends_with("ampproject.net") || host.ends_with("ampproject.org") {
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
        assert!(is_amp_url("https://example.com/article?amp=1"));
        assert!(is_amp_url("https://example.com/article?output=amp"));
    }

    #[test]
    fn detects_ampproject_subdomain() {
        // ampproject hosts trip `.amp` in their host string anyway, but the
        // is_cached_amp check below handles them more specifically.
        assert!(is_amp_url(
            "https://www-cnn-com.cdn.ampproject.org/c/s/www.cnn.com/sample"
        ));
    }

    #[test]
    fn rejects_amputeestore_false_positive() {
        // This is the canonical false-positive case from the URLConversions
        // 500-row sample — the legacy substring scan flagged these as AMP
        // because `/amp` matches against `//amp` after the scheme. Our
        // component-scoped scan correctly says no.
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
            "https://www.google.co.uk/amp/s/example.com/article"
        ));
    }

    #[test]
    fn cached_detects_bing_amp() {
        assert!(is_cached_amp(
            "https://www.bing.com/amp/s/example.com/article"
        ));
    }

    #[test]
    fn cached_detects_ampproject() {
        assert!(is_cached_amp("https://cdn.ampproject.org/c/s/example.com"));
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
