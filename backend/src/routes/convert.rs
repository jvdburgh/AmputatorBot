//! `/api/v1/convert` (this file) and the shared convert-engine internals
//! ([`ConvertInput`], [`ConvertOutcome`], [`convert_inner`]) that the v2
//! handler also calls.
//!
//! The split: parsing the request and rendering the response are per-version;
//! the resolve loop in the middle is shared. v2 (camelCase, JSON body) and
//! v1 (snake_case, query string) reach the same engine via [`convert_inner`]
//! and render its [`ConvertOutcome`] each in their own dialect.
//!
//! v1 contract changes locked in for v7:
//!
//! - `gc` is accepted but silently ignored — legacy `run_api` never read it.
//! - 560 + 561 collapse to 200 + null canonical (web-standard shape).
//! - `?r=true` returns a 303 redirect to the chosen canonical (or falls
//!   through to the JSON response if no canonical was found).
//! - `entry_type` is always [`EntryType::Api`] on v1. Callers that need to
//!   annotate the origin use [`super::convert_v2`] instead, where it's a
//!   first-class body field.

use axum::{
    Json,
    extract::State,
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
};
use serde_json::json;

use crate::canonical::database::Resolution;
use crate::canonical::{self, Database, HttpFetcher, PageSource, PgDatabase, ResolveOpts};
use crate::models::{EntryType, Link};
use crate::state::AppState;

use super::query_parser::{self, ParseError};

/// Normalized input both v1 and v2 produce before handing off to
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

/// v1 HTTP entry point. Just unwraps state + URI; the actual dispatch logic
/// lives in [`dispatch_v1`] so integration tests can call it without
/// constructing Axum extractors.
#[utoipa::path(
    method(get, post),
    path = "/api/v1/convert",
    tag = "convert",
    params(
        ("q" = String, Query, description = "URL or text containing URLs. Required."),
        ("gac" = Option<bool>, Query, description = "Use guess-and-check fallback. Default true."),
        ("md" = Option<u32>, Query, description = "Max recursion depth chasing AMP chains. Default 3."),
        ("gc" = Option<bool>, Query, description = "Accepted for legacy parity but silently ignored."),
        ("r" = Option<bool>, Query, description = "If true, 303-redirect to the canonical instead of returning JSON."),
    ),
    responses(
        (status = 200, description = "Array of resolved Link objects, snake_case", body = Vec<crate::models::Link>),
        (status = 303, description = "Redirect to canonical (only when ?r=true and a canonical was found)"),
        (status = 400, description = "Missing required `q` parameter", body = crate::routes::error::ErrorResponseV1),
        (status = 406, description = "No AMP URL detected in the input", body = crate::routes::error::ErrorResponseV1),
    )
)]
pub async fn handler(State(state): State<AppState>, uri: Uri) -> Response {
    dispatch_v1(&state.fetcher, &state.db, uri.query().unwrap_or("")).await
}

/// Parse + render for the v1 contract. Generic over the trait bounds so
/// tests can supply mocks.
pub async fn dispatch_v1<P, D>(fetcher: &P, db: &D, raw_query: &str) -> Response
where
    P: PageSource,
    D: Database,
{
    let params = match query_parser::parse(raw_query) {
        Ok(p) => p,
        Err(ParseError::MissingQ) => return missing_q_response_v1(),
    };
    let input = ConvertInput {
        q: params.q,
        use_gac: params.use_gac,
        max_depth: params.max_depth,
        redirect: params.redirect,
        entry_type: EntryType::Api,
        api_version: 1,
    };
    render_v1(convert_inner(fetcher, db, input).await)
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
            let resolution = Resolution {
                entry_type: input.entry_type,
                api_version: input.api_version,
                original_url: url,
                canonical_url: link.canonical.as_ref().and_then(|c| c.url.as_deref()),
                canonical_type: link.canonical.as_ref().and_then(|c| c.type_),
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
pub(super) fn redirect_target(link: &Link) -> Option<&str> {
    link.canonical
        .as_ref()
        .and_then(|c| c.url.as_deref())
        .or_else(|| link.amp_canonical.as_ref().and_then(|c| c.url.as_deref()))
}

/// Render a v1 outcome → snake_case JSON `Response`. Matches the legacy
/// `/api/v1/convert` shape byte-for-byte. Public so integration tests can
/// drive the full v1 stack (engine + render) without going through Axum.
pub fn render_v1(outcome: ConvertOutcome) -> Response {
    match outcome {
        ConvertOutcome::Resolved(links) => (StatusCode::OK, Json(links)).into_response(),
        ConvertOutcome::Redirect(target) => Redirect::to(&target).into_response(),
        ConvertOutcome::NoAmp => no_amp_response_v1(),
    }
}

fn missing_q_response_v1() -> Response {
    error_response_v1(
        StatusCode::BAD_REQUEST,
        "api_error_required_field_missing",
        "Error: No query field provided. Please specify a query (q=)",
    )
}

fn no_amp_response_v1() -> Response {
    error_response_v1(
        StatusCode::NOT_ACCEPTABLE,
        "error_no_amp",
        "Error: Entry doesn't meet criteria (no AMP link detected)",
    )
}

fn error_response_v1(status: StatusCode, result_code: &str, message: &str) -> Response {
    (
        status,
        Json(json!({
            "error_message": message,
            "result_code": result_code,
        })),
    )
        .into_response()
}

/// Compile-time check that production types satisfy the trait bounds the
/// generic handler requires. Cheaper than a runtime smoke test.
#[allow(dead_code)]
fn _assert_app_state_satisfies_bounds(state: AppState) {
    fn takes<P: PageSource, D: Database>(_p: &P, _d: &D) {}
    takes::<HttpFetcher, PgDatabase>(&state.fetcher, &state.db);
}
