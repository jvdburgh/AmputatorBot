//! [`Database`] — the cache-layer abstraction the canonical engine and HTTP
//! handler write against.
//!
//! Production impl: [`crate::canonical::PgDatabase`] (sqlx + Postgres).
//! Tests use inline mocks per file (`MockDatabase` in the methods/database.rs
//! tests, `EmptyDb` in the resolver tests). The traits stay generic so
//! everything compiles without a live Postgres.
//!
//! Mirrors the [`crate::canonical::PageSource`] pattern.

use std::future::{Future, ready};

use anyhow::Result;

use crate::models::{CanonicalType, EntryType};

/// One row's worth of resolution result, queued for persistence.
///
/// Mirrors the columns the legacy `add_data` in
/// `praw-python-archive/datahandlers/remote_datahandler.py:180+` wrote. `handled_utc`
/// is filled by the DB's `DEFAULT NOW()` — not by Rust — so server clock
/// owns the timestamp and bulk imports can preserve originals via explicit
/// overrides.
#[derive(Debug, Clone)]
pub struct Resolution<'a> {
    /// Where the resolution originated. v1 always sets [`EntryType::Api`];
    /// v2 reads it from the private `X-AmputatorBot-Entry-Type` header (the
    /// Devvit bot and the website tag their requests; direct API callers
    /// fall through to [`EntryType::Api`]). The field is deliberately not
    /// part of the public v2 JSON schema.
    pub entry_type: EntryType,
    /// Which API surface produced this row: `1` for `/api/v1/convert`,
    /// `2` for `/api/v2/convert`. Stored in `links.api_version`. Legacy
    /// CSV imports stay NULL so the new-vs-old boundary is queryable.
    pub api_version: i16,
    pub original_url: &'a str,
    pub canonical_url: Option<&'a str>,
    pub canonical_type: Option<CanonicalType>,
}

/// `Send + Sync` are required so this trait composes with `tokio::spawn`
/// and Axum handlers (futures must be `Send`).
pub trait Database: Send + Sync {
    /// Look up a previously-cached canonical URL for `original_url`.
    ///
    /// Returns `Ok(Some(url))` on cache hit, `Ok(None)` on cache miss, or
    /// `Err(...)` on actual DB failures (connection lost, query syntax,
    /// etc.). The caller decides whether to surface the error or treat it
    /// as a miss — `crate::canonical::methods::database::find` chooses the
    /// latter so DB outages don't crash canonical-finding.
    fn lookup_canonical(
        &self,
        original_url: &str,
    ) -> impl Future<Output = Result<Option<String>>> + Send;

    /// Persist one resolution result to the cache. Ports
    /// `praw-python-archive/datahandlers/remote_datahandler.py:save_entry` — the legacy
    /// bot inserted one row per URL whenever `origin.is_amp` was true,
    /// regardless of whether canonicals were found (caller-side guard).
    ///
    /// Default impl is a no-op so tests that exercise canonical-finding
    /// don't need to care about write-back. Production [`crate::canonical::PgDatabase`]
    /// overrides it with a real INSERT.
    ///
    /// Returns `Err(...)` on DB failures. Callers decide whether to log-and-
    /// swallow (matches legacy `save_entry`'s try/except) or propagate.
    fn record_resolution(&self, _entry: Resolution<'_>) -> impl Future<Output = Result<()>> + Send {
        ready(Ok(()))
    }
}
