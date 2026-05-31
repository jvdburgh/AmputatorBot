//! `GUESS_AND_CHECK` — heuristic URL-only AMP-variant removal.
//!
//! Strips `/amp/` path segments, decodes `google.com/amp/s/` and
//! `cdn.ampproject.org` cache hosts, drops `amp.` subdomains, and removes
//! any query params whose `name=value` pair contains the substring `amp`
//! (case-insensitive). Catches `?amp=1`, `?hs_amp=true`, Google's
//! `amp_js_v=…&amp_gsa=…` injections, and `output=amp`, while preserving
//! routing params like `?id=70031273`. The "check" happens at the
//! orchestrator level: every candidate URL is fetched and its article text
//! compared to the origin's, captured in `Canonical::confidence_*`.
//! `VERIFIED` = article match succeeded; `UNCONFIRMED` = couldn't verify.

use url::Url;

use super::MethodContext;

pub fn find(ctx: &MethodContext<'_>) -> Vec<String> {
    strip_amp_variants(ctx.url)
}

pub(crate) fn strip_amp_variants(url_str: &str) -> Vec<String> {
    let Ok(parsed) = Url::parse(url_str) else {
        return Vec::new();
    };

    let mut out: Vec<String> = Vec::new();

    if let Some(host) = parsed.host_str()
        && host.ends_with(".cdn.ampproject.org")
        && let Some(unwrapped) = unwrap_ampproject_cache(parsed.path())
    {
        out.push(unwrapped);
    }

    if let Some(host) = parsed.host_str()
        && (host == "google.com" || host.starts_with("www.google."))
        && let Some(unwrapped) = unwrap_google_amp_cache(parsed.path())
    {
        out.push(unwrapped);
    }

    if let Some(host) = parsed.host_str()
        && (host == "bing.com" || host.starts_with("www.bing."))
        && let Some(unwrapped) = unwrap_google_amp_cache(parsed.path())
    {
        out.push(unwrapped);
    }

    if let Some(host) = parsed.host_str()
        && let Some(stripped) = host.strip_prefix("amp.")
    {
        let mut u = parsed.clone();
        let _ = u.set_host(Some(stripped));
        out.push(u.to_string());
    }

    if !is_amp_cache_host(parsed.host_str()) {
        let stripped_path = strip_amp_path_segment(parsed.path());
        if stripped_path != parsed.path() {
            let mut u = parsed.clone();
            u.set_path(&stripped_path);
            out.push(u.to_string());
        }
    }

    if let Some(stripped) = parsed.path().strip_suffix(".amp") {
        let mut u = parsed.clone();
        u.set_path(stripped);
        out.push(u.to_string());
    }

    if let Some(query) = parsed.query()
        && let Some(cleaned) = strip_amp_query_params(query)
    {
        let mut u = parsed.clone();
        u.set_query(if cleaned.is_empty() {
            None
        } else {
            Some(&cleaned)
        });
        out.push(u.to_string());
    }

    // Cache-unwrap rules above preserve the original query. Apply the
    // amp-query strip to every produced candidate too so e.g.
    // `google.com/amp/s/example.eu/article?amp_js_v=0.1` → unwrap to
    // `example.eu/article?amp_js_v=0.1` → strip to `example.eu/article` is
    // a single candidate in one pass.
    out = out
        .into_iter()
        .map(|url| {
            let Ok(mut parsed) = Url::parse(&url) else {
                return url;
            };
            let Some(query) = parsed.query() else {
                return url;
            };
            let Some(cleaned) = strip_amp_query_params(query) else {
                return url;
            };
            parsed.set_query(if cleaned.is_empty() {
                None
            } else {
                Some(&cleaned)
            });
            parsed.to_string()
        })
        .collect();

    out.sort();
    out.dedup();
    out
}

/// A query pair (`name=value` or just `name`) is an AMP marker iff its
/// lowercased text contains the substring `amp`. Mirrors the spirit of
/// [`crate::canonical::amp_detect::AMP_KEYWORDS`] but at the structured
/// query-pair level: catches `amp=1`, `hs_amp=true`, `amp_js_v=…`,
/// `output=amp`, etc., while leaving `id=70031273` alone.
///
/// False-positive risk: a value like `vendor=AmpEnergy` would get stripped.
/// Accepted trade-off — vanishingly rare in real-world canonical URLs.
fn is_amp_query_param(pair: &str) -> bool {
    pair.to_ascii_lowercase().contains("amp")
}

