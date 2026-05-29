//! `GET /api/v1/convert` — legacy GET endpoint. Deprecated.
//!
//! Kept alive for existing third-party integrations. New callers should use
//! [`super::convert`] (`POST /api/v2/convert`, JSON in / JSON out, camelCase).
//!
//! v1 contract changes locked in for v7:
//!
//! - `gc` is accepted but silently ignored — legacy `run_api` never read it.
//! - 560 + 561 collapse to 200 + null canonical (web-standard shape).
//! - `?r=true` returns a 303 redirect to the chosen canonical (or falls
//!   through to the JSON response if no canonical was found).
//! - `entry_type` is always [`EntryType::Api`] on v1. Callers that need to
//!   annotate the origin use [`super::convert`] instead, where it's a
//!   first-class body field.

// File-local allow: the Legacy* schema mirrors at the bottom of this file
// carry `#[deprecated]` so utoipa emits `deprecated: true` in the OpenAPI
// spec (Scalar uses that for the "Deprecated" badge). The `#[derive(ToSchema)]`
// expansion + the `body = Vec<LegacyLink>` path annotation count as uses of
// those deprecated items inside this file. Suppress here so the spec assembly
// compiles cleanly — the deprecation still applies to any external caller
// (only `mod.rs` references them, with its own file-local allow).
#![allow(deprecated)]

use axum::{
    Json,
    extract::State,
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
};
use serde_json::json;

use crate::canonical::{Database, HttpFetcher, PageSource, PgDatabase};
use crate::models::{CanonicalType, EntryType};
use crate::state::AppState;

use super::convert_engine::{ConvertInput, ConvertOutcome, convert_inner};
use super::query_parser::{self, ParseError};

/// HTTP entry point. Unwraps state + URI; the actual dispatch logic lives
/// in [`legacy_dispatch`] so integration tests can call it without
/// constructing Axum extractors.
#[utoipa::path(
    get,
    path = "/api/v1/convert",
    tag = "convert",
    summary = "Convert AMP URL (legacy v1)",
    description = "Resolve one or more AMP URLs passed in the `q` query string and return their canonicals. Legacy GET endpoint kept alive for existing third-party integrations — new clients should use `POST /api/v2/convert`.",
    params(
        ("q" = String, Query, description = "URL or text containing URLs. Required.", example = json!("https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/amp/")),
        ("gac" = Option<bool>, Query, description = "Use guess-and-check fallback. Default true."),
        ("md" = Option<u32>, Query, description = "Max recursion depth chasing AMP chains. Default 3."),
        ("gc" = Option<bool>, Query, description = "Accepted for legacy parity but silently ignored."),
        ("r" = Option<bool>, Query, description = "If true, 303-redirect to the canonical instead of returning JSON."),
    ),
    responses(
        (
            status = 200,
            description = "Array of resolved Link objects, snake_case",
            body = Vec<LegacyLink>,
            example = json!([{
                "amp_canonical": null,
                "canonical": {
                    "domain": "electrek.co",
                    "is_alt": false,
                    "is_amp": false,
                    "is_cached": false,
                    "is_valid": true,
                    "type": "REL",
                    "url": "https://electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/",
                    "url_similarity": null
                },
                "canonicals": [{
                    "domain": "electrek.co",
                    "is_alt": false,
                    "is_amp": false,
                    "is_cached": false,
                    "is_valid": true,
                    "type": "REL",
                    "url": "https://electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/",
                    "url_similarity": null
                }],
                "origin": {
                    "domain": "google.com",
                    "is_amp": true,
                    "is_cached": true,
                    "is_valid": true,
                    "url": "https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/amp/"
                }
            }])
        ),
        (status = 303, description = "Redirect to canonical (only when ?r=true and a canonical was found)"),
        (status = 400, description = "Missing required `q` parameter", body = crate::routes::error::LegacyErrorResponse),
        (status = 406, description = "No AMP URL detected in the input", body = crate::routes::error::LegacyErrorResponse),
    )
)]
#[deprecated = "Use POST /api/v2/convert"]
pub async fn handler(State(state): State<AppState>, uri: Uri) -> Response {
    legacy_dispatch(&state.fetcher, &state.db, uri.query().unwrap_or("")).await
}

/// Parse + render for the v1 contract. Generic over the trait bounds so
/// tests can supply mocks.
pub async fn legacy_dispatch<P, D>(fetcher: &P, db: &D, raw_query: &str) -> Response
where
    P: PageSource,
    D: Database,
{
    let params = match query_parser::parse(raw_query) {
        Ok(p) => p,
        Err(ParseError::MissingQ) => return missing_q_response(),
    };
    let input = ConvertInput {
        q: params.q,
        use_gac: params.use_gac,
        max_depth: params.max_depth,
        redirect: params.redirect,
        entry_type: EntryType::Api,
        api_version: 1,
    };
    legacy_render(convert_inner(fetcher, db, input).await)
}

