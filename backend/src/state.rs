//! [`AppState`] — shared services every HTTP handler depends on.
//!
//! Held by the Axum router via [`axum::extract::State`]. Cheap to clone:
//! [`HttpFetcher`] wraps an `Arc`d `reqwest::Client`, [`PgDatabase`] wraps
//! `sqlx::PgPool` (also `Arc`-internally). Cloning happens once per request.

use crate::canonical::{HttpFetcher, PgDatabase};
use crate::stats::Stats;

#[derive(Clone)]
pub struct AppState {
    pub fetcher: HttpFetcher,
    pub db: PgDatabase,
    pub stats: Stats,
}

impl AppState {
    pub fn new(fetcher: HttpFetcher, db: PgDatabase, stats: Stats) -> Self {
        Self { fetcher, db, stats }
    }
}
