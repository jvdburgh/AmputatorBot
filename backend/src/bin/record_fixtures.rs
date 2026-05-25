//! One-off recorder: fetch every URL in a URLConversions CSV export and
//! save the response HTML to `backend/tests/fixtures/html/{id}.json` for
//! offline replay by the M2.10 parity test.
//!
//! Idempotent — already-recorded fixtures are skipped, so the tool can be
//! re-run safely after a crash or partial run.
//!
//! Usage:
//!
//! ```bash
//! cd backend
//! cargo run --bin record_fixtures -- \
//!     --input tests/fixtures/urlconversions/URLConversions_500_including_failed_and_false_positives.csv \
//!     --output tests/fixtures/html
//! ```
//!
//! CSV schema (no header, comma-separated):
//!   id, type, timestamp_utc, original_url, canonical_url, canonical_type, note

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use amputatorbot_backend::canonical::HttpFetcher;

#[derive(Parser, Debug)]
#[command(about = "Record HTML fixtures for AmputatorBot parity tests")]
struct Args {
    /// Input CSV file (a URLConversions export).
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory for HTML fixtures.
    #[arg(short, long, default_value = "tests/fixtures/html")]
    output: PathBuf,

    /// Maximum concurrent in-flight fetches. Default 4 is conservative —
    /// many distinct publisher domains, no one of them hit hard.
    #[arg(short, long, default_value_t = 4)]
    concurrency: usize,

    /// Re-record even when a fixture already exists.
    #[arg(long)]
    force: bool,
}

/// One recorded fixture: the original CSV row + the fetched page.
///
/// `expected_canonical` / `expected_type` come from the CSV's
/// `canonical_url` / `canonical_type` columns; they're what the legacy
/// Python bot recorded for this URL. The parity test compares the new
/// Rust impl's output against these.
#[derive(Debug, Serialize, Deserialize)]
struct FixtureRecord {
    csv_id: i64,
    original_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_canonical: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_type: Option<String>,
    fetched_at: String,
    final_url: String,
    status_code: u16,
    html: String,
}

struct CsvRow {
    csv_id: i64,
    original_url: String,
    expected_canonical: Option<String>,
    expected_type: Option<String>,
}

fn read_csv(path: &Path) -> Result<Vec<CsvRow>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("opening {}", path.display()))?;

    let rows: Vec<CsvRow> = reader
        .records()
        .filter_map(|r| r.ok())
        .filter_map(|r| {
            let csv_id: i64 = r.get(0)?.parse().ok()?;
            let original_url = r.get(3)?.trim().to_string();
            if original_url.is_empty() {
                return None;
            }
            let expected_canonical = r
                .get(4)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from);
            let expected_type = r
                .get(5)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from);
            Some(CsvRow {
                csv_id,
                original_url,
                expected_canonical,
                expected_type,
            })
        })
        .collect();

    Ok(rows)
}

#[derive(Debug, Clone, Copy)]
enum Outcome {
    Recorded,
    Skipped,
    Failed,
}

async fn record_one(fetcher: &HttpFetcher, row: CsvRow, output: &Path, force: bool) -> Outcome {
    let path = output.join(format!("{}.json", row.csv_id));
    if !force && path.exists() {
        return Outcome::Skipped;
    }

    let page = match fetcher.fetch(&row.original_url).await {
        Ok(p) => p,
        Err(e) => {
            // `?e` uses anyhow's Debug impl, which prints the full cause
            // chain ("Caused by: …"). `%e` (Display) only shows the top
            // context. We want the full chain so we can tell apart DNS,
            // TLS, HTTP-status, redirect-loop, timeout, etc.
            tracing::warn!(csv_id = row.csv_id, url = %row.original_url, error = ?e, "fetch failed");
            return Outcome::Failed;
        }
    };

    let record = FixtureRecord {
        csv_id: row.csv_id,
        original_url: row.original_url,
        expected_canonical: row.expected_canonical,
        expected_type: row.expected_type,
        fetched_at: chrono::Utc::now().to_rfc3339(),
        final_url: page.current_url,
        status_code: page.status_code,
        html: page.html,
    };

    let json = match serde_json::to_string(&record) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!(csv_id = record.csv_id, error = %e, "serialize failed");
            return Outcome::Failed;
        }
    };

    if let Err(e) = std::fs::write(&path, json) {
        tracing::error!(csv_id = record.csv_id, error = %e, "write failed");
        return Outcome::Failed;
    }

    Outcome::Recorded
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    std::fs::create_dir_all(&args.output)
        .with_context(|| format!("creating {}", args.output.display()))?;

    let rows = read_csv(&args.input)?;
    let total = rows.len();
    println!("Loaded {total} rows from {}", args.input.display());
    println!(
        "Output: {} (concurrency={}, force={})",
        args.output.display(),
        args.concurrency,
        args.force
    );

    let fetcher = HttpFetcher::new()?;

    let mut stream = futures::stream::iter(rows.into_iter().map(|row| {
        let fetcher = &fetcher;
        let output = &args.output;
        async move {
            (
                row.csv_id,
                record_one(fetcher, row, output, args.force).await,
            )
        }
    }))
    .buffer_unordered(args.concurrency);

    let mut recorded = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let mut done = 0usize;

    while let Some((csv_id, outcome)) = stream.next().await {
        done += 1;
        match outcome {
            Outcome::Recorded => recorded += 1,
            Outcome::Skipped => skipped += 1,
            Outcome::Failed => failed += 1,
        }
        if done.is_multiple_of(25) || done == total {
            eprintln!(
                "  [{done}/{total}] last_id={csv_id} recorded={recorded} skipped={skipped} failed={failed}"
            );
        }
    }

    println!("\nDone. recorded={recorded} skipped={skipped} failed={failed} total={total}");
    Ok(())
}
