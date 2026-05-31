//! `POST /api/v2/convert` — the default JSON API.
//!
//! JSON body in camelCase, JSON response envelope in camelCase. `entryType`
//! is a first-class body field instead of an HTTP header. Status-code map
//! mirrors v1 except for missing/invalid body fields, which return 400 via
//! Axum's default JSON-rejection path — its default error body is fine and
//! callers get a clear "deserialize failed at field X" message.
//!
//! Response shape on 200 is always an envelope ([`ConvertResponse`]):
//! `{ links: Link[], comment: string | null }`. `comment` is the Reddit-
//! formatted reply markdown — populated when `generateMarkdownComment: true`
//! was set on the request, `null` otherwise. The Devvit bot uses this to
//! get the canonical resolution and the post-ready markdown in a single
//! call; the website's "generate Reddit comment" copy-paste box does the
//! same. Conversion of the inner [`crate::models::Link`] tree to camelCase
//! happens at the edge ([`camelize`]) so no v2-specific wrapper types are
//! needed in `crate::models`.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::models::{CanonicalType, EntryType};
use crate::reply::{BuildReplyOptions, build_reply};
use crate::state::AppState;

use super::convert_engine::{ConvertInput, ConvertOutcome, convert_inner};

/// Header that signals the call's origin. Set internally by the Devvit app
/// (`COMMENT` / `SUBMISSION` / `MENTION`) and the website (`ONLINE`); absent
/// on direct API calls, which default to `API`. Intentionally undocumented
/// in the public API schema so external callers can't fake per-source
/// analytics in the cache.
const ENTRY_TYPE_HEADER: &str = "x-amputatorbot-entry-type";

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConvertBody {
    /// The AMP URL to resolve, or free-form text containing one or more AMP
    /// URLs. The resolver runs the same URL-extraction pass it uses for
    /// Reddit comment bodies, so a chat-style sentence with one or more URLs
    /// works the same as pasting a single URL.
    #[schema(
        example = "https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/amp/"
    )]
    pub query: String,
    /// Fall back to the guess-and-check heuristic — strip AMP keywords from
    /// the URL, fetch it, accept the result when article-text similarity is
    /// high — when no explicit canonical signal is found.
    #[serde(default = "default_guess_and_check")]
    pub guess_and_check: bool,
    /// Maximum recursion depth when chasing AMP-of-AMP chains.
    #[serde(default = "default_max_depth")]
    #[schema(maximum = 5, example = 3)]
    pub max_depth: u32,
    /// Return a 303 redirect to the canonical URL instead of the JSON
    /// envelope.
    #[serde(default)]
    pub redirect: bool,
    /// When `true`, populate `comment` in the response with the
    /// Reddit-formatted reply markdown the bot would post (or that you'd
    /// copy-paste manually). Defaults to `false` since most callers only
    /// want the canonical-resolution results.
    #[serde(default)]
    pub generate_markdown_comment: bool,
    /// Optional per-install footer addendum — rendered inside the reply's
    /// superscript footer as ` | <text>` when `generateMarkdownComment` is
    /// true and `entryType` is a bot variant (`COMMENT` / `SUBMISSION` /
    /// `MENTION`). Ignored for `API` / `ONLINE` entry types.
    #[serde(default)]
    pub custom_footer: Option<String>,
}

/// Envelope returned on 200 OK.
#[derive(utoipa::ToSchema)]
#[schema(rename_all = "camelCase")]
pub struct ConvertResponse {
    /// One entry per URL the resolver saw in `query`. Empty when no AMP
    /// URLs were detected (in which case the request returns 406 instead).
    pub links: Vec<Link>,
    /// Reddit-formatted reply markdown, ready to post or copy-paste. Only
    /// populated when `generateMarkdownComment: true` was set on the
    /// request and at least one AMP URL was resolved; `null` otherwise.
    pub comment: Option<String>,
}

// ---------------------------------------------------------------------------
// camelCase schema mirrors.
//
// Doc-only types — the runtime `crate::models::Link`/`Canonical`/`UrlMeta`
// stay snake_case so v1's serde output is unchanged; v2's response is
// camelCased at the edge by [`camelize`]. The mirrors below exist purely so
// utoipa documents the same camelCase shape that v2 actually emits. Field
// rustdoc here is the per-field description Scalar shows in the docs, so
// keep it user-facing (not implementation talk).

