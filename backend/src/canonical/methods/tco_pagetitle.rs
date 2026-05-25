//! `TCO_PAGETITLE` method — Twitter shortlinks with `amp=1`.
//!
//! Twitter's `t.co` shortlinks accept `?amp=1` to return a tiny interstitial
//! page where the `<title>` contains the destination URL. This method reads
//! that title.
//!
//! Trigger condition: current URL contains `https://t.co` AND `amp=1`.
//!
//! Ports the `TCO_PAGETITLE` branch of
//! `archive/helpers/canonical_methods.py:66-70`.

use super::MethodContext;

pub fn find(ctx: &MethodContext<'_>) -> Vec<String> {
    if !ctx.url.contains("https://t.co") || !ctx.url.contains("amp=1") {
        return Vec::new();
    }

    let title = ctx.page.title.trim();
    if title.is_empty() || title == "Error: Title not found" {
        return Vec::new();
    }

    vec![title.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::Page;
    use crate::canonical::methods::CanonicalFlags;

    fn page_with(title: &str, url: &str) -> Page {
        Page {
            current_url: url.into(),
            status_code: 200,
            title: title.into(),
            html: String::new(),
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
    fn skips_non_tco_urls() {
        let url = "https://example.com/x?amp=1";
        let page = page_with("https://target.com/article", url);
        assert!(find(&ctx(&page, url)).is_empty());
    }

    #[test]
    fn skips_tco_without_amp_query() {
        let url = "https://t.co/abc123";
        let page = page_with("https://target.com/article", url);
        assert!(find(&ctx(&page, url)).is_empty());
    }

    #[test]
    fn returns_title_as_canonical_for_tco_amp() {
        let url = "https://t.co/abc123?amp=1";
        let page = page_with("https://target.com/article", url);
        let r = find(&ctx(&page, url));
        assert_eq!(r, vec!["https://target.com/article"]);
    }

    #[test]
    fn skips_when_title_is_the_fallback_marker() {
        let url = "https://t.co/abc123?amp=1";
        let page = page_with("Error: Title not found", url);
        assert!(find(&ctx(&page, url)).is_empty());
    }

    #[test]
    fn skips_when_title_is_empty() {
        let url = "https://t.co/abc123?amp=1";
        let page = page_with("", url);
        assert!(find(&ctx(&page, url)).is_empty());
    }
}
