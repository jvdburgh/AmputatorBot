//! `SCHEMA_MAINENTITY` method — JSON-LD `mainEntityOfPage`.
//!
//! Schema.org's `mainEntityOfPage` property, embedded in `<script type=
//! "application/ld+json">` blobs by many CMSes, points at the canonical
//! page URL. Ports the `SCHEMA_MAINENTITY` branch of
//! `archive/helpers/canonical_methods.py:60-63`.

use std::sync::LazyLock;

use regex::Regex;

use super::{MethodContext, find_in_inline_scripts, resolve_against};

static MAIN_ENTITY_RE: LazyLock<Regex> = LazyLock::new(|| {
    // Pulls the URL out of a JSON-LD blob: `"mainEntityOfPage": "..."`.
    Regex::new(r#""mainEntityOfPage"\s?:\s?["']([^"']+)["']"#).expect("mainEntityOfPage regex")
});

pub fn find(ctx: &MethodContext<'_>) -> Vec<String> {
    let doc = ctx.parsed_html();
    match find_in_inline_scripts(&doc, &MAIN_ENTITY_RE) {
        Some(url) => resolve_against(ctx.url, &url).into_iter().collect(),
        None => Vec::new(),
    }
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

    fn ctx(page: &Page) -> MethodContext<'_> {
        MethodContext {
            page,
            url: "https://amp.example.eu/article",
            original_url: "https://amp.example.eu/article",
            flags: CanonicalFlags::default(),
        }
    }

    #[test]
    fn finds_mainentityofpage_in_jsonld() {
        let page = page_with(
            r#"<html><head><script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@type": "NewsArticle",
                "mainEntityOfPage": "https://example.eu/article",
                "headline": "Title"
            }
            </script></head></html>"#,
        );
        let r = find(&ctx(&page));
        assert_eq!(r, vec!["https://example.eu/article"]);
    }

    #[test]
    fn unescapes_forward_slashes() {
        let page =
            page_with(r#"<script>"mainEntityOfPage":"https:\/\/example.eu\/article"</script>"#);
        let r = find(&ctx(&page));
        assert_eq!(r, vec!["https://example.eu/article"]);
    }

    #[test]
    fn returns_empty_when_absent() {
        let page = page_with(
            r#"<script>{"@type": "NewsArticle", "headline": "no mainEntity here"}</script>"#,
        );
        let r = find(&ctx(&page));
        assert!(r.is_empty());
    }
}
