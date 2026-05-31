//! [`Database`] — cache-layer abstraction. Production impl is
//! [`crate::canonical::PgDatabase`] (sqlx + Postgres); tests use inline
//! mocks so the canonical engine compiles without a live DB.

use std::future::{Future, ready};

use anyhow::Result;

use crate::models::{CanonicalType, ConfidenceLevel, EntryType};

/// One row to write to `links`. `handled_utc` is set by the DB's
/// `DEFAULT NOW()` so the server clock owns timestamps; bulk imports can
/// still pass explicit values via direct INSERTs.
#[derive(Debug, Clone)]
pub struct Resolution<'a> {
    /// Where the resolution originated. The Devvit bot and the website tag
    /// their requests via the private `X-AmputatorBot-Entry-Type` header;
    /// direct API callers default to `Api`. Stored for per-source analytics,
    /// not part of the public v2 JSON schema.
    pub entry_type: EntryType,
    /// `1` for `/api/v1/convert`, `2` for `/api/v2/convert`. Pre-v7 CSV
    /// imports stay NULL so the new-vs-old boundary is queryable.
    pub api_version: i16,
    pub original_url: &'a str,
    pub canonical_url: Option<&'a str>,
    pub canonical_type: Option<CanonicalType>,
    pub url_similarity: Option<f64>,
    pub article_similarity: Option<f64>,
    pub confidence_score: Option<f64>,
    pub confidence_level: Option<ConfidenceLevel>,
}

/// `Send + Sync` so this trait composes with `tokio::spawn` and Axum.
pub trait Database: Send + Sync {
    /// `Ok(Some)` on cache hit, `Ok(None)` on miss, `Err` on DB failure.
    /// The DATABASE method swallows errors as cache misses so outages don't
    /// crash canonical-finding.
    fn lookup_canonical(
        &self,
        original_url: &str,
    ) -> impl Future<Output = Result<Option<String>>> + Send;

    /// Persist one resolution to the cache. Default no-op so tests that
    /// don't exercise write-back don't have to mock it; `PgDatabase`
    /// overrides with a real INSERT.
    fn record_resolution(&self, _entry: Resolution<'_>) -> impl Future<Output = Result<()>> + Send {
        ready(Ok(()))
    }
}
