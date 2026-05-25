//! [`Database`] — abstraction over "look up a cached canonical for this URL."
//!
//! Production uses [`crate::canonical::PgDatabase`] (in `pg_database.rs`);
//! tests use a `MockDatabase` defined inline per test file. The orchestrator
//! and the DATABASE canonical method are written against this trait so
//! every existing unit test can run without a live Postgres.
//!
//! Mirrors the [`crate::canonical::PageSource`] pattern.

use std::future::Future;

use anyhow::Result;

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
}
