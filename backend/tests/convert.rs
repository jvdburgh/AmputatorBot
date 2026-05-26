//! Integration tests for `/api/v1/convert`.
//!
//! Drive [`amputatorbot_backend::routes::convert::convert_inner`] directly
//! with a mock [`PageSource`] and a recording [`Database`]. Skips the
//! Axum router so tests don't need a live HTTP server; the same code runs
//! the request once it leaves the router, and the router's only job is
//! adapting `(State, HeaderMap, Uri)` into the same call.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;

use amputatorbot_backend::canonical::database::Resolution;
use amputatorbot_backend::canonical::{Database, Page, PageSource};
use amputatorbot_backend::models::{CanonicalType, EntryType};
use amputatorbot_backend::routes::convert::convert_inner;
use anyhow::Result;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use http_body_util::BodyExt;
use serde_json::Value;

// ════════════════════════════════════════════════════════════════════════
//  Mocks
// ════════════════════════════════════════════════════════════════════════

/// URL → HTML map. Unknown URLs return a fetch error.
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

/// Captures every [`Database::record_resolution`] call into a Vec.
///
/// Owned strings (not borrowed) because the Resolution's lifetime is bound
/// to the handler's call; we copy out before storing.
#[derive(Default)]
struct RecordingDatabase {
    recorded: Mutex<Vec<RecordedResolution>>,
}

#[derive(Debug, Clone, PartialEq)]
struct RecordedResolution {
    entry_type: EntryType,
    original_url: String,
    canonical_url: Option<String>,
    canonical_type: Option<CanonicalType>,
}

impl Database for RecordingDatabase {
    fn lookup_canonical(
        &self,
        _original_url: &str,
    ) -> impl Future<Output = Result<Option<String>>> + Send {
        // Tests don't care about cache hits; resolver finds canonicals via
        // mock HTML instead.
        std::future::ready(Ok(None))
    }

    fn record_resolution(&self, entry: Resolution<'_>) -> impl Future<Output = Result<()>> + Send {
        self.recorded.lock().unwrap().push(RecordedResolution {
            entry_type: entry.entry_type,
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

fn headers_with(entry_type: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        "X-AmputatorBot-Entry-Type",
        HeaderValue::from_str(entry_type).unwrap(),
    );
    h
}

// ════════════════════════════════════════════════════════════════════════
//  Tests
// ════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn missing_q_returns_400() {
    let fetcher = MockPageSource::new();
    let db = RecordingDatabase::default();
    let resp = convert_inner(&fetcher, &db, "gac=true&md=3", &HeaderMap::new()).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let json = body_json(resp).await;
    assert_eq!(json["result_code"], "api_error_required_field_missing");
    assert!(db.recorded.lock().unwrap().is_empty(), "no DB write on 400");
}

#[tokio::test]
async fn empty_q_returns_400() {
    let fetcher = MockPageSource::new();
    let db = RecordingDatabase::default();
    let resp = convert_inner(&fetcher, &db, "q=&gac=true", &HeaderMap::new()).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn non_amp_url_returns_406() {
    // The body parses cleanly to a URL, but it doesn't match any AMP
    // pattern → 406 NO_AMP. No fetch, no DB write.
    let fetcher = MockPageSource::new();
    let db = RecordingDatabase::default();
    let resp = convert_inner(
        &fetcher,
        &db,
        "q=https%3A%2F%2Fnews.ycombinator.com%2Fitem%3Fid%3D42",
        &HeaderMap::new(),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
    let json = body_json(resp).await;
    assert_eq!(json["result_code"], "error_no_amp");
    assert!(db.recorded.lock().unwrap().is_empty(), "406 must not write");
}

#[tokio::test]
async fn non_url_text_returns_406() {
    let fetcher = MockPageSource::new();
    let db = RecordingDatabase::default();
    let resp = convert_inner(&fetcher, &db, "q=hello%20world", &HeaderMap::new()).await;
    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
}

#[tokio::test]
async fn happy_path_encoded_url_returns_200_with_canonical() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    // The trailing `%20` flips the parser onto the args-decoded path; the
    // decoded space at the end is a URL boundary for linkify, so the AMP
    // URL extracts cleanly.
    let raw = "q=https%3A%2F%2Fwww.google.com%2Famp%2Fs%2Fexample.eu%2Farticle%20";
    let resp = convert_inner(&fetcher, &db, raw, &HeaderMap::new()).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let arr = json.as_array().expect("response is array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["canonical"]["url"], target);

    // One record written, entry_type=API (header missing → default).
    let recorded = db.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].entry_type, EntryType::Api);
    assert_eq!(recorded[0].canonical_url.as_deref(), Some(target));
    assert_eq!(recorded[0].canonical_type, Some(CanonicalType::Rel));
}

#[tokio::test]
async fn happy_path_unencoded_url_returns_200() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let raw = format!("gac=true&q={amp}");
    let resp = convert_inner(&fetcher, &db, &raw, &HeaderMap::new()).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json[0]["canonical"]["url"], target);
}

