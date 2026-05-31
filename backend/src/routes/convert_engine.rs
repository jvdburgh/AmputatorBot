//! Shared resolve-loop engine called by both convert handlers.
//!
//! HTTP-surface concerns (snake_case vs. camelCase, query string vs. JSON
//! body, response envelope) live per-version in [`super::convert`] (v2,
//! default) and [`super::legacy_convert`] (v1). The engine itself is
//! version-agnostic — both handlers normalize their inputs into
//! [`ConvertInput`] and render the resulting [`ConvertOutcome`] in their
//! own dialect.

use crate::canonical::database::Resolution;
use crate::canonical::{self, Database, PageSource, ResolveOpts};
use crate::models::{EntryType, Link};

/// Normalized input both handlers produce before handing off to
/// [`convert_inner`]. Decoupling the input shape from the resolve loop keeps
/// the two endpoints honest: same engine, different surface.
#[derive(Debug, Clone)]
pub struct ConvertInput {
    pub q: String,
    pub use_gac: bool,
    pub max_depth: u32,
    pub redirect: bool,
    pub entry_type: EntryType,
    /// 1 for v1, 2 for v2. Persisted to `links.api_version` so the analytical
    /// "who's using v2?" question is a single GROUP BY.
    pub api_version: i16,
}

/// What [`convert_inner`] decided to do. Stays version-agnostic so each
/// handler renders its own JSON shape (snake_case for v1, camelCase for v2).
#[derive(Debug)]
pub enum ConvertOutcome {
    /// 200 OK with the array of resolved links.
    Resolved(Vec<Link>),
    /// 303 redirect to this URL.
    Redirect(String),
    /// 406 — no AMP URL detected in `input.q`.
    NoAmp,
}

/// Shared engine. Resolves every URL in `input.q`, persists results, and
/// returns a [`ConvertOutcome`]. Generic over the trait bounds so unit tests
/// can drive it with mocks.
pub async fn convert_inner<P, D>(fetcher: &P, db: &D, input: ConvertInput) -> ConvertOutcome
where
    P: PageSource,
    D: Database,
{
    // Match the legacy `check_criteria(mustBeAMP=True)` precheck: if the body
    // parses to zero AMP URLs we never reach `resolve()`. Returning early here
    // keeps DB writes scoped to "we actually tried" cases.
    let urls = canonical::extract_urls(&input.q);
    if !urls.iter().any(|u| canonical::is_amp_url(u)) {
        return ConvertOutcome::NoAmp;
    }

    let opts = ResolveOpts {
        use_gac: input.use_gac,
        max_depth: input.max_depth,
        ..ResolveOpts::default()
    };

    let mut links: Vec<Link> = Vec::with_capacity(urls.len());
    for url in &urls {
        let link = canonical::resolve(fetcher, db, url, opts).await;

        // Legacy `save_entry` writes one row per link whose origin is AMP
        // (regardless of whether a canonical was found). A row with
        // `canonical_url = NULL` is meaningful: it tells the next run "we
        // tried this URL and got nothing."
        if link.origin.is_amp == Some(true) {
            let chosen = link.canonical.as_ref();
            let resolution = Resolution {
                entry_type: input.entry_type,
                api_version: input.api_version,
                original_url: url,
                canonical_url: chosen.and_then(|c| c.url.as_deref()),
                canonical_type: chosen.and_then(|c| c.type_),
                url_similarity: chosen.and_then(|c| c.url_similarity),
                article_similarity: chosen.and_then(|c| c.article_similarity),
                confidence_score: chosen.and_then(|c| c.confidence_score),
                confidence_level: chosen.and_then(|c| c.confidence_level),
            };
            if let Err(e) = db.record_resolution(resolution).await {
                tracing::warn!(error = ?e, url = %url, "record_resolution failed; continuing");
            }
        }

        links.push(link);
    }

    if input.redirect
        && let Some(target) = links.iter().find_map(redirect_target).map(String::from)
    {
        return ConvertOutcome::Redirect(target);
    }

    ConvertOutcome::Resolved(links)
}

/// Pick the URL we'd redirect to: prefer the non-AMP `canonical`, fall back
/// to `amp_canonical` (set when the origin was a cached AMP URL and the
/// resolver couldn't reach a non-AMP version). Matches the legacy fall-
/// through in `run_amputatorbotcom` (`AmputatorBotCom/main.py:67-75`).
pub fn redirect_target(link: &Link) -> Option<&str> {
    link.canonical
        .as_ref()
        .and_then(|c| c.url.as_deref())
        .or_else(|| link.amp_canonical.as_ref().and_then(|c| c.url.as_deref()))
}
