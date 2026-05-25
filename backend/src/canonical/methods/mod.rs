//! Canonical-finding methods.
//!
//! Each method gets its own file under `methods/`. The 11 methods correspond
//! 1:1 to the variants of [`crate::models::CanonicalType`], in priority order.
//!
//! Ports `archive/helpers/canonical_methods.py:get_canonical_with_soup` —
//! the Python version dispatched on `meta.type` via `if/elif`; here each
//! method is its own function and [`try_method`] dispatches.

use regex::Regex;
use scraper::{Html, Selector};
use url::Url;

use crate::canonical::Page;
use crate::models::CanonicalType;

pub mod bing_original;
pub mod canurl;
pub mod google_js;
pub mod google_manual;
pub mod og_url;
pub mod rel;

/// Per-request canonical-finding configuration. Ports the `use_db`/`use_gac`/
/// `use_mr` flags from `archive/helpers/utils.py:get_canonicals`.
///
/// These flags are progressively *disabled* during an iteration: once any
/// method finds a non-AMP canonical, the resource-heavy methods (DB,
/// guess-and-check, meta-refresh) get turned off for the rest of the depth
/// loop. The legacy bot did this to bound work-per-request.
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

/// Inputs every canonical method needs.
///
/// `page` is the freshly-fetched HTML for `url`. `original_url` is the URL
/// the API caller sent (used for similarity scoring); during the depth-loop
/// `url` may differ from `original_url` because we follow an AMP chain.
pub struct MethodContext<'a> {
    pub page: &'a Page,
    pub url: &'a str,
    pub original_url: &'a str,
    pub flags: CanonicalFlags,
}

impl MethodContext<'_> {
    /// Cached parse of [`Self::page`]'s HTML. Each method that needs a
    /// document calls this once.
    pub fn parsed_html(&self) -> Html {
        self.page.parse()
    }
}

/// Try one canonical-finding method against the context.
///
/// Returns the candidate canonical URL(s) the method discovered, **without**
/// the legacy bot's downstream validation pass (is-amp / similarity / domain
/// extraction). Validation lives in the orchestration loop so each method
/// stays focused on extraction.
///
/// Returns `None` (or an empty vec) if the method didn't apply (e.g. a
/// Google-specific method on a non-Google URL, or a gated method whose flag
/// is off).
pub fn try_method(method: CanonicalType, ctx: &MethodContext<'_>) -> Vec<String> {
    match method {
        CanonicalType::Rel => rel::find(ctx),
        CanonicalType::Canurl => canurl::find(ctx),
        CanonicalType::OgUrl => og_url::find(ctx),
        CanonicalType::GoogleManualRedirect => google_manual::find(ctx),
        CanonicalType::GoogleJsRedirect => google_js::find(ctx),
        CanonicalType::BingOriginalUrl => bing_original::find(ctx),
        // The remaining 5 methods land in subsequent commits — schema/tco/
        // meta-refresh (M2.5c), guess-and-check (M2.6 — needs readability),
        // database (M3 — needs sqlx).
        CanonicalType::SchemaMainentity
        | CanonicalType::TcoPagetitle
        | CanonicalType::MetaRedirect
        | CanonicalType::GuessAndCheck
        | CanonicalType::Database => Vec::new(),
    }
}

/// Search every inline `<script>` (i.e. `<script>` without a `src` attribute,
/// matching the Python `find_all("script", {"src": False})`) for `pattern`
/// and return capture group 1.
///
/// Ports `archive/helpers/canonical_methods.py:get_can_url_with_regex`.
/// `\/` is unescaped to `/` — some scripts emit JSON-encoded URLs like
/// `https:\/\/example.com\/`.
pub(crate) fn find_in_inline_scripts(html: &Html, pattern: &Regex) -> Option<String> {
    static SCRIPT: std::sync::LazyLock<Selector> =
        std::sync::LazyLock::new(|| Selector::parse("script:not([src])").expect("script selector"));

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

/// Resolve a candidate URL from an HTML attribute against the base URL.
///
/// Ports `archive/helpers/canonical_methods.py:get_can_urls_by_tags` —
/// rewrites `//host/path` and `/path` references into absolute URLs using
/// the source URL's scheme + authority. The `url` crate's `Url::join`
/// handles every case (protocol-relative, root-relative, fully-relative,
/// already-absolute) in one call.
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
            resolve_against("https://example.com/page", "//cdn.example.com/img").as_deref(),
            Some("https://cdn.example.com/img")
        );
    }

    #[test]
    fn resolve_root_relative() {
        assert_eq!(
            resolve_against("https://example.com/page", "/other/path").as_deref(),
            Some("https://example.com/other/path")
        );
    }

    #[test]
    fn resolve_absolute_passes_through() {
        assert_eq!(
            resolve_against("https://example.com/page", "https://other.example/x").as_deref(),
            Some("https://other.example/x")
        );
    }

    #[test]
    fn resolve_empty_returns_none() {
        assert!(resolve_against("https://example.com", "").is_none());
    }

    #[test]
    fn resolve_with_invalid_base_returns_none() {
        assert!(resolve_against("not a url", "/foo").is_none());
    }
}