#[tokio::test]
async fn no_canonical_found_returns_200_with_null_canonical() {
    // AMP origin, fetch succeeds but the page has no canonical signals
    // anywhere — every method returns nothing. v7 decision: 200 + null
    // canonical (not 560).
    let amp = "https://www.google.com/amp/s/example.eu/empty";
    let fetcher = MockPageSource::new().with(amp, "<html><body>nothing here</body></html>");
    let db = RecordingDatabase::default();

    let raw = format!("q={amp}");
    let resp = convert_inner(&fetcher, &db, &raw, &HeaderMap::new()).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(
        json[0]["canonical"].is_null(),
        "canonical should be null, got {:?}",
        json[0]["canonical"]
    );

    // Legacy save_entry writes a row even on "no canonical found" — the
    // null canonical_url is meaningful ("we tried"). Faithful port.
    let recorded = db.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].canonical_url, None);
    assert_eq!(recorded[0].canonical_type, None);
}

#[tokio::test]
async fn redirect_mode_303_to_canonical() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let raw = format!("q={amp}&r=true");
    let resp = convert_inner(&fetcher, &db, &raw, &HeaderMap::new()).await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    let location = resp
        .headers()
        .get("location")
        .expect("303 must carry Location")
        .to_str()
        .unwrap();
    assert_eq!(location, target);

    // DB write still happens for r=true (matches legacy: save_entry runs
    // before the redirect branch in main.py:run_amputatorbotcom).
    assert_eq!(db.recorded.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn redirect_mode_without_canonical_falls_through_to_200() {
    // r=true but the page has no canonical → we can't redirect to nothing.
    // Fall through to the 200+JSON response so the caller knows why.
    let amp = "https://www.google.com/amp/s/example.eu/empty";
    let fetcher = MockPageSource::new().with(amp, "<html><body>nothing</body></html>");
    let db = RecordingDatabase::default();

    let raw = format!("q={amp}&r=true");
    let resp = convert_inner(&fetcher, &db, &raw, &HeaderMap::new()).await;

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn entry_type_header_comment_recorded() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let raw = format!("q={amp}");
    let resp = convert_inner(&fetcher, &db, &raw, &headers_with("COMMENT")).await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        db.recorded.lock().unwrap()[0].entry_type,
        EntryType::Comment
    );
}

#[tokio::test]
async fn entry_type_header_case_insensitive() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let raw = format!("q={amp}");
    let resp = convert_inner(&fetcher, &db, &raw, &headers_with("submission")).await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        db.recorded.lock().unwrap()[0].entry_type,
        EntryType::Submission
    );
}

#[tokio::test]
async fn unknown_entry_type_header_falls_back_to_api() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let raw = format!("q={amp}");
    let resp = convert_inner(&fetcher, &db, &raw, &headers_with("BANANA")).await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(db.recorded.lock().unwrap()[0].entry_type, EntryType::Api);
}

#[tokio::test]
async fn missing_entry_type_header_defaults_to_api() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let raw = format!("q={amp}");
    let resp = convert_inner(&fetcher, &db, &raw, &HeaderMap::new()).await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(db.recorded.lock().unwrap()[0].entry_type, EntryType::Api);
}

#[tokio::test]
async fn gc_param_silently_ignored() {
    // Legacy /api/v1/convert never read gc; we match. Setting gc=true should
    // produce identical output to gc=false (the bare array, no reply text).
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let raw = format!("q={amp}&gc=true");
    let resp = convert_inner(&fetcher, &db, &raw, &HeaderMap::new()).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.is_array(), "gc should not change response shape");
    assert!(
        !json.to_string().contains("reply_markdown"),
        "response must not contain reply_markdown"
    );
}
