//! Snapshot tests — pin the resolver's `Link` JSON output for representative
//! canonical-finding scenarios. Catches accidental response-shape drift
//! (field renames, added/removed keys, serialization changes).
//!
//! Stores expected JSON in `tests/snapshots/snapshots__<name>.snap`. To
//! regenerate after an intentional shape change:
//!
//! ```bash
//! cargo install cargo-insta   # one-time
//! INSTA_UPDATE=always cargo nextest run --test snapshots
//! # then `cargo insta review` to accept/reject the diffs
//! ```
//!
//! Each test scenario is small enough to read in one screen. The real
//! end-to-end parity validation lives in `parity.rs`; this file is the
//! shape regression guard.

use std::collections::HashMap;
use std::future::{Future, ready};

use anyhow::Result;
use insta::assert_json_snapshot;

use amputatorbot_backend::canonical::{Page, PageSource, ResolveOpts, resolve};

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
            url.into(),
            Page {
                current_url: url.into(),
                status_code: 200,
                title: String::new(),
                html: html.into(),
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
            .ok_or_else(|| anyhow::anyhow!("not in mock: {url}"));
        ready(r)
    }
}

fn rel_canonical(href: &str) -> String {
    format!(
        r#"<!doctype html><html><head><link rel="canonical" href="{href}"></head><body>x</body></html>"#
    )
}

#[tokio::test]
async fn snapshot_rel_canonical() {
    let amp = "https://www.google.com/amp/s/example.com/article";
    let mock = MockPageSource::new().with(amp, &rel_canonical("https://example.com/article"));
    let link = resolve(&mock, amp, ResolveOpts::default()).await;
    assert_json_snapshot!(link);
}

#[tokio::test]
async fn snapshot_og_url_canonical() {
    let amp = "https://amp.example.com/post-42";
    let html = r#"<!doctype html><html><head>
        <meta property="og:url" content="https://example.com/post-42">
    </head><body>x</body></html>"#;
    let mock = MockPageSource::new().with(amp, html);
    let link = resolve(&mock, amp, ResolveOpts::default()).await;
    assert_json_snapshot!(link);
}

#[tokio::test]
async fn snapshot_non_amp_origin_returns_empty_link() {
    // Non-AMP origins short-circuit before any fetch — the result has
    // origin.is_amp = false and no canonicals.
    let url = "https://news.ycombinator.com/item?id=42";
    let mock = MockPageSource::new();
    let link = resolve(&mock, url, ResolveOpts::default()).await;
    assert_json_snapshot!(link);
}

#[tokio::test]
async fn snapshot_amputeestore_false_positive_short_circuits() {
    // The famous false-positive shape in the URLConversions fixture set.
    // The legacy substring-AMP-detect flagged `amputeestore.com` URLs as
    // AMP (because `//a...mp` after the scheme spans `/amp`). Our
    // component-scoped detector correctly rejects them — origin.is_amp
    // = false, canonical-finding never runs.
    let url = "https://amputeestore.com/products/tamarack-glidewear-prosthetic-liner-patch";
    let mock = MockPageSource::new();
    let link = resolve(&mock, url, ResolveOpts::default()).await;
    assert_json_snapshot!(link);
}

#[tokio::test]
async fn snapshot_dead_end_amp_with_cached_origin_surfaces_amp_canonical() {
    // Origin is a Google AMP-cache URL. After exhausting depth every
    // canonical is still AMP → amp_canonical falls back so the caller
    // gets something better than the cached URL.
    let origin = "https://www.google.com/amp/s/example.com/article/amp/";
    let stuck = "https://example.com/article/amp/";
    let mock = MockPageSource::new()
        .with(origin, &rel_canonical(stuck))
        .with(stuck, &rel_canonical(stuck));
    let link = resolve(&mock, origin, ResolveOpts::default()).await;
    assert_json_snapshot!(link);
}

#[tokio::test]
async fn snapshot_depth_recursion_through_amp_chain() {
    let origin = "https://www.google.com/amp/s/example.com/article/amp/";
    let intermediate = "https://example.com/article/amp/";
    let final_url = "https://example.com/article/";

    let mock = MockPageSource::new()
        .with(origin, &rel_canonical(intermediate))
        .with(intermediate, &rel_canonical(final_url));
    let link = resolve(&mock, origin, ResolveOpts::default()).await;
    assert_json_snapshot!(link);
}

#[tokio::test]
async fn snapshot_multiple_canonical_signals_sorted_by_similarity() {
    let amp = "https://www.google.com/amp/s/example.com/article-2024-tesla";
    let html = r#"<!doctype html><html><head>
        <link rel="canonical" href="https://example.com/article-2024-tesla">
        <meta property="og:url" content="https://example.com/totally-different-path">
    </head><body>x</body></html>"#;
    let mock = MockPageSource::new().with(amp, html);
    let link = resolve(&mock, amp, ResolveOpts::default()).await;
    assert_json_snapshot!(link);
}

#[tokio::test]
async fn snapshot_meta_refresh_canonical() {
    let amp = "https://amp.example.com/article";
    let html = r#"<!doctype html><html><head>
        <meta http-equiv="refresh" content="0; url=https://example.com/article">
    </head></html>"#;
    let mock = MockPageSource::new().with(amp, html);
    let link = resolve(&mock, amp, ResolveOpts::default()).await;
    assert_json_snapshot!(link);
}

#[tokio::test]
async fn snapshot_schema_mainentity_canonical() {
    let amp = "https://amp.example.com/news/story";
    let html = r#"<!doctype html><html><head><script type="application/ld+json">
        { "@type": "NewsArticle",
          "mainEntityOfPage": "https://example.com/news/story",
          "headline": "x" }
    </script></head><body>x</body></html>"#;
    let mock = MockPageSource::new().with(amp, html);
    let link = resolve(&mock, amp, ResolveOpts::default()).await;
    assert_json_snapshot!(link);
}

#[tokio::test]
async fn snapshot_fetch_failure_returns_empty_link() {
    // AMP origin but mock has no pages → first fetch fails → graceful
    // empty Link (origin populated, canonicals empty, no canonical).
    let amp = "https://www.google.com/amp/s/example.com/article";
    let mock = MockPageSource::new();
    let link = resolve(&mock, amp, ResolveOpts::default()).await;
    assert_json_snapshot!(link);
}
