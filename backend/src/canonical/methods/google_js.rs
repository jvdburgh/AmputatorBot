//! `GOOGLE_JS_REDIRECT` method — Google search redirect via JS.
//!
//! Some Google interstitial pages don't surface the destination URL via an
//! `<a>` link; they only set it in a JS variable `var redirectUrl = "..."`.
//! This method regex-greps inline `<script>` blocks for that pattern.
//!
//! Trigger condition: current URL contains `url?` AND `www.google.`.
//!
//! Ports the `GOOGLE_JS_REDIRECT` branch of
//! `archive/helpers/canonical_methods.py:46-51`.

use std::sync::LazyLock;

use regex::Regex;

use super::{MethodContext, find_in_inline_scripts, resolve_against};

static REDIRECT_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    // Pulls the URL out of `var redirectUrl = "...";` (or single-quoted).
    Regex::new(r#"var\s?redirectUrl\s?=\s?["']([^"']+)["']"#).expect("redirectUrl regex")
});

pub fn find(ctx: &MethodContext<'_>) -> Vec<String> {
    let cur = ctx.url.to_ascii_lowercase();
    if !cur.contains("url?") || !cur.contains("www.google.") {
        return Vec::new();
    }

    let doc = ctx.parsed_html();
    match find_in_inline_scripts(&doc, &REDIRECT_URL_RE) {
        Some(url) => resolve_against(ctx.url, &url).into_iter().collect(),
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
    fn skips_when_not_google_redirect() {
        let page = page_with(
            r#"<script>var redirectUrl = "https://example.com";</script>"#,
            "https://example.com/",
        );
        let r = find(&ctx(&page, "https://example.com/"));
        assert!(r.is_empty());
    }

    #[test]
    fn extracts_redirect_url_from_js() {
        let url = "https://www.google.com/url?q=https://example.com&sa=t";
        let page = page_with(
            r#"<html><body><script>
                var redirectUrl = "https://example.com/article";
            </script></body></html>"#,
            url,
        );
        let r = find(&ctx(&page, url));
        assert_eq!(r, vec!["https://example.com/article"]);
    }

    #[test]
    fn handles_single_quoted_url() {
        let url = "https://www.google.com/url?q=foo";
        let page = page_with(
            r#"<script>var redirectUrl = 'https://example.com/article';</script>"#,
            url,
        );
        let r = find(&ctx(&page, url));
        assert_eq!(r, vec!["https://example.com/article"]);
    }

    #[test]
    fn unescapes_forward_slashes() {
        let url = "https://www.google.com/url?q=foo";
        let page = page_with(
            r#"<script>var redirectUrl = "https:\/\/example.com\/article";</script>"#,
            url,
        );
        let r = find(&ctx(&page, url));
        assert_eq!(r, vec!["https://example.com/article"]);
    }

    #[test]
    fn returns_empty_when_pattern_absent() {
        let url = "https://www.google.com/url?q=foo";
        let page = page_with(
            r#"<script>var somethingElse = "https://example.com";</script>"#,
            url,
        );
        let r = find(&ctx(&page, url));
        assert!(r.is_empty());
    }

    #[test]
    fn ignores_external_scripts() {
        let url = "https://www.google.com/url?q=foo";
        let page = page_with(r#"<script src="/redirect.js"></script>"#, url);
        let r = find(&ctx(&page, url));
        assert!(r.is_empty());
    }
}
