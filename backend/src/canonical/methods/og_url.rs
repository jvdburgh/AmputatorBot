//! `<meta property="og:url" content="...">` method.
//!
//! Open Graph URLs published by content publishers (Facebook OG standard)
//! often point at the canonical, non-AMP version of an article. Ports the
//! `OG_URL` branch of `archive/helpers/canonical_methods.py`.

use scraper::Selector;

use super::{MethodContext, resolve_against};

pub fn find(ctx: &MethodContext<'_>) -> Vec<String> {
    static SELECTOR: std::sync::LazyLock<Selector> = std::sync::LazyLock::new(|| {
        Selector::parse(r#"meta[property="og:url"]"#).expect("og:url selector")
    });

    let doc = ctx.parsed_html();
    doc.select(&SELECTOR)
        .filter_map(|el| el.value().attr("content"))
        .filter_map(|content| resolve_against(ctx.url, content))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::Page;
    use crate::canonical::methods::CanonicalFlags;

    fn page_with(html: &str) -> Page {
        Page {
            current_url: "https://amp.example.com/article".into(),
            status_code: 200,
            title: "test".into(),
            html: html.into(),
        }
    }

    fn ctx<'a>(page: &'a Page) -> MethodContext<'a> {
        MethodContext {
            page,
            url: "https://amp.example.com/article",
            original_url: "https://amp.example.com/article",
            flags: CanonicalFlags::default(),
        }
    }

    #[test]
    fn finds_og_url() {
        let page = page_with(
            r#"<html><head><meta property="og:url" content="https://example.com/article"></head></html>"#,
        );
        let result = find(&ctx(&page));
        assert_eq!(result, vec!["https://example.com/article"]);
    }

    #[test]
    fn ignores_other_og_properties() {
        let page = page_with(
            r#"<html><head>
                <meta property="og:title" content="Article Title">
                <meta property="og:image" content="https://example.com/img.png">
                <meta property="og:url" content="https://example.com/article">
            </head></html>"#,
        );
        let result = find(&ctx(&page));
        assert_eq!(result, vec!["https://example.com/article"]);
    }

    #[test]
    fn returns_empty_when_missing() {
        let page = page_with(r#"<html><head></head></html>"#);
        let result = find(&ctx(&page));
        assert!(result.is_empty());
    }
}
