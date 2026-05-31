//! `PgDatabase` — production [`Database`] impl backed by a `sqlx::PgPool`.
//!
//! Lookup picks the most-recent canonical (URLs' "right" canonical can change
//! over years as sites move). `entry_id DESC` tiebreaks. The composite index
//! `(original_url, handled_utc DESC)` makes this an index-only scan.

use anyhow::Result;
use sqlx::PgPool;

use super::database::{Database, Resolution};
use crate::models::{CanonicalType, ConfidenceLevel, EntryType};

#[derive(Clone)]
pub struct PgDatabase {
    pool: PgPool,
}

impl PgDatabase {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl Database for PgDatabase {
    async fn lookup_canonical(&self, original_url: &str) -> Result<Option<String>> {
        // 1-year freshness gate: stale cache rows are worse than re-resolving
        // (publishers move slugs + restructure paths).
        let row = sqlx::query!(
            "SELECT canonical_url \
             FROM links \
             WHERE original_url = $1 \
               AND canonical_url IS NOT NULL \
               AND handled_utc > NOW() - INTERVAL '1 year' \
             ORDER BY handled_utc DESC, entry_id DESC \
             LIMIT 1",
            original_url
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|r| r.canonical_url))
    }

    async fn record_resolution(&self, entry: Resolution<'_>) -> Result<()> {
        // `handled_utc` omitted — DB's `DEFAULT NOW()` owns the timestamp.
        sqlx::query!(
            "INSERT INTO links \
             (entry_type, original_url, canonical_url, canonical_type, api_version, \
              url_similarity, article_similarity, confidence_score, confidence_level) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            entry.entry_type as EntryType,
            entry.original_url,
            entry.canonical_url,
            entry.canonical_type as Option<CanonicalType>,
            entry.api_version,
            entry.url_similarity.map(|f| f as f32),
            entry.article_similarity.map(|f| f as f32),
            entry.confidence_score.map(|f| f as f32),
            entry.confidence_level as Option<ConfidenceLevel>,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