/// One resolved URL. The response emits an array of these, one per URL the
/// resolver detected in `query` — typically a single entry, but a
/// chat-style body with multiple URLs produces multiple entries.
#[allow(dead_code)]
#[derive(utoipa::ToSchema)]
#[schema(rename_all = "camelCase")]
pub struct Link {
    /// When the only canonicals found are themselves AMP and the origin was
    /// a cached AMP URL, this surfaces the AMP canonical so callers get
    /// *something* better than the cached form. `null` otherwise.
    pub amp_canonical: Option<Canonical>,
    /// The best non-AMP canonical picked from `canonicals`. `null` when no
    /// non-AMP canonical could be reached.
    pub canonical: Option<Canonical>,
    /// Every candidate canonical found, sorted by similarity descending.
    /// Often a single entry, sometimes several (e.g. syndicated copies).
    pub canonicals: Vec<Canonical>,
    /// The URL as received from the caller, plus parsed metadata.
    pub origin: UrlMeta,
}

/// A discovered canonical URL plus metadata about how it was found.
#[allow(dead_code)]
#[derive(utoipa::ToSchema)]
#[schema(rename_all = "camelCase")]
pub struct Canonical {
    /// Parsed host (e.g. `"electrek.co"`). `null` when the URL couldn't be
    /// parsed.
    pub domain: Option<String>,
    /// `true` when this is an alternative canonical (e.g. a syndicated copy)
    /// rather than the primary one returned in `canonical`.
    pub is_alt: bool,
    /// `true` when the URL still matches AMP patterns, `false` once the
    /// resolver has confirmed it's non-AMP. `null` when undetermined.
    pub is_amp: Option<bool>,
    /// `true` when the URL is a cached AMP variant served by an AMP cache
    /// (e.g. `google.com/amp/s/...`, `cdn.ampproject.org/...`).
    pub is_cached: Option<bool>,
    /// `true` when the URL parses as a valid HTTP(S) URL.
    pub is_valid: Option<bool>,
    /// Which canonical-finding method produced this candidate. See the
    /// `CanonicalType` schema for the full list (`REL`, `OG_URL`,
    /// `GUESS_AND_CHECK`, etc.).
    #[schema(rename = "type")]
    pub type_: Option<CanonicalType>,
    /// The resolved canonical URL.
    pub url: Option<String>,
    /// Ratcliff-Obershelp string similarity (0–1) between the candidate URL
    /// and the input URL the resolver was working from.
    pub url_similarity: Option<f64>,
    /// Ratcliff-Obershelp similarity (0–1) between the article text extracted
    /// from the origin page and the article text extracted from the candidate
    /// canonical's page. `null` when either side had no extractable article
    /// (e.g. origin was a Google AMP-cache interstitial, or the candidate
    /// fetch was blocked/4xx).
    pub article_similarity: Option<f64>,
    /// Combined confidence score (0–1). Formula: when article similarity is
    /// available, `0.7 * article + 0.2 * method_weight + 0.1 * url`; otherwise
    /// `min(0.6, 0.4 * method_weight + 0.2 * url)` — so a result can never
    /// be `VERIFIED` without an article-content match.
    pub confidence_score: Option<f64>,
    /// Bucketed `confidence_score`: `VERIFIED` (≥ 0.65), `LIKELY` (≥ 0.35),
    /// or `UNCONFIRMED` (< 0.35).
    pub confidence_level: Option<crate::models::ConfidenceLevel>,
}

/// Parsed metadata about an input URL — the shape used by `Link.origin`.
/// All fields are nullable: `null` means the resolver couldn't determine a
/// value (e.g. `domain` when the URL didn't parse). `isValid: false` flags a
/// malformed URL.
#[allow(dead_code)]
#[derive(utoipa::ToSchema)]
#[schema(rename_all = "camelCase")]
pub struct UrlMeta {
    /// Parsed host (e.g. `"google.com"`).
    pub domain: Option<String>,
    /// `true` when the URL matches AMP patterns (path contains `/amp/`,
    /// known AMP-cache hosts, etc.).
    pub is_amp: Option<bool>,
    /// `true` when the URL is a cached AMP variant served by an AMP cache
    /// (e.g. `google.com/amp/s/...`, `cdn.ampproject.org/...`).
    pub is_cached: Option<bool>,
    /// `true` when the URL parses as a valid HTTP(S) URL.
    pub is_valid: Option<bool>,
    /// The URL as received from the caller.
    pub url: Option<String>,
}

