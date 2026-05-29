//! Aggregate stats served from `GET /api/v2/stats`.
//!
//! Currently a single value — the total number of rows in `links`, which is
//! the bot's "AMP links converted since 2019" headline number on the
//! homepage. A full table count is cheap-ish on 1.7M rows (~50ms cold,
//! index-only scans don't help — `COUNT(*)` needs to visit every visible
//! row) but we cache the result for an hour so the homepage doesn't run
//! one count per pageview.
//!
//! `Arc<RwLock<...>>` rather than `tokio::sync::Mutex` because reads vastly
//! outnumber writes (every cache hit is a read; only the once-an-hour miss
//! takes the write lock). The "thundering-herd" race where N concurrent
//! cold requests each run their own COUNT is accepted — the homepage isn't
//! a hot path and the COUNT is harmless to repeat.

use std::sync::Arc;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::sync::RwLock;

const CACHE_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Clone)]
pub struct Stats {
    pool: PgPool,
    cache: Arc<RwLock<Option<CachedCount>>>,
}

#[derive(Clone, Copy)]
struct CachedCount {
    value: i64,
    fetched_at: Instant,
}

impl Stats {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Total rows in `links` — the homepage's "X converted" counter.
    ///
    /// Returns the cached value when fresh; on miss, runs `SELECT COUNT(*)`
    /// and refreshes the cache. The count is `i64` because Postgres'
    /// `COUNT(*)` is a bigint and sqlx surfaces it as such; we keep the
    /// signed type rather than converting to `u64` so DB-side weirdness
    /// (a negative result, somehow) surfaces as a runtime error in the
    /// handler instead of being silently coerced.
    pub async fn converted_total(&self) -> anyhow::Result<i64> {
        if let Some(cached) = *self.cache.read().await
            && cached.fetched_at.elapsed() < CACHE_TTL
        {
            return Ok(cached.value);
        }

        let row = sqlx::query!("SELECT COUNT(*) AS count FROM links")
            .fetch_one(&self.pool)
            .await?;
        let value = row.count.unwrap_or(0);

        *self.cache.write().await = Some(CachedCount {
            value,
            fetched_at: Instant::now(),
        });

        Ok(value)
    }
}
