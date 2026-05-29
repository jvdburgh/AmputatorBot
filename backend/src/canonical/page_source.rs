//! [`PageSource`] — abstraction over "fetch the HTML of this URL."
//!
//! Production uses [`crate::canonical::HttpFetcher`] (in `http_fetcher.rs`);
//! tests and the M2.10 parity runner use a `MockPageSource` that pulls
//! hand-crafted or recorded HTML from a map. The orchestrator and
//! `guess_and_check` are written against this trait so they don't need to
//! know which fetching strategy is in play.

use std::future::Future;

use anyhow::Result;

use crate::canonical::Page;

/// `Send + Sync` are required for the trait to compose with `tokio::spawn`
/// and `axum` handlers, which require futures to be `Send`.
pub trait PageSource: Send + Sync {
    /// Fetch the page at `url` and return a [`Page`] (final URL after
    /// redirects, status code, title, raw HTML).
    ///
    /// Errors at transport level (DNS, TCP, TLS, timeout) propagate; HTTP
    /// error statuses (4xx/5xx) don't error — they come back in
    /// `Page::status_code`.
    fn fetch(&self, url: &str) -> impl Future<Output = Result<Page>> + Send;
}
