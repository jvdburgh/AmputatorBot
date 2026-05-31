//! `REL` — `<link rel="canonical" href="...">`. The highest-priority signal:
//! HTML5-standard, set by ~every SEO-aware CMS, and the publisher's own
//! declaration of which URL is canonical.

use scraper::Selector;

use super::{MethodContext, resolve_against};

pub fn find(ctx: &MethodContext<'_>) -> Vec<String> {
    static SELECTOR: std::sync::LazyLock<Selector> =
        std::sync::LazyLock::new(|| Selector::parse("link[rel=canonical]").unwrap());

    let doc = ctx.parsed_html();
    doc.select(&SELECTOR)
        .filter_map(|el| el.value().attr("href"))
        .filter_map(|href| resolve_against(ctx.url, href))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::Page;
    use crate::canonical::methods::CanonicalFlags;

    fn page_with(html: &str) -> Page {
        Page {
            current_url: "https://amp.example.eu/article".into(),
            status_code: 200,
            title: "test".into(),
            html: html.into(),
        }
    }

    fn ctx<'a>(page: &'a Page) -> MethodContext<'a> {
        MethodContext {
            page,
            url: "https://amp.example.eu/article",
            original_url: "https://amp.example.eu/article",
            flags: CanonicalFlags::default(),
        }
    }

    #[test]
    fn finds_link_rel_canonical() {
        let page = page_with(
            r#"<html><head><link rel="canonical" href="https://example.eu/article"></head><body></body></html>"#,
        );
        let result = find(&ctx(&page));
        assert_eq!(result, vec!["https://example.eu/article"]);
    }

    #[test]
    fn resolves_relative_canonical() {
        let page = page_with(r#"<html><head><link rel="canonical" href="/article"></head></html>"#);
        let result = find(&ctx(&page));
        assert_eq!(result, vec!["https://amp.example.eu/article"]);
    }

    #[test]
    fn returns_empty_when_missing() {
        let page = page_with("<html><head></head><body>no canonical</body></html>");
        let result = find(&ctx(&page));
        assert!(result.is_empty());
    }

    #[test]
    fn ignores_other_rel_values() {
        let page = page_with(
            r#"<html><head>
                <link rel="alternate" href="https://example.eu/amp/">
                <link rel="canonical" href="https://example.eu/article">
            </head></html>"#,
        );
        let result = find(&ctx(&page));
        assert_eq!(result, vec!["https://example.eu/article"]);
    }
}
