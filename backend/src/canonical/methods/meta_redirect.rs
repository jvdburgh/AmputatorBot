//! `META_REDIRECT` method — `<meta http-equiv="refresh">` URL extraction.
//!
//! Older sites and some AMP cache pages use HTML meta-refresh to bounce
//! visitors to the canonical URL. The `content` attribute looks like
//! `"0; url=https://example.eu/article"` (timeout + URL).
//!
//! Gated by `ctx.flags.use_mr` (mirrors the Python `use_mr` flag — meta-
//! redirect is one of the resource-heavy methods that gets disabled by the
//! orchestration loop after a non-AMP canonical has been found).
//!
//! Ports the `META_REDIRECT` branch of `archive/helpers/canonical_methods.py:73-77`
//! and the helper `get_can_urls_with_meta_redirect:180-198`.

use scraper::Selector;

use super::{MethodContext, resolve_against};

pub fn find(ctx: &MethodContext<'_>) -> Vec<String> {
    if !ctx.flags.use_mr {
        return Vec::new();
    }

    static SELECTOR: std::sync::LazyLock<Selector> = std::sync::LazyLock::new(|| {
        Selector::parse(r#"meta[http-equiv="refresh"][content]"#).expect("meta-refresh selector")
    });

    let doc = ctx.parsed_html();
    doc.select(&SELECTOR)
        .filter_map(|el| el.value().attr("content"))
        .filter_map(extract_url_from_content)
        .filter_map(|url| resolve_against(ctx.url, &url))
        .collect()
}

/// Extract the URL portion of a `content="0; url=https://..."` attribute.
///
/// The format is loose: any prefix (typically a timeout in seconds), then a
/// literal `url=` (case-insensitive in practice but Python only matches
/// lowercase, so we do too), then the URL. Returns `None` if `url=` doesn't
/// appear.
fn extract_url_from_content(content: &str) -> Option<String> {
    let idx = content.find("url=")?;
    let url = &content[idx + "url=".len()..];
    let url = url.trim().trim_matches(|c| c == '"' || c == '\'');
    if url.is_empty() {
        None
    } else {
        Some(url.to_string())
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

    fn ctx_default(page: &Page) -> MethodContext<'_> {
        MethodContext {
            page,
            url: "https://amp.example.eu/article",
            original_url: "https://amp.example.eu/article",
            flags: CanonicalFlags::default(),
        }
    }

    fn ctx_no_mr(page: &Page) -> MethodContext<'_> {
        MethodContext {
            page,
            url: "https://amp.example.eu/article",
            original_url: "https://amp.example.eu/article",
            flags: CanonicalFlags {
                use_mr: false,
                ..CanonicalFlags::default()
            },
        }
    }

    #[test]
    fn finds_meta_refresh_url() {
        let page = page_with(
            r#"<html><head><meta http-equiv="refresh" content="0; url=https://example.eu/article"></head></html>"#,
        );
        let r = find(&ctx_default(&page));
        assert_eq!(r, vec!["https://example.eu/article"]);
    }

    #[test]
    fn ignores_other_meta_tags() {
        let page = page_with(
            r#"<html><head>
                <meta charset="utf-8">
                <meta name="description" content="url=https://wrong.com">
                <meta http-equiv="refresh" content="5; url=https://example.eu/article">
            </head></html>"#,
        );
        let r = find(&ctx_default(&page));
        assert_eq!(r, vec!["https://example.eu/article"]);
    }

    #[test]
    fn returns_empty_when_use_mr_disabled() {
        let page =
            page_with(r#"<meta http-equiv="refresh" content="0; url=https://example.eu/x">"#);
        assert!(find(&ctx_no_mr(&page)).is_empty());
    }

    #[test]
    fn returns_empty_when_no_meta_refresh() {
        let page = page_with(r#"<html><head><title>x</title></head></html>"#);
        assert!(find(&ctx_default(&page)).is_empty());
    }

    #[test]
    fn returns_empty_when_content_has_no_url_keyword() {
        let page = page_with(r#"<meta http-equiv="refresh" content="5">"#);
        assert!(find(&ctx_default(&page)).is_empty());
    }

    #[test]
    fn resolves_relative_redirect_url() {
        let page = page_with(r#"<meta http-equiv="refresh" content="0; url=/article">"#);
        let r = find(&ctx_default(&page));
        assert_eq!(r, vec!["https://amp.example.eu/article"]);
    }
}
