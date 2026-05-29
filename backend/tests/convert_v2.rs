//! Integration tests for `POST /api/v2/convert`.
//!
//! Same mock pattern as `tests/convert.rs` — drive [`dispatch_v2`] with a
//! mock [`PageSource`] + recording [`Database`]. The handler is a thin
//! Axum wrapper over the same code path.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;

use amputatorbot_backend::canonical::database::Resolution;
use amputatorbot_backend::canonical::{Database, Page, PageSource};
use amputatorbot_backend::models::{CanonicalType, EntryType};
use amputatorbot_backend::routes::convert_v2::{ConvertBodyV2, dispatch_v2};
use anyhow::Result;
use axum::http::StatusCode;
use axum::response::Response;
use http_body_util::BodyExt;
use serde_json::{Value, json};

// ════════════════════════════════════════════════════════════════════════
//  Mocks (mirror tests/convert.rs)
// ════════════════════════════════════════════════════════════════════════

struct MockPageSource {
    pages: HashMap<String, Page>,
}
impl MockPageSource {
    fn new() -> Self {
        Self {
            pages: HashMap::new(),
        }
    }
    fn with(mut self, url: &str, html: &str) -> Self {
        self.pages.insert(
            url.to_string(),
            Page {
                current_url: url.to_string(),
                status_code: 200,
                title: "test".to_string(),
                html: html.to_string(),
            },
        );
        self
    }
}
impl PageSource for MockPageSource {
    fn fetch(&self, url: &str) -> impl Future<Output = Result<Page>> + Send {
        let r = self
            .pages
            .get(url)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("MockPageSource: no page for {url}"));
        std::future::ready(r)
    }
}

#[derive(Default)]
struct RecordingDatabase {
    recorded: Mutex<Vec<RecordedResolution>>,
}
#[derive(Debug, Clone, PartialEq)]
struct RecordedResolution {
    entry_type: EntryType,
    api_version: i16,
    original_url: String,
    canonical_url: Option<String>,
    canonical_type: Option<CanonicalType>,
}
impl Database for RecordingDatabase {
    fn lookup_canonical(&self, _: &str) -> impl Future<Output = Result<Option<String>>> + Send {
        std::future::ready(Ok(None))
    }
    fn record_resolution(&self, entry: Resolution<'_>) -> impl Future<Output = Result<()>> + Send {
        self.recorded.lock().unwrap().push(RecordedResolution {
            entry_type: entry.entry_type,
            api_version: entry.api_version,
            original_url: entry.original_url.to_string(),
            canonical_url: entry.canonical_url.map(String::from),
            canonical_type: entry.canonical_type,
        });
        std::future::ready(Ok(()))
    }
}

// ════════════════════════════════════════════════════════════════════════
//  Helpers
// ════════════════════════════════════════════════════════════════════════

fn rel_canonical_html(href: &str) -> String {
    format!(
        r#"<!doctype html><html><head><link rel="canonical" href="{href}"></head><body>x</body></html>"#
    )
}

async fn body_json(resp: Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).expect("response body should be JSON")
}

/// Build a `ConvertBodyV2` from a JSON value. Mirrors what Axum's
/// `Json<ConvertBodyV2>` extractor does once the body lands.
fn body_from(value: Value) -> ConvertBodyV2 {
    serde_json::from_value(value).expect("test JSON must deserialize")
}

// ════════════════════════════════════════════════════════════════════════
//  Tests
// ════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn happy_path_returns_camelcase_response() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let body = body_from(json!({ "query": amp }));
    let resp = dispatch_v2(&fetcher, &db, body).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let arr = json.as_array().expect("response is array");
    assert_eq!(arr.len(), 1);

    // Response keys are camelCase recursively.
    assert_eq!(arr[0]["canonical"]["url"], target);
    assert_eq!(arr[0]["canonical"]["isAmp"], false);
    assert_eq!(arr[0]["origin"]["isAmp"], true);
    assert_eq!(arr[0]["origin"]["isCached"], true);
    assert!(arr[0]["ampCanonical"].is_null());
    // No snake_case stragglers in the top-level shape.
    assert!(arr[0].get("is_amp").is_none());
    assert!(arr[0].get("amp_canonical").is_none());

    // v2 always logs api_version=2.
    let recorded = db.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].api_version, 2);
    assert_eq!(recorded[0].entry_type, EntryType::Api);
}

#[tokio::test]
async fn entry_type_from_body_is_recorded() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let body = body_from(json!({
        "query": amp,
        "entryType": "COMMENT"
    }));
    let resp = dispatch_v2(&fetcher, &db, body).await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        db.recorded.lock().unwrap()[0].entry_type,
        EntryType::Comment
    );
}

