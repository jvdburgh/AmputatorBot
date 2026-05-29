//! `POST /api/v2/convert` — the modern JSON API.
//!
//! Same engine as v1 (delegates to [`super::convert::convert_inner`]),
//! different surface: JSON body in camelCase, JSON response in camelCase.
//! `entryType` is a first-class body field instead of an HTTP header.
//!
//! Status-code map mirrors v1 except for missing/invalid body fields, which
//! return 400 via Axum's default JSON-rejection path. We don't override
//! Axum's `JsonRejection` — its default error body is fine for a v2 API and
//! callers get a clear "deserialize failed at field X" message.
//!
//! Response shape is the standard [`Link`] tree with keys camelCased
//! recursively. The conversion happens at the edge ([`camelize`]) so no
//! v2-specific wrapper types are needed in `crate::models`.

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::models::EntryType;
use crate::state::AppState;

use super::convert::{ConvertInput, ConvertOutcome, convert_inner};

/// JSON body for `POST /api/v2/convert`.
///
/// `#[serde(deny_unknown_fields)]` keeps the contract tight — typos in
/// field names produce a 400 rather than silently using a default. v2 is
/// the modern API; strict beats lenient.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConvertBodyV2 {
    /// The AMP URL to resolve, or free-form text containing one or more AMP
    /// URLs (the resolver runs the same URL-extraction pass it uses for
    /// Reddit comment bodies, so pasting a chat message or a `praw`-style
    /// blob works the same as pasting a single URL).
    pub query: String,
    #[serde(default = "default_guess_and_check")]
    pub guess_and_check: bool,
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    #[serde(default)]
    pub redirect: bool,
    /// Where the call originated. Optional — when omitted (or `null`) the
    /// resolver defaults to [`EntryType::Api`] so plain API adapters don't
    /// have to set it on every call. Devvit triggers send `"COMMENT"` /
    /// `"SUBMISSION"` / `"MENTION"`; the website sends `"ONLINE"`. Strict
    /// matching: a typo like `"comments"` returns 400.
    #[schema(default = "API")]
    #[serde(default)]
    pub entry_type: Option<EntryType>,
}

fn default_guess_and_check() -> bool {
    true
}
fn default_max_depth() -> u32 {
    3
}

/// v2 HTTP entry point. Unwraps state + JSON body; the actual dispatch
/// logic lives in [`dispatch_v2`] so integration tests can drive it
/// without going through Axum's extractor stack.
#[utoipa::path(
    post,
    path = "/api/v2/convert",
    tag = "convert",
    request_body = ConvertBodyV2,
    responses(
        (status = 200, description = "Array of resolved Link objects, recursively camelCased", body = Vec<crate::models::Link>),
        (status = 303, description = "Redirect to canonical (only when redirect=true and a canonical was found)"),
        (status = 400, description = "Empty `query` field", body = crate::routes::error::ErrorResponseV2),
        (status = 406, description = "No AMP URL detected", body = crate::routes::error::ErrorResponseV2),
        (status = 422, description = "Body failed to deserialize — unknown field, bad casing, or wrong type"),
    )
)]
pub async fn handler(State(state): State<AppState>, Json(body): Json<ConvertBodyV2>) -> Response {
    dispatch_v2(&state.fetcher, &state.db, body).await
}

/// Build + render for the v2 contract. Generic over the trait bounds so
/// tests can supply mocks.
pub async fn dispatch_v2<P, D>(fetcher: &P, db: &D, body: ConvertBodyV2) -> Response
where
    P: crate::canonical::PageSource,
    D: crate::canonical::Database,
{
    if body.query.is_empty() {
        return missing_query_response_v2();
    }
    let input = ConvertInput {
        q: body.query,
        use_gac: body.guess_and_check,
        max_depth: body.max_depth,
        redirect: body.redirect,
        entry_type: body.entry_type.unwrap_or(EntryType::Api),
        api_version: 2,
    };
    render_v2(convert_inner(fetcher, db, input).await)
}

/// Render [`ConvertOutcome`] → camelCase JSON `Response`. The link tree is
/// produced by serde with its native snake_case naming, then transformed
/// recursively at the edge.
fn render_v2(outcome: ConvertOutcome) -> Response {
    match outcome {
        ConvertOutcome::Resolved(links) => {
            let value = serde_json::to_value(&links).expect("Link is always serializable");
            (StatusCode::OK, Json(camelize(value))).into_response()
        }
        ConvertOutcome::Redirect(target) => Redirect::to(&target).into_response(),
        ConvertOutcome::NoAmp => no_amp_response_v2(),
    }
}

fn missing_query_response_v2() -> Response {
    error_response_v2(
        StatusCode::BAD_REQUEST,
        "api_error_required_field_missing",
        "Error: No query field provided. Set `query` in the JSON body.",
    )
}

fn no_amp_response_v2() -> Response {
    error_response_v2(
        StatusCode::NOT_ACCEPTABLE,
        "error_no_amp",
        "Error: Entry doesn't meet criteria (no AMP link detected)",
    )
}

fn error_response_v2(status: StatusCode, result_code: &str, message: &str) -> Response {
    // Error keys are already valid camelCase identifiers; emit directly.
    (
        status,
        Json(json!({
            "errorMessage": message,
            "resultCode": result_code,
        })),
    )
        .into_response()
}

/// Recursively rename object keys from `snake_case` to `camelCase`.
///
/// Used by v2's response renderer so the snake_case [`crate::models`] types
/// (kept that way for v1 compatibility) come out camelCase on the wire.
/// Cost: one Value clone + tree walk per response. Negligible vs. the
/// canonical-finding work that produced the data.
fn camelize(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (snake_to_camel(&k), camelize(v)))
                .collect(),
        ),
        Value::Array(arr) => Value::Array(arr.into_iter().map(camelize).collect()),
        other => other,
    }
}

fn snake_to_camel(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut capitalize = false;
    for c in s.chars() {
        if c == '_' {
            capitalize = true;
        } else if capitalize {
            out.push(c.to_ascii_uppercase());
            capitalize = false;
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_to_camel_basics() {
        assert_eq!(snake_to_camel("is_amp"), "isAmp");
        assert_eq!(snake_to_camel("url_similarity"), "urlSimilarity");
        assert_eq!(snake_to_camel("amp_canonical"), "ampCanonical");
        // Already camel / single word — pass through unchanged.
        assert_eq!(snake_to_camel("origin"), "origin");
        assert_eq!(snake_to_camel("canonical"), "canonical");
    }

    #[test]
    fn snake_to_camel_handles_trailing_underscore() {
        // serde's `#[serde(rename = "type")]` on Canonical.type_ already
        // hides the trailing underscore from JSON output, but be defensive.
        assert_eq!(snake_to_camel("foo_"), "foo");
    }

    #[test]
    fn camelize_walks_nested_objects() {
        let input = json!({
            "amp_canonical": null,
            "canonical": {
                "url_similarity": 0.95,
                "is_amp": false
            },
            "canonicals": [
                { "is_alt": true, "url_similarity": 0.5 }
            ],
            "origin": { "is_amp": true, "is_cached": false }
        });
        let out = camelize(input);
        assert_eq!(out["ampCanonical"], Value::Null);
        assert_eq!(out["canonical"]["urlSimilarity"], 0.95);
        assert_eq!(out["canonical"]["isAmp"], false);
        assert_eq!(out["canonicals"][0]["isAlt"], true);
        assert_eq!(out["origin"]["isCached"], false);
    }
}
