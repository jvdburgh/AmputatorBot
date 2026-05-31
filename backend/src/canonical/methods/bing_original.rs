//! `BING_ORIGINAL_URL` method — Bing AMP cache outbound URL.
//!
//! Bing's AMP cache pages embed the publisher's canonical URL in an inline
//! JS object like `"originalUrl": "https://publisher/article"`. This method
//! regex-greps for that.
//!
//! Trigger condition: the page's final URL (after redirects) contains
//! `/amp/s/` AND `www.bing.`.
//!
//! Ports the `BING_ORIGINAL_URL` branch of
//! `praw-python-archive/helpers/canonical_methods.py:53-57`.

use std::sync::LazyLock;

use regex::Regex;

use super::{MethodContext, find_in_inline_scripts, resolve_against};

static ORIGINAL_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    // Pulls the URL out of a JSON-like blob: `"originalUrl": "..."`
    Regex::new(r#"["']originalUrl["']\s?:\s?["']([^"']+)["']"#).expect("originalUrl regex")
});

pub fn find(ctx: &MethodContext<'_>) -> Vec<String> {
    let cur = ctx.page.current_url.to_ascii_lowercase();
    if !cur.contains("/amp/s/") || !cur.contains("www.bing.") {
        return Vec::new();
    }

    let doc = ctx.parsed_html();
    match find_in_inline_scripts(&doc, &ORIGINAL_URL_RE) {
        Some(url) => resolve_against(&ctx.page.current_url, &url)
            .into_iter()
            .collect(),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::Page;
    use crate::canonical::methods::CanonicalFlags;

    fn page_with(html: &str, current_url: &str) -> Page {
        Page {
            current_url: current_url.into(),
            status_code: 200,
            title: "test".into(),
            html: html.into(),
        }
    }

    fn ctx<'a>(page: &'a Page, url: &'a str) -> MethodContext<'a> {
        MethodContext {
            page,
            url,
            original_url: url,
            flags: CanonicalFlags::default(),
        }
    }

    #[test]
    fn skips_when_not_bing_amp() {
        let page = page_with(
            r#"<script>{"originalUrl": "https://example.eu"}</script>"#,
            "https://example.eu/",
        );
        let r = find(&ctx(&page, "https://example.eu/"));
        assert!(r.is_empty());
    }

    #[test]
    fn extracts_original_url_from_bing_amp() {
        let url = "https://www.bing.com/amp/s/example.eu/article";
        let page = page_with(
            r#"<html><body><script>
                {"foo": 1, "originalUrl": "https://example.eu/article", "bar": 2}
            </script></body></html>"#,
            url,
        );
        let r = find(&ctx(&page, url));
        assert_eq!(r, vec!["https://example.eu/article"]);
    }

    #[test]
    fn handles_single_quoted_url() {
        let url = "https://www.bing.com/amp/s/example.eu/article";
        let page = page_with(
            r#"<script>{'originalUrl':'https://example.eu/article'}</script>"#,
            url,
        );
        let r = find(&ctx(&page, url));
        assert_eq!(r, vec!["https://example.eu/article"]);
    }

    #[test]
    fn unescapes_forward_slashes() {
        let url = "https://www.bing.com/amp/s/example.eu/article";
        let page = page_with(
            r#"<script>"originalUrl":"https:\/\/example.eu\/article"</script>"#,
            url,
        );
        let r = find(&ctx(&page, url));
        assert_eq!(r, vec!["https://example.eu/article"]);
    }

    #[test]
    fn returns_empty_when_pattern_absent() {
        let url = "https://www.bing.com/amp/s/example.eu/article";
        let page = page_with(r#"<script>{"foo": 1}</script>"#, url);
        let r = find(&ctx(&page, url));
        assert!(r.is_empty());
    }
}
