//! `GOOGLE_MANUAL_REDIRECT` method — Google search outbound-link interstitials.
//!
//! When Google sometimes redirects search-result clicks through
//! `https://www.google.com/url?q=https://target.example/...`, the interstitial
//! page contains plain `<a href="...">` links to the actual destination. This
//! method scrapes those.
//!
use scraper::Selector;

use super::{MethodContext, resolve_against};

pub fn find(ctx: &MethodContext<'_>) -> Vec<String> {
    let cur = ctx.page.current_url.to_ascii_lowercase();
    if !cur.contains("url?q=") || !cur.contains("www.google.") {
        return Vec::new();
    }

    static A_HREF: std::sync::LazyLock<Selector> =
        std::sync::LazyLock::new(|| Selector::parse("a[href]").expect("a selector"));

    let doc = ctx.parsed_html();
    doc.select(&A_HREF)
        .filter_map(|el| el.value().attr("href"))
        .filter_map(|href| resolve_against(&ctx.page.current_url, href))
        .collect()
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
            r#"<a href="https://example.eu">link</a>"#,
            "https://example.eu/",
        );
        let r = find(&ctx(&page, "https://example.eu/"));
        assert!(r.is_empty());
    }

    #[test]
    fn returns_anchor_hrefs_on_google_redirect() {
        let url = "https://www.google.com/url?q=https://example.eu/article";
        let page = page_with(
            r#"<html><body>
                <a href="https://example.eu/article">go</a>
                <a href="https://other.com/back">back</a>
            </body></html>"#,
            url,
        );
        let r = find(&ctx(&page, url));
        assert_eq!(
            r,
            vec!["https://example.eu/article", "https://other.com/back"]
        );
    }

    #[test]
    fn resolves_relative_anchors() {
        let url = "https://www.google.com/url?q=https://example.eu/x";
        let page = page_with(r#"<html><body><a href="/article">x</a></body></html>"#, url);
        let r = find(&ctx(&page, url));
        assert_eq!(r, vec!["https://www.google.com/article"]);
    }

    /// Locks in the fix for the PBS / google.com/amp/s issue: the resolver
    /// asks the fetcher for the AMP-cache URL, but Google redirects to a
    /// `google.com/url?q=` interstitial. The trigger has to check
    /// `page.current_url` (post-redirect) rather than the URL we requested.
    #[test]
    fn fires_when_only_redirected_url_matches_pattern() {
        let requested = "https://www.google.com/amp/s/www.pbs.org/newshour/amp/article";
        let redirected = "https://www.google.com/url?q=https://www.pbs.org/newshour/article";
        let page = page_with(
            r#"<html><body>
                <a href="https://www.pbs.org/newshour/article">go</a>
            </body></html>"#,
            redirected,
        );
        let r = find(&ctx(&page, requested));
        assert_eq!(r, vec!["https://www.pbs.org/newshour/article"]);
    }
}