#[tokio::test]
async fn missing_query_returns_400() {
    let fetcher = MockPageSource::new();
    let db = RecordingDatabase::default();
    let body = body_from(json!({ "query": "" }));
    let resp = dispatch_v2(&fetcher, &db, body).await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    // Error response uses camelCase keys too.
    assert_eq!(json["resultCode"], "api_error_required_field_missing");
    assert!(json.get("result_code").is_none());
}

#[tokio::test]
async fn non_amp_url_returns_406() {
    let fetcher = MockPageSource::new();
    let db = RecordingDatabase::default();
    let body = body_from(json!({ "query": "https://news.ycombinator.com/item?id=42" }));
    let resp = dispatch_v2(&fetcher, &db, body).await;

    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
    let json = body_json(resp).await;
    assert_eq!(json["resultCode"], "error_no_amp");
}

#[tokio::test]
async fn redirect_303_to_canonical() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let body = body_from(json!({ "query": amp, "redirect": true }));
    let resp = dispatch_v2(&fetcher, &db, body).await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    let location = resp.headers().get("location").unwrap().to_str().unwrap();
    assert_eq!(location, target);
}

#[tokio::test]
async fn defaults_apply_when_optional_fields_omitted() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    // Only `query` set. guessAndCheck/maxDepth/redirect/entryType all default.
    let body = body_from(json!({ "query": amp }));
    let resp = dispatch_v2(&fetcher, &db, body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    // entryType defaulted to API.
    assert_eq!(db.recorded.lock().unwrap()[0].entry_type, EntryType::Api);
}

#[tokio::test]
async fn explicit_null_entry_type_defaults_to_api() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    // Callers that send `entryType: null` explicitly get the same fallback
    // as omitting the field.
    let body = body_from(json!({ "query": amp, "entryType": null }));
    let resp = dispatch_v2(&fetcher, &db, body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(db.recorded.lock().unwrap()[0].entry_type, EntryType::Api);
}

#[tokio::test]
async fn query_with_text_around_multiple_urls_resolves_all() {
    // `query` mirrors the v1 `q` parameter: either a bare URL or free text
    // containing one or more URLs. The same URL-extractor used for Reddit
    // comment bodies handles both, so a chat-style paste works.
    let amp1 = "https://www.google.com/amp/s/example.eu/article-1";
    let amp2 = "https://www.google.com/amp/s/example.eu/article-2";
    let target1 = "https://example.eu/article-1";
    let target2 = "https://example.eu/article-2";
    let fetcher = MockPageSource::new()
        .with(amp1, &rel_canonical_html(target1))
        .with(amp2, &rel_canonical_html(target2));
    let db = RecordingDatabase::default();

    let body = body_from(json!({
        "query": format!("hey, check these out: {amp1} and also {amp2} — thanks"),
    }));
    let resp = dispatch_v2(&fetcher, &db, body).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let arr = json.as_array().expect("response is array");
    assert_eq!(arr.len(), 2, "both AMP URLs should produce a Link");
    assert_eq!(arr[0]["canonical"]["url"], target1);
    assert_eq!(arr[1]["canonical"]["url"], target2);

    let recorded = db.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0].original_url, amp1);
    assert_eq!(recorded[1].original_url, amp2);
}

#[tokio::test]
async fn unknown_field_rejected_by_strict_deserializer() {
    // `deny_unknown_fields` should reject typos like `entry_type` (snake
    // when it should be `entryType`) at deserialization time. We test the
    // *deserializer* directly here since dispatch_v2 takes a parsed body
    // and Axum is what would return the 400 in production.
    let parsed: std::result::Result<ConvertBodyV2, _> =
        serde_json::from_value(json!({ "query": "x", "entry_type": "API" }));
    assert!(parsed.is_err(), "strict deserializer must reject typos");
}

#[tokio::test]
async fn invalid_entry_type_value_rejected() {
    // Strict enum values: serde rejects unknown EntryType strings.
    let parsed: std::result::Result<ConvertBodyV2, _> =
        serde_json::from_value(json!({ "query": "x", "entryType": "BANANA" }));
    assert!(parsed.is_err(), "unknown entryType must be rejected");
}

#[tokio::test]
async fn invalid_entry_type_casing_rejected() {
    // SCREAMING_SNAKE_CASE is required; "comment" doesn't match.
    let parsed: std::result::Result<ConvertBodyV2, _> =
        serde_json::from_value(json!({ "query": "x", "entryType": "comment" }));
    assert!(
        parsed.is_err(),
        "v2 is strict on casing; lowercase entryType must be rejected"
    );
}
