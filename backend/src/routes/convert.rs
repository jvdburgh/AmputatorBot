//! `GET|POST /api/v1/convert` — the load-bearing public API endpoint.
//!
//! Ports `archive/AmputatorBotCom/main.py:run_api` (161+) with the v7
//! contract changes locked in:
//!
//! - `gc` is accepted but silently ignored — the legacy `run_api` never
//!   read it (only the HTML-form route did), so "port verbatim" means no
//!   API wiring in M3. Devvit (M5) reuses [`crate::reply`] directly.
//! - Custom status codes 560 ("no canonicals") and 561 ("problematic
//!   domain") are collapsed to **200** with a null `canonical`. Web-standard
//!   shape; clients differentiate via the response body if they care.
//! - `X-AmputatorBot-Entry-Type` header lets Devvit annotate where a
//!   resolution came from (`COMMENT`, `SUBMISSION`, `MENTION`, `ONLINE`).
//!   Missing or unrecognized → defaults to `API` with a warn log.
//! - `r=true` returns a 303 redirect to the chosen canonical (or falls
//!   through to the JSON response if no canonical was found).
//!
//! Status-code map after the v7 contract pass:
//!
//! | Code | When |
//! |------|------|
//! | 200  | AMP detected and `resolve()` ran. `canonical` may be null. |
//! | 303  | `r=true` and at least one canonical (or amp_canonical) found. |
//! | 400  | `q` is missing or empty. |
//! | 406  | Body parsed, but no URL inside was AMP. |
//! | 500  | Anything thrown by `resolve()` that bubbled past the engine's `Result`-swallowing. |

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
};
use serde_json::json;

use crate::canonical::database::Resolution;
use crate::canonical::{self, Database, HttpFetcher, PageSource, PgDatabase, ResolveOpts};
use crate::models::{EntryType, Link};
use crate::state::AppState;

use super::query_parser::{self, ParseError};

/// HTTP entry point. Threads request shape into the generic [`convert_inner`]
/// implementation, which takes traits so tests can drive it with mocks.
pub async fn handler(State(state): State<AppState>, headers: HeaderMap, uri: Uri) -> Response {
    let raw_query = uri.query().unwrap_or("");
    convert_inner(&state.fetcher, &state.db, raw_query, &headers).await
}

/// Generic core, separated from Axum so unit tests can pass `MockPageSource`
/// + `MockDatabase` without spinning up an HTTP server.
pub async fn convert_inner<P, D>(
    fetcher: &P,
    db: &D,
    raw_query: &str,
    headers: &HeaderMap,
) -> Response
where
    P: PageSource,
    D: Database,
{
    let params = match query_parser::parse(raw_query) {
        Ok(p) => p,
        Err(ParseError::MissingQ) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "api_error_required_field_missing",
                "Error: No query field provided. Please specify a query (q=)",
            );
        }
    };

    let entry_type = entry_type_from_headers(headers);

    // Match the legacy `check_criteria(mustBeAMP=True)` precheck: if the
    // body parses to zero AMP URLs we never reach `resolve()`. Returning
    // 406 here keeps DB writes scoped to "we actually tried" cases, mirroring
    // the legacy `save_entry` placement after the criteria branch.
    let urls = canonical::extract_urls(&params.q);
    let any_amp = urls.iter().any(|u| canonical::is_amp_url(u));
    if !any_amp {
        return error_response(
            StatusCode::NOT_ACCEPTABLE,
            "error_no_amp",
            "Error: Entry doesn't meet criteria (no AMP link detected)",
        );
    }

    let opts = ResolveOpts {
        use_gac: params.use_gac,
        max_depth: params.max_depth,
        ..ResolveOpts::default()
    };

    let mut links: Vec<Link> = Vec::with_capacity(urls.len());
    for url in &urls {
        let link = canonical::resolve(fetcher, db, url, opts).await;

        // Legacy `save_entry` writes one row per link whose origin is AMP
        // (regardless of whether a canonical was found). Mirror exactly:
        // a row with `canonical_url = NULL` is meaningful — it tells the
        // next run "we tried this URL and got nothing."
        if link.origin.is_amp == Some(true) {
            let resolution = Resolution {
                entry_type,
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

    if params.redirect
        && let Some(target) = links.iter().find_map(redirect_target)
    {
        return Redirect::to(target).into_response();
    }

    (StatusCode::OK, Json(links)).into_response()
}

/// Pick the URL we'd redirect to: prefer the non-AMP `canonical`, fall back
/// to `amp_canonical` (which is set when the origin was a cached AMP URL
/// and the resolver couldn't reach a non-AMP version). Matches the legacy
/// fall-through in `run_amputatorbotcom` (`AmputatorBotCom/main.py:67-75`).
fn redirect_target(link: &Link) -> Option<&str> {
    link.canonical
        .as_ref()
        .and_then(|c| c.url.as_deref())
        .or_else(|| link.amp_canonical.as_ref().and_then(|c| c.url.as_deref()))
}

const ENTRY_TYPE_HEADER: &str = "X-AmputatorBot-Entry-Type";

/// Parse the [`ENTRY_TYPE_HEADER`] header. Missing → [`EntryType::Api`]
/// silently. Invalid → [`EntryType::Api`] with a warn-level log entry so
/// Devvit-side typos surface in Scaleway Cockpit without breaking the call.
fn entry_type_from_headers(headers: &HeaderMap) -> EntryType {
    let Some(raw) = headers.get(ENTRY_TYPE_HEADER) else {
        return EntryType::Api;
    };
    let Ok(value) = raw.to_str() else {
        tracing::warn!(
            header = ENTRY_TYPE_HEADER,
            "header value is not valid ASCII; defaulting to API"
        );
        return EntryType::Api;
    };
    match value.trim().to_ascii_uppercase().as_str() {
        "API" => EntryType::Api,
        "COMMENT" => EntryType::Comment,
        "SUBMISSION" => EntryType::Submission,
        "MENTION" => EntryType::Mention,
        "ONLINE" => EntryType::Online,
        other => {
            tracing::warn!(
                header = ENTRY_TYPE_HEADER,
                value = other,
                "unknown entry-type header value; defaulting to API"
            );
            EntryType::Api
        }
    }
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

/// Ensure the concrete production types implement the traits the handler
/// generic-bounds against. Catches refactor regressions at compile time
/// without needing a runtime test.
#[allow(dead_code)]
fn _assert_app_state_satisfies_bounds(state: AppState) {
    fn takes<P: PageSource, D: Database>(_p: &P, _d: &D) {}
    takes::<HttpFetcher, PgDatabase>(&state.fetcher, &state.db);
}
