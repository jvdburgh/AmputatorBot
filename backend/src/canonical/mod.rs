//! Canonical-finding engine.
//!
//! This module ports `praw-python-archive/helpers/canonical_methods.py` and
//! `praw-python-archive/helpers/utils.py:get_canonicals` from the legacy Python bot.
//!
//! The 11 canonical methods (10 + DATABASE cache lookup) are tried in
//! priority order — see [`crate::models::CanonicalType::ALL`].

/// Maximum length (in characters) for any URL the bot will accept as a
/// canonical, or persist to the `links` cache. Mirrors the SQL
/// `CHECK (length(...) <= 2048)` in `backend/migrations/001_initial.sql` —
/// keep both in sync.
///
/// Rationale: matches the Sitemaps protocol cap (the only widely-adopted
/// formal standard for URL length), stays under Postgres' btree max
/// (~2704 bytes for an indexed value), and rejects pathological URLs
/// that are almost always junk (tracking blobs, malformed forwards).
pub const MAX_URL_LEN: usize = 2048;

pub mod amp_detect;
pub mod database;
pub mod domain;
pub mod http_fetcher;
pub mod methods;
pub mod page;
pub mod page_source;
pub mod pg_database;
pub mod resolve_opts;
pub mod resolver;
pub mod url_extract;

pub use amp_detect::{is_amp_url, is_cached_amp};
pub use database::Database;
pub use domain::extract_domain;
pub use http_fetcher::HttpFetcher;
pub use methods::{CanonicalFlags, MethodContext, try_method};
pub use page::Page;
pub use page_source::PageSource;
pub use pg_database::PgDatabase;
pub use resolve_opts::ResolveOpts;
pub use resolver::resolve;
pub use url_extract::{extract_urls, remove_markdown};
