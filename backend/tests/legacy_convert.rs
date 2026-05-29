//! Integration tests for `/api/v1/convert`.
//!
//! Drive [`amputatorbot_backend::routes::legacy_convert::legacy_dispatch`] with
//! a mock [`PageSource`] and a recording [`Database`]. Same code path the live
//! Axum handler runs — the handler is a thin wrapper that just unpacks
//! `State` + `Uri`.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;

use amputatorbot_backend::canonical::database::Resolution;
use amputatorbot_backend::canonical::{Database, Page, PageSource};
use amputatorbot_backend::models::{CanonicalType, EntryType};
use amputatorbot_backend::routes::legacy_convert::legacy_dispatch;
use anyhow::Result;
use axum::http::StatusCode;
use axum::response::Response;
use http_body_util::BodyExt;
use serde_json::Value;

// ════════════════════════════════════════════════════════════════════════
//  Mocks
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
    fn lookup_canonical(
        &self,
        _original_url: &str,
    ) -> impl Future<Output = Result<Option<String>>> + Send {
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

// ════════════════════════════════════════════════════════════════════════
//  Tests
// ════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn missing_q_returns_400() {
    let fetcher = MockPageSource::new();
    let db = RecordingDatabase::default();
    let resp = legacy_dispatch(&fetcher, &db, "gac=true&md=3").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let json = body_json(resp).await;
    assert_eq!(json["result_code"], "api_error_required_field_missing");
    assert!(db.recorded.lock().unwrap().is_empty(), "no DB write on 400");
}

#[tokio::test]
async fn empty_q_returns_400() {
    let fetcher = MockPageSource::new();
    let db = RecordingDatabase::default();
    let resp = legacy_dispatch(&fetcher, &db, "q=&gac=true").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn non_amp_url_returns_406() {
    let fetcher = MockPageSource::new();
    let db = RecordingDatabase::default();
    let resp = legacy_dispatch(
        &fetcher,
        &db,
        "q=https%3A%2F%2Fnews.ycombinator.com%2Fitem%3Fid%3D42",
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
    let resp = legacy_dispatch(&fetcher, &db, "q=hello%20world").await;
    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
}

#[tokio::test]
async fn happy_path_encoded_url_returns_200_with_canonical() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let raw = "q=https%3A%2F%2Fwww.google.com%2Famp%2Fs%2Fexample.eu%2Farticle";
    let resp = legacy_dispatch(&fetcher, &db, raw).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let arr = json.as_array().expect("response is array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["canonical"]["url"], target);

    let recorded = db.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    // v1 always logs API + version 1.
    assert_eq!(recorded[0].entry_type, EntryType::Api);
    assert_eq!(recorded[0].api_version, 1);
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
    let resp = legacy_dispatch(&fetcher, &db, &raw).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json[0]["canonical"]["url"], target);
}

#[tokio::test]
async fn no_canonical_found_returns_200_with_null_canonical() {
    // AMP origin, fetch succeeds, but the page has no canonical signals.
    // v7 decision: 200 + null canonical (not the legacy 560).
    let amp = "https://www.google.com/amp/s/example.eu/empty";
    let fetcher = MockPageSource::new().with(amp, "<html><body>nothing here</body></html>");
    let db = RecordingDatabase::default();

    let raw = format!("q={amp}");
    let resp = legacy_dispatch(&fetcher, &db, &raw).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json[0]["canonical"].is_null());

    // Faithful port: a row with null canonical_url still gets written.
    let recorded = db.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].canonical_url, None);
    assert_eq!(recorded[0].canonical_type, None);
    assert_eq!(recorded[0].api_version, 1);
}

#[tokio::test]
async fn redirect_mode_303_to_canonical() {
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let raw = format!("q={amp}&r=true");
    let resp = legacy_dispatch(&fetcher, &db, &raw).await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    let location = resp.headers().get("location").unwrap().to_str().unwrap();
    assert_eq!(location, target);

    // DB write still happens for r=true.
    assert_eq!(db.recorded.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn redirect_mode_without_canonical_falls_through_to_200() {
    let amp = "https://www.google.com/amp/s/example.eu/empty";
    let fetcher = MockPageSource::new().with(amp, "<html><body>nothing</body></html>");
    let db = RecordingDatabase::default();

    let raw = format!("q={amp}&r=true");
    let resp = legacy_dispatch(&fetcher, &db, &raw).await;

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn gc_param_silently_ignored() {
    // Legacy /api/v1/convert never read gc; we match.
    let amp = "https://www.google.com/amp/s/example.eu/article";
    let target = "https://example.eu/article";
    let fetcher = MockPageSource::new().with(amp, &rel_canonical_html(target));
    let db = RecordingDatabase::default();

    let raw = format!("q={amp}&gc=true");
    let resp = legacy_dispatch(&fetcher, &db, &raw).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.is_array());
    assert!(
        !json.to_string().contains("reply_markdown"),
        "v1 must not contain reply_markdown"
    );
}