/// Render a v1 outcome → snake_case JSON `Response`. Matches the legacy
/// `/api/v1/convert` shape byte-for-byte. Public so integration tests can
/// drive the full v1 stack (engine + render) without going through Axum.
pub fn legacy_render(outcome: ConvertOutcome) -> Response {
    match outcome {
        ConvertOutcome::Resolved(links) => (StatusCode::OK, Json(links)).into_response(),
        ConvertOutcome::Redirect(target) => Redirect::to(&target).into_response(),
        ConvertOutcome::NoAmp => no_amp_response(),
    }
}

fn missing_q_response() -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "api_error_required_field_missing",
        "Error: No query field provided. Please specify a query (q=)",
    )
}

fn no_amp_response() -> Response {
    error_response(
        StatusCode::NOT_ACCEPTABLE,
        "error_no_amp",
        "Error: Entry doesn't meet criteria (no AMP link detected)",
    )
}

fn error_response(status: StatusCode, result_code: &str, message: &str) -> Response {
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

// ---------------------------------------------------------------------------
// Deprecated v1 schema mirrors. snake_case fields (matching v1's wire
// format). Same semantics as the v2 mirrors in [`super::convert`] — only
// the field naming convention differs.

/// **Deprecated.** One resolved URL. v1 returns an array of these.
#[allow(dead_code)]
#[deprecated = "Use POST /api/v2/convert (Link)"]
#[derive(utoipa::ToSchema)]
#[schema(deprecated)]
pub struct LegacyLink {
    /// When the only canonicals found are themselves AMP and the origin was
    /// a cached AMP URL, this surfaces the AMP canonical so callers get
    /// *something* better than the cached form. `null` otherwise.
    pub amp_canonical: Option<LegacyCanonical>,
    /// The best non-AMP canonical picked from `canonicals`. `null` when no
    /// non-AMP canonical could be reached.
    pub canonical: Option<LegacyCanonical>,
    /// Every candidate canonical found, sorted by similarity descending.
    pub canonicals: Vec<LegacyCanonical>,
    /// The URL as received from the caller, plus parsed metadata.
    pub origin: LegacyUrlMeta,
}

/// **Deprecated.** A discovered canonical URL plus metadata about how it
/// was found.
#[allow(dead_code)]
#[deprecated = "Use POST /api/v2/convert (Canonical)"]
#[derive(utoipa::ToSchema)]
#[schema(deprecated)]
pub struct LegacyCanonical {
    /// Parsed host (e.g. `"electrek.co"`). `null` when the URL couldn't be
    /// parsed.
    pub domain: Option<String>,
    /// `true` when this is an alternative canonical (e.g. a syndicated
    /// copy) rather than the primary one returned in `canonical`.
    pub is_alt: bool,
    /// `true` when the URL still matches AMP patterns, `false` once the
    /// resolver has confirmed it's non-AMP. `null` when undetermined.
    pub is_amp: Option<bool>,
    /// `true` when the URL is a cached AMP variant served by an AMP cache
    /// (e.g. `google.com/amp/s/...`, `cdn.ampproject.org/...`).
    pub is_cached: Option<bool>,
    /// `true` when the URL parses as a valid HTTP(S) URL.
    pub is_valid: Option<bool>,
    /// Which canonical-finding method produced this candidate.
    #[schema(rename = "type")]
    pub type_: Option<CanonicalType>,
    /// The resolved canonical URL.
    pub url: Option<String>,
    /// Article-text similarity score (0–1) against the origin. Only set
    /// when the candidate came from the `GUESS_AND_CHECK` method.
    pub url_similarity: Option<f64>,
}

/// **Deprecated.** Parsed metadata about an input URL.
#[allow(dead_code)]
#[deprecated = "Use POST /api/v2/convert (UrlMeta)"]
#[derive(utoipa::ToSchema)]
#[schema(deprecated)]
pub struct LegacyUrlMeta {
    /// Parsed host (e.g. `"google.com"`).
    pub domain: Option<String>,
    /// `true` when the URL matches AMP patterns.
    pub is_amp: Option<bool>,
    /// `true` when the URL is a cached AMP variant served by an AMP cache.
    pub is_cached: Option<bool>,
    /// `true` when the URL parses as a valid HTTP(S) URL.
    pub is_valid: Option<bool>,
    /// The URL as received from the caller.
    pub url: Option<String>,
}
