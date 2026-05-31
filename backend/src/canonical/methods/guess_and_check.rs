//! `GUESS_AND_CHECK` — heuristic URL-only AMP-variant removal.
//!
//! Strips `/amp/` path segments, decodes `google.com/amp/s/` and
//! `cdn.ampproject.org` cache hosts, drops `amp.` subdomains. The "check"
//! happens at the orchestrator level: every candidate URL from every method
//! is fetched and its article text compared to the origin's, with the
//! result captured in `Canonical::confidence_*`. A `VERIFIED` result means
//! the article match succeeded; `UNCONFIRMED` means we couldn't verify.

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
        && let Some(unwrapped) = unwrap_ampproject_cache(parsed.path(), parsed.query())
    {
        out.push(unwrapped);
    }

    if let Some(host) = parsed.host_str()
        && (host == "google.com" || host.starts_with("www.google."))
        && let Some(unwrapped) = unwrap_google_amp_cache(parsed.path(), parsed.query())
    {
        out.push(unwrapped);
    }

    if let Some(host) = parsed.host_str()
        && (host == "bing.com" || host.starts_with("www.bing."))
        && let Some(unwrapped) = unwrap_google_amp_cache(parsed.path(), parsed.query())
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
        let path = parsed.path();
        let stripped_path = strip_amp_path_segment(path);
        if stripped_path != path {
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
        && let Some(cleaned_query) = strip_amp_query_params(query)
    {
        let mut u = parsed.clone();
        u.set_query(if cleaned_query.is_empty() {
            None
        } else {
            Some(&cleaned_query)
        });
        out.push(u.to_string());
    }

    out.sort();
    out.dedup();
    out
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

fn unwrap_ampproject_cache(path: &str, query: Option<&str>) -> Option<String> {
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
    Some(match query {
        Some(q) => format!("{scheme}://{rest}?{q}"),
        None => format!("{scheme}://{rest}"),
    })
}

fn unwrap_google_amp_cache(path: &str, query: Option<&str>) -> Option<String> {
    let after_prefix = path.strip_prefix("/amp/")?;
    let (scheme, rest) = match after_prefix.strip_prefix("s/") {
        Some(s) => ("https", s),
        None => ("http", after_prefix),
    };
    if rest.is_empty() || rest.starts_with('/') {
        return None;
    }
    Some(match query {
        Some(q) => format!("{scheme}://{rest}?{q}"),
        None => format!("{scheme}://{rest}"),
    })
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

fn strip_amp_query_params(query: &str) -> Option<String> {
    let mut changed = false;
    let kept: Vec<&str> = query
        .split('&')
        .filter(|pair| {
            let name = pair.split_once('=').map(|(n, _)| n).unwrap_or(pair);
            let is_amp_param = name == "amp"
                || name.ends_with("_amp")
                || (name == "output" && *pair == "output=amp");
            if is_amp_param {
                changed = true;
            }
            !is_amp_param
        })
        .collect();
    changed.then(|| kept.join("&"))
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
