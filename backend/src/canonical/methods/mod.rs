//! Canonical-finding methods. One file per variant of
//! [`crate::models::CanonicalType`]; [`try_method`] dispatches.

use regex::Regex;
use scraper::{Html, Selector};
use url::Url;

use crate::canonical::Page;
use crate::models::CanonicalType;

pub mod bing_original;
pub mod canurl;
pub mod database;
pub mod google_js;
pub mod google_manual;
pub mod guess_and_check;
pub mod meta_redirect;
pub mod og_url;
pub mod rel;
pub mod schema_mainentity;
pub mod tco_pagetitle;

/// Per-request gates on the resource-heavy methods (`DATABASE`,
/// `GUESS_AND_CHECK`, `META_REDIRECT`). The resolver flips these off mid-
/// iteration once a non-AMP canonical surfaces, to bound work per request.
#[derive(Debug, Clone, Copy)]
pub struct CanonicalFlags {
    pub use_db: bool,
    pub use_gac: bool,
    pub use_mr: bool,
}

impl Default for CanonicalFlags {
    fn default() -> Self {
        Self {
            use_db: true,
            use_gac: true,
            use_mr: true,
        }
    }
}

/// Inputs every canonical method needs. `url` is the URL the resolver is
/// processing at this iteration; `original_url` is the caller's input
/// (preserved across iterations).
pub struct MethodContext<'a> {
    pub page: &'a Page,
    pub url: &'a str,
    pub original_url: &'a str,
    pub flags: CanonicalFlags,
}

impl MethodContext<'_> {
    pub fn parsed_html(&self) -> Html {
        self.page.parse()
    }
}

/// Dispatch the synchronous methods. `DATABASE` is async (sqlx) and routes
/// through the resolver's own awaiting code path.
pub fn try_method(method: CanonicalType, ctx: &MethodContext<'_>) -> Vec<String> {
    match method {
        CanonicalType::Rel => rel::find(ctx),
        CanonicalType::Canurl => canurl::find(ctx),
        CanonicalType::OgUrl => og_url::find(ctx),
        CanonicalType::GoogleManualRedirect => google_manual::find(ctx),
        CanonicalType::GoogleJsRedirect => google_js::find(ctx),
        CanonicalType::BingOriginalUrl => bing_original::find(ctx),
        CanonicalType::SchemaMainentity => schema_mainentity::find(ctx),
        CanonicalType::TcoPagetitle => tco_pagetitle::find(ctx),
        CanonicalType::MetaRedirect => meta_redirect::find(ctx),
        CanonicalType::GuessAndCheck => guess_and_check::find(ctx),
        CanonicalType::Database => Vec::new(),
    }
}

/// Run `pattern` against every inline `<script>` block; return capture
/// group 1 with `\/` unescaped to `/` for JSON-encoded URLs.
pub(crate) fn find_in_inline_scripts(html: &Html, pattern: &Regex) -> Option<String> {
    static SCRIPT: std::sync::LazyLock<Selector> =
        std::sync::LazyLock::new(|| Selector::parse("script:not([src])").unwrap());

    for script in html.select(&SCRIPT) {
        let text = script.text().collect::<String>();
        if let Some(caps) = pattern.captures(&text)
            && let Some(m) = caps.get(1)
        {
            return Some(m.as_str().replace("\\/", "/"));
        }
    }
    None
}

/// Resolve a candidate URL against the base. Handles protocol-relative,
/// root-relative, fully-relative, and already-absolute inputs via `Url::join`.
pub(crate) fn resolve_against(base: &str, candidate: &str) -> Option<String> {
    if candidate.is_empty() {
        return None;
    }
    let base = Url::parse(base).ok()?;
    let resolved = base.join(candidate).ok()?;
    Some(resolved.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_protocol_relative() {
        assert_eq!(
            resolve_against("https://example.eu/page", "//cdn.example.eu/img").as_deref(),
            Some("https://cdn.example.eu/img")
        );
    }

    #[test]
    fn resolve_root_relative() {
        assert_eq!(
            resolve_against("https://example.eu/page", "/other/path").as_deref(),
            Some("https://example.eu/other/path")
        );
    }

    #[test]
    fn resolve_absolute_passes_through() {
        assert_eq!(
            resolve_against("https://example.eu/page", "https://other.example/x").as_deref(),
            Some("https://other.example/x")
        );
    }

    #[test]
    fn resolve_empty_returns_none() {
        assert!(resolve_against("https://example.eu", "").is_none());
    }

    #[test]
    fn resolve_with_invalid_base_returns_none() {
        assert!(resolve_against("not a url", "/foo").is_none());
    }
}
