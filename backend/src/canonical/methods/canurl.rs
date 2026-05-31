//! `CANURL` — custom `a="amp-canurl"` attribute some publishers add to
//! a tag (alongside or instead of `rel=canonical`) to mark the non-AMP
//! variant. Matched via CSS attribute-selector `[a="amp-canurl"]`.

use scraper::Selector;

use super::{MethodContext, resolve_against};

pub fn find(ctx: &MethodContext<'_>) -> Vec<String> {
    static SELECTOR: std::sync::LazyLock<Selector> =
        std::sync::LazyLock::new(|| Selector::parse(r#"[a="amp-canurl"]"#).unwrap());

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
    fn finds_amp_canurl_attribute() {
        let page = page_with(
            r#"<html><body><a a="amp-canurl" href="https://example.eu/article">canonical</a></body></html>"#,
        );
        let result = find(&ctx(&page));
        assert_eq!(result, vec!["https://example.eu/article"]);
    }

    #[test]
    fn returns_empty_when_missing() {
        let page = page_with(r#"<html><body><a href="https://other.com/">link</a></body></html>"#);
        let result = find(&ctx(&page));
        assert!(result.is_empty());
    }

    #[test]
    fn resolves_relative_url() {
        let page =
            page_with(r#"<html><body><a a="amp-canurl" href="/article">x</a></body></html>"#);
        let result = find(&ctx(&page));
        assert_eq!(result, vec!["https://amp.example.eu/article"]);
    }
}
