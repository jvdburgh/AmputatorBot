//! [`PgDatabase`] — production [`Database`] impl backed by a `sqlx::PgPool`.
//!
//! The legacy bot's lookup query (from
//! `archive/datahandlers/remote_datahandler.py:get_entry_by_original_url`)
//! was an unordered `LIMIT 1`. We modernize: prefer the most-recently-
//! resolved canonical, since the bot has been running for years and the
//! "right" canonical for a given URL can change over time (sites move,
//! the canonical-finding methods themselves have improved). `entry_id
//! DESC` is a deterministic tiebreaker when two rows share `handled_utc`.
//!
//! The composite index `(original_url, handled_utc DESC)` makes this an
//! index-only scan even on URLs with thousands of historical resolutions.
//!
//! We only read `canonical_url` — the orchestrator labels every result
//! from this method as `CanonicalType::Database`, so the row's own
//! `canonical_type` is ignored.

use anyhow::Result;
use sqlx::PgPool;

use super::database::Database;

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
        let row = sqlx::query!(
            "SELECT canonical_url \
             FROM links \
             WHERE original_url = $1 AND canonical_url IS NOT NULL \
             ORDER BY handled_utc DESC, entry_id DESC \
             LIMIT 1",
            original_url
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|r| r.canonical_url))
    }
}
