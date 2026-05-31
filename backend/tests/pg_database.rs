//! Integration tests against the local Docker Postgres (`just db-up`).
//!
//! Requires `DATABASE_URL` to be set (defaults to the docker-compose URL).
//! Each test runs migrations + cleans the `links` table on entry so it's
//! self-contained.

use amputatorbot_backend::canonical::database::{Database, Resolution};
use amputatorbot_backend::canonical::pg_database::PgDatabase;
use amputatorbot_backend::models::{CanonicalType, ConfidenceLevel, EntryType};
use chrono::{Duration, Utc};
use sqlx::PgPool;

const DEFAULT_DB_URL: &str = "postgres://amputatorbot:amputatorbot@localhost:5432/amputatorbot";

async fn setup() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DB_URL.to_string());
    let pool = match PgPool::connect(&url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("skipping pg_database test ({e}). Run `just db-up` to enable.");
            return None;
        }
    };
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::query!("TRUNCATE links RESTART IDENTITY")
        .execute(&pool)
        .await
        .unwrap();
    Some(pool)
}

/// The 1-year freshness gate: a row older than 365 days for the same
/// `original_url` must NOT be returned; the resolver should re-discover the
/// canonical instead of trusting potentially-stale cache data.
#[tokio::test]
async fn lookup_filters_rows_older_than_one_year() {
    let Some(pool) = setup().await else { return };
    let db = PgDatabase::new(pool.clone());

    let original = "https://www.google.com/amp/s/example.eu/article-freshness-test";
    let stale_canonical = "https://example.eu/article-old";
    let fresh_canonical = "https://example.eu/article-new";

    // Stale: 18 months ago — must be ignored.
    let stale_when = Utc::now() - Duration::days(18 * 30);
    sqlx::query!(
        "INSERT INTO links (entry_type, original_url, canonical_url, canonical_type, api_version, handled_utc) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        EntryType::Api as EntryType,
        original,
        stale_canonical,
        CanonicalType::Rel as CanonicalType,
        2_i16,
        stale_when,
    )
    .execute(&pool)
    .await
    .unwrap();

    let hit = db.lookup_canonical(original).await.unwrap();
    assert!(
        hit.is_none(),
        "stale row (>1 year) must NOT be returned, got {hit:?}"
    );

    // Fresh: 6 months ago — must be returned.
    let fresh_when = Utc::now() - Duration::days(180);
    sqlx::query!(
        "INSERT INTO links (entry_type, original_url, canonical_url, canonical_type, api_version, handled_utc) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        EntryType::Api as EntryType,
        original,
        fresh_canonical,
        CanonicalType::Rel as CanonicalType,
        2_i16,
        fresh_when,
    )
    .execute(&pool)
    .await
    .unwrap();

    let hit = db.lookup_canonical(original).await.unwrap();
    assert_eq!(hit.as_deref(), Some(fresh_canonical));
}

/// `record_resolution` writes all confidence-related columns so future
/// `lookup_canonical` calls (or out-of-band analytics queries) see the
/// scoring data.
#[tokio::test]
async fn record_resolution_persists_confidence_columns() {
    let Some(pool) = setup().await else { return };
    let db = PgDatabase::new(pool.clone());

    let original = "https://www.google.com/amp/s/example.eu/article-confidence";
    let canonical = "https://example.eu/article-confidence";

    db.record_resolution(Resolution {
        entry_type: EntryType::Api,
        api_version: 2,
        original_url: original,
        canonical_url: Some(canonical),
        canonical_type: Some(CanonicalType::Rel),
        url_similarity: Some(0.87),
        article_similarity: Some(0.92),
        confidence_score: Some(0.90),
        confidence_level: Some(ConfidenceLevel::Verified),
    })
    .await
    .unwrap();

    let row = sqlx::query!(
        "SELECT canonical_url, url_similarity, article_similarity, confidence_score, \
                confidence_level AS \"confidence_level: ConfidenceLevel\" \
         FROM links WHERE original_url = $1 LIMIT 1",
        original
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.canonical_url.as_deref(), Some(canonical));
    assert!((row.url_similarity.unwrap() - 0.87).abs() < 0.001);
    assert!((row.article_similarity.unwrap() - 0.92).abs() < 0.001);
    assert!((row.confidence_score.unwrap() - 0.90).abs() < 0.001);
    assert_eq!(row.confidence_level, Some(ConfidenceLevel::Verified));
}