fn strip_amp_query_params(query: &str) -> Option<String> {
    let mut changed = false;
    let kept: Vec<&str> = query
        .split('&')
        .filter(|pair| {
            let drop = is_amp_query_param(pair);
            if drop {
                changed = true;
            }
            !drop
        })
        .collect();
    changed.then(|| kept.join("&"))
}

pub(crate) fn is_amp_cache_host(host: Option<&str>) -> bool {
    let Some(h) = host else { return false };
    h == "google.com"
        || h.starts_with("www.google.")
        || h == "bing.com"
        || h.starts_with("www.bing.")
        || h.ends_with(".cdn.ampproject.org")
}

/// True when the URL's host is a known AMP cache. Article-text similarity
/// is meaningless for these URLs because the cache responds with an
/// interstitial / redirect-notice page, not the cached article — comparing
/// that 200-byte boilerplate against the real canonical's text produces a
/// trivially low score that misrepresents the result's correctness.
pub(crate) fn url_is_on_cache_host(url: &str) -> bool {
    url::Url::parse(url)
        .ok()
        .map(|u| is_amp_cache_host(u.host_str()))
        .unwrap_or(false)
}

fn unwrap_ampproject_cache(path: &str) -> Option<String> {
    let after_prefix = path
        .strip_prefix("/c/")
        .or_else(|| path.strip_prefix("/v/"))?;
    let (scheme, rest) = match after_prefix.strip_prefix("s/") {
        Some(s) => ("https", s),
        None => ("http", after_prefix),
    };
    if rest.is_empty() || rest.starts_with('/') {
        return None;
    }
    Some(format!("{scheme}://{rest}"))
}

fn unwrap_google_amp_cache(path: &str) -> Option<String> {
    let after_prefix = path.strip_prefix("/amp/")?;
    let (scheme, rest) = match after_prefix.strip_prefix("s/") {
        Some(s) => ("https", s),
        None => ("http", after_prefix),
    };
    if rest.is_empty() || rest.starts_with('/') {
        return None;
    }
    Some(format!("{scheme}://{rest}"))
}

