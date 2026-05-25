//! Parity test — runs the resolver against recorded HTML fixtures and
//! reports how its output compares to what the legacy Python bot recorded
//! in the `URLConversions` table.
//!
//! Setup:
//!
//! ```bash
//! cd backend
//! cargo run --bin record_fixtures -- \
//!   --input tests/fixtures/urlconversions/URLConversions_500_including_failed_and_false_positives.csv
//! cargo nextest run --test parity
//! ```
//!
//! If `tests/fixtures/html/` is empty (gitignored, never recorded locally),
//! the test logs a skip and passes. Doesn't fail CI just because nobody ran
//! the recorder.
//!
//! Output categorization per fixture:
//!
//! - **match** — both legacy + new found the same canonical URL (or both
//!   found nothing).
//! - **mismatch_url** — both found a canonical but they disagree on the URL.
//! - **legacy_only** — legacy found a canonical, we didn't (resolver missed
//!   it, possibly because it needed a depth-recursion to a URL we don't
//!   have HTML for).
//! - **new_only** — we found a canonical, legacy recorded none. Could be a
//!   legitimate improvement (e.g. legacy false-positive on an AMP-detection
//!   misfire — the famous `amputeestore.com` shape) or our false positive.
//! - **skipped** — fixture has no HTML (the recorder failed to fetch). Not
//!   counted toward the rate.

use std::collections::HashMap;
use std::fs;
use std::future::{Future, ready};
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use amputatorbot_backend::canonical::{Page, PageSource, ResolveOpts, resolve};

#[derive(Debug, Deserialize)]
struct FixtureRecord {
    csv_id: i64,
    original_url: String,
    expected_canonical: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    expected_type: Option<String>,
    #[allow(dead_code)]
    fetched_at: String,
    final_url: String,
    status_code: u16,
    html: String,
}

/// `PageSource` impl backed by an in-memory map of `url -> Page`. Keys include
/// both the original CSV URL and the post-redirect `final_url` so the
/// resolver finds the page whichever it requests.
struct RecordedPageSource {
    pages: HashMap<String, Page>,
}

impl PageSource for RecordedPageSource {
    fn fetch(&self, url: &str) -> impl Future<Output = Result<Page>> + Send {
        let result = self
            .pages
            .get(url)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no recorded HTML for {url}"));
        ready(result)
    }
}

fn load_fixtures(dir: &Path) -> Vec<FixtureRecord> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .filter_map(|e| {
            let bytes = fs::read(e.path()).ok()?;
            serde_json::from_slice::<FixtureRecord>(&bytes).ok()
        })
        .collect()
}

fn build_page_source(fixtures: &[FixtureRecord]) -> RecordedPageSource {
    let mut pages = HashMap::with_capacity(fixtures.len() * 2);
    for f in fixtures {
        let page = Page {
            current_url: f.final_url.clone(),
            status_code: f.status_code,
            title: String::new(),
            html: f.html.clone(),
        };
        pages.insert(f.original_url.clone(), page.clone());
        if f.final_url != f.original_url {
            pages.insert(f.final_url.clone(), page);
        }
    }
    RecordedPageSource { pages }
}

#[derive(Default)]
struct Stats {
    matched: usize,
    mismatch_url: usize,
    legacy_only: usize,
    new_only: usize,
    skipped: usize,
}

#[tokio::test]
async fn parity_against_recorded_urlconversions() {
    let fixtures_dir = Path::new("tests/fixtures/html");
    let fixtures = load_fixtures(fixtures_dir);

    if fixtures.is_empty() {
        eprintln!(
            "[parity] No fixtures in {} — run `cargo run --bin record_fixtures -- \
             --input tests/fixtures/urlconversions/URLConversions_500_including_failed_and_false_positives.csv` \
             then re-run this test. Skipping.",
            fixtures_dir.display()
        );
        return;
    }

    eprintln!("[parity] Loaded {} fixtures", fixtures.len());
    let source = build_page_source(&fixtures);
    let mut stats = Stats::default();
    let mut mismatches: Vec<(i64, String, String, String)> = Vec::new();

    for f in &fixtures {
        if f.html.is_empty() || f.status_code != 200 {
            stats.skipped += 1;
            continue;
        }

        let link = resolve(&source, &f.original_url, ResolveOpts::default()).await;
        let found = link.canonical.as_ref().and_then(|c| c.url.clone());

        match (f.expected_canonical.as_deref(), found.as_deref()) {
            (Some(exp), Some(got)) if exp == got => stats.matched += 1,
            (Some(exp), Some(got)) => {
                stats.mismatch_url += 1;
                mismatches.push((f.csv_id, f.original_url.clone(), exp.into(), got.into()));
            }
            (Some(_), None) => stats.legacy_only += 1,
            (None, Some(_)) => stats.new_only += 1,
            (None, None) => stats.matched += 1,
        }
    }

    let assessed = stats.matched + stats.mismatch_url + stats.legacy_only + stats.new_only;
    let rate = if assessed > 0 {
        100.0 * stats.matched as f64 / assessed as f64
    } else {
        0.0
    };

    eprintln!(
        "\n[parity] {matched}/{assessed} matched ({rate:.1}%) \
         | mismatch_url={mm} legacy_only={lo} new_only={no} skipped={sk}",
        matched = stats.matched,
        assessed = assessed,
        rate = rate,
        mm = stats.mismatch_url,
        lo = stats.legacy_only,
        no = stats.new_only,
        sk = stats.skipped,
    );

    if !mismatches.is_empty() {
        eprintln!("\n[parity] first 10 mismatches:");
        for (id, url, exp, got) in mismatches.iter().take(10) {
            eprintln!("  csv_id={id} url={url}");
            eprintln!("    expected: {exp}");
            eprintln!("    found:    {got}");
        }
    }

    // Hard floor: if we matched 0 across the board with non-empty fixtures
    // present, something's broken (probably the resolver, not the data).
    // We don't assert a tighter rate yet — that'll get ratcheted up as bugs
    // surface and get fixed in follow-up commits.
    assert!(
        assessed == 0 || stats.matched > 0,
        "parity test produced 0 matches across {assessed} assessed fixtures — \
         the resolver is broken or fixtures are corrupt"
    );
}