fn default_guess_and_check() -> bool {
    true
}
fn default_max_depth() -> u32 {
    3
}

/// HTTP entry point. Unwraps state + JSON body; the actual dispatch logic
/// lives in [`dispatch`] so integration tests can drive it without going
/// through Axum's extractor stack.
#[utoipa::path(
    post,
    path = "/api/v2/convert",
    tag = "convert",
    summary = "Convert AMP URL",
    description = "Resolve AMP URLs to their canonicals.",
    request_body = ConvertBody,
    responses(
        (status = 200, description = "Resolved links + optional Reddit reply markdown, recursively camelCased", body = ConvertResponse),
        (status = 303, description = "Redirect to canonical (only when redirect=true and a canonical was found)"),
        (status = 400, description = "Empty `query` field", body = crate::routes::error::ErrorResponse),
        (status = 406, description = "No AMP URL detected", body = crate::routes::error::ErrorResponse),
        (status = 422, description = "Body failed to deserialize — unknown field, bad casing, or wrong type"),
    )
)]
pub async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ConvertBody>,
) -> Response {
    let entry_type = entry_type_from_headers(&headers);
    dispatch(&state.fetcher, &state.db, body, entry_type).await
}

fn entry_type_from_headers(headers: &HeaderMap) -> EntryType {
    headers
        .get(ENTRY_TYPE_HEADER)
        .and_then(|h| h.to_str().ok())
        .and_then(parse_entry_type)
        .unwrap_or(EntryType::Api)
}

fn parse_entry_type(s: &str) -> Option<EntryType> {
    match s {
        "API" => Some(EntryType::Api),
        "COMMENT" => Some(EntryType::Comment),
        "SUBMISSION" => Some(EntryType::Submission),
        "MENTION" => Some(EntryType::Mention),
        "ONLINE" => Some(EntryType::Online),
        _ => None,
    }
}

/// Build + render the v2 contract. Generic over the trait bounds so tests
/// can supply mocks. `entry_type` comes from the
/// `X-AmputatorBot-Entry-Type` header in the HTTP handler; tests pass it
/// directly.
pub async fn dispatch<P, D>(
    fetcher: &P,
    db: &D,
    body: ConvertBody,
    entry_type: EntryType,
) -> Response
where
    P: crate::canonical::PageSource,
    D: crate::canonical::Database,
{
    if body.query.is_empty() {
        return missing_query_response();
    }
    let custom_footer = body.custom_footer;
    let generate_comment = body.generate_markdown_comment;
    let input = ConvertInput {
        q: body.query,
        use_gac: body.guess_and_check,
        max_depth: body.max_depth,
        redirect: body.redirect,
        entry_type,
        api_version: 2,
    };
    render(
        convert_inner(fetcher, db, input).await,
        entry_type,
        custom_footer,
        generate_comment,
    )
}

/// Render [`ConvertOutcome`] → camelCase JSON `Response`. The Link tree is
/// produced by serde with its native snake_case naming, then transformed
/// recursively at the edge into the v2 envelope shape.
fn render(
    outcome: ConvertOutcome,
    entry_type: EntryType,
    custom_footer: Option<String>,
    generate_comment: bool,
) -> Response {
    match outcome {
        ConvertOutcome::Resolved(links) => {
            let comment = if generate_comment {
                build_reply(
                    &links,
                    &BuildReplyOptions {
                        entry_type,
                        custom_footer,
                    },
                )
            } else {
                None
            };
            let links_value =
                camelize(serde_json::to_value(&links).expect("Link is always serializable"));
            let envelope = json!({
                "links": links_value,
                "comment": comment,
            });
            (StatusCode::OK, Json(envelope)).into_response()
        }
        ConvertOutcome::Redirect(target) => Redirect::to(&target).into_response(),
        ConvertOutcome::NoAmp => no_amp_response(),
    }
}

fn missing_query_response() -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "api_error_required_field_missing",
        "Error: No query field provided. Set `query` in the JSON body.",
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
/// Used by the response renderer so the snake_case [`crate::models`] types
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