fn strip_amp_path_segment(path: &str) -> String {
    let mut out = path.replace("/amp/", "/");
    if let Some(rest) = out.strip_suffix("/amp/") {
        out = if rest.is_empty() {
            "/".to_string()
        } else {
            rest.to_string()
        };
    } else if let Some(rest) = out.strip_suffix("/amp") {
        out = if rest.is_empty() {
            "/".to_string()
        } else {
            rest.to_string()
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwraps_google_amp_cache_https() {
        let result = strip_amp_variants(
            "https://www.google.com/amp/s/www.pbs.org/newshour/politics/article",
        );
        assert!(result.contains(&"https://www.pbs.org/newshour/politics/article".to_string()));
    }

    #[test]
    fn unwraps_bing_amp_cache() {
        let result = strip_amp_variants("https://www.bing.com/amp/s/example.com/article");
        assert!(result.contains(&"https://example.com/article".to_string()));
    }

    #[test]
    fn unwraps_cdn_ampproject_org() {
        let result = strip_amp_variants(
            "https://www-pbs-org.cdn.ampproject.org/c/s/www.pbs.org/newshour/politics/article",
        );
        assert!(result.contains(&"https://www.pbs.org/newshour/politics/article".to_string()));
    }

    #[test]
    fn strips_amp_subdomain() {
        let result = strip_amp_variants("https://amp.example.com/news/story");
        assert!(result.contains(&"https://example.com/news/story".to_string()));
    }

    #[test]
    fn strips_amp_path_segment_in_middle() {
        let result = strip_amp_variants("https://news.sky.com/story/amp/article-13161235");
        assert!(result.contains(&"https://news.sky.com/story/article-13161235".to_string()));
    }

    #[test]
    fn strips_amp_extension() {
        let result = strip_amp_variants("https://bbc.com/news/articles/cxyz.amp");
        assert!(result.contains(&"https://bbc.com/news/articles/cxyz".to_string()));
    }

    #[test]
    fn strips_amp_query_param() {
        let result = strip_amp_variants("https://example.com/article?hs_amp=true");
        assert!(result.contains(&"https://example.com/article".to_string()));
    }

    #[test]
    fn strips_google_amp_cache_injected_params_but_keeps_others() {
        // The cdn.ampproject.org viewer appends `amp_js_v=…&amp_gsa=…` to
        // every URL it serves. Both pairs contain "amp" → stripped.
        // utm_source=x doesn't contain "amp" → kept.
        let result = strip_amp_variants(
            "https://www.example.com/article?amp_js_v=0.1&amp_gsa=1&utm_source=x",
        );
        assert!(
            result.contains(&"https://www.example.com/article?utm_source=x".to_string()),
            "amp_* stripped, utm_source kept: got {result:?}"
        );
    }

    /// The user's reported case: routing-essential query params (like
    /// `?id=…` used by ABC News for article lookup) must be preserved.
    /// Stripping all queries would break the URL; the substring filter
    /// only touches pairs that mention `amp`.
    #[test]
    fn preserves_routing_query_params_without_amp() {
        let url = "https://abcnews.go.com/Politics/story?id=70031273&amp_gsa=1";
        let result = strip_amp_variants(url);
        assert!(
            result.contains(&"https://abcnews.go.com/Politics/story?id=70031273".to_string()),
            "id=… should be preserved, amp_gsa stripped: {result:?}"
        );
    }

    /// Edge case the substring filter knowingly accepts: a value like
    /// `vendor=AmpEnergy` is technically not an AMP marker but does contain
    /// "amp", so we strip it. Acceptable false positive — vanishingly rare
    /// in real canonical URLs and the cost of a smarter filter outweighs the
    /// occasional miss.
    #[test]
    fn substring_filter_strips_amp_in_value_too() {
        let result = strip_amp_variants("https://example.com/article?vendor=AmpEnergy");
        assert!(
            result.contains(&"https://example.com/article".to_string()),
            "vendor=AmpEnergy treated as amp-flavored (acceptable false positive): {result:?}"
        );
    }

    #[test]
    fn returns_empty_on_invalid_url() {
        assert!(strip_amp_variants("not a url").is_empty());
    }

    #[test]
    fn does_not_strip_amp_inside_a_word() {
        let result = strip_amp_variants("https://example.com/sports/champion");
        assert!(result.is_empty());
    }

    /// Path-strip rule must NOT fire when the host is itself an AMP cache —
    /// e.g. for `google.com/amp/s/<pub>/.../amp/...` blindly replacing
    /// `/amp/` in the path would produce a garbage Google URL like
    /// `google.com/s/<pub>/...`. Cache decoders handle cache hosts; path-strip
    /// only runs once we've unwrapped to the publisher's URL.
    #[test]
    fn skips_path_strip_on_cache_host() {
        let result =
            strip_amp_variants("https://www.google.com/amp/s/news.sky.com/story/amp/article");
        // The cache-unwrap rule should still fire.
        assert!(result.contains(&"https://news.sky.com/story/amp/article".to_string()));
        // The garbage path-strip result must NOT be present.
        assert!(
            !result
                .iter()
                .any(|u| u.starts_with("https://www.google.com/s/")),
            "path-strip leaked on cache host: {result:?}"
        );
    }

    #[test]
    fn url_is_on_cache_host_matches_known_caches() {
        assert!(url_is_on_cache_host("https://www.google.com/amp/s/x/y"));
        assert!(url_is_on_cache_host("https://www.bing.com/amp/s/x/y"));
        assert!(url_is_on_cache_host(
            "https://www-pbs-org.cdn.ampproject.org/c/s/x/y"
        ));
        assert!(!url_is_on_cache_host("https://news.sky.com/story/article"));
        assert!(!url_is_on_cache_host("https://example.com/article"));
        assert!(!url_is_on_cache_host("not a url"));
    }

    #[test]
    fn skips_path_strip_on_cdn_ampproject_host() {
        let result = strip_amp_variants(
            "https://www-news-example-com.cdn.ampproject.org/c/s/news.example.com/story/amp/article",
        );
        // Cache-unwrap fires.
        assert!(result.contains(&"https://news.example.com/story/amp/article".to_string()));
        // Path-strip on the cdn.ampproject.org host should NOT fire.
        assert!(
            !result
                .iter()
                .any(|u| u.contains("cdn.ampproject.org") && !u.contains("/amp/")),
            "path-strip leaked on cdn host: {result:?}"
        );
    }
}
