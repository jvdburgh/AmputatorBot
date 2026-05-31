//! [`HttpFetcher`] — the production [`crate::canonical::PageSource`] impl.
//!
//! Wraps a shared `reqwest::Client` with sane defaults (timeout, redirect
//! policy, rustls TLS), rotating user-agent per request to reduce upstream
//! 403s. Ports `praw-python-archive/helpers/utils.py:get_page` + `get_randomized_headers`.
//!
//! ## Limitations (known, accepted)
//!
//! Plain reqwest+rustls has a TLS fingerprint that doesn't match real
//! browsers, and big publishers (sky.com, bbc.com, anything fronted by
//! Cloudflare with aggressive bot rules) 403 us regardless of headers.
//! `wreq` with browser-fingerprint emulation was evaluated and abandoned —
//! it beat passive fingerprinting but not the JS-challenge layer that the
//! same publishers run, and the BoringSSL build cost (~50 MB of native
//! tooling in the Docker stage, ~10× the binary size) wasn't worth the
//! marginal coverage gain. See `GUESS_AND_DONT_CHECK` canonical method for the
//! fallback path when the publisher blocks us.

use std::future::Future;
use std::time::Duration;

use anyhow::{Context, Result};
use scraper::{Html, Selector};

use crate::canonical::{Page, PageSource};

/// User-agent strings rotated through per-request.
///
/// Modernized for 2026 — the legacy Python list (`praw-python-archive/static/static.py:28-38`)
/// was 10 mobile Chrome UAs from Android 7/8/9 with Chrome 61-80, all from
/// 2018-2019. Publishers' anti-bot heuristics flag these as suspicious now.
///
/// Current list: 15 Firefox UAs across platforms + 3 Firefox versions.
/// All Firefox — Mozilla-only, matching the project's anti-AMP ideological
/// alignment (AMP is a Google initiative; we're not pretending to be Chrome).
const USER_AGENTS: &[&str] = &[
    // Firefox 151 — Linux
    "Mozilla/5.0 (X11; Linux x86_64; rv:151.0) Gecko/20100101 Firefox/151.0",
    "Mozilla/5.0 (X11; Linux aarch64; rv:151.0) Gecko/20100101 Firefox/151.0",
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:151.0) Gecko/20100101 Firefox/151.0",
    // Firefox 151 — macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.7; rv:151.0) Gecko/20100101 Firefox/151.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 15.0; rv:151.0) Gecko/20100101 Firefox/151.0",
    // Firefox 151 — Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:151.0) Gecko/20100101 Firefox/151.0",
    "Mozilla/5.0 (Windows NT 10.0; WOW64; rv:151.0) Gecko/20100101 Firefox/151.0",
    // Firefox 151 — Android
    "Mozilla/5.0 (Android 14; Mobile; rv:151.0) Gecko/151.0 Firefox/151.0",
    "Mozilla/5.0 (Android 15; Mobile; rv:151.0) Gecko/151.0 Firefox/151.0",
    "Mozilla/5.0 (Android 14; Tablet; rv:151.0) Gecko/151.0 Firefox/151.0",
    // Firefox 150 — one version back, mix of OSes
    "Mozilla/5.0 (X11; Linux x86_64; rv:150.0) Gecko/20100101 Firefox/150.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
    // Firefox 140 ESR — conservative/enterprise installs
    "Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:140.0) Gecko/20100101 Firefox/140.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.6; rv:140.0) Gecko/20100101 Firefox/140.0",
];

const ACCEPT_HEADER: &str =
    "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8";
const ACCEPT_LANGUAGE: &str = "en-US";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_REDIRECTS: usize = 10;

/// Shared HTTP client for canonical-finding.
///
/// Holds a `reqwest::Client` with:
/// - 15s overall request timeout (matches Python)
/// - Up to 10 redirects followed
/// - rustls TLS backend
///
/// Cheap to clone (the underlying `Client` is `Arc`-internally).
#[derive(Debug, Clone)]
pub struct HttpFetcher {
    client: reqwest::Client,
}

impl HttpFetcher {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
            .user_agent("amputatorbot-backend") // overridden per request below
            .build()
            .context("building reqwest client")?;
        Ok(Self { client })
    }

    /// Fetch `url`. Returns `Ok(Page)` for any successful HTTP response
    /// (regardless of status code — the canonical-finding logic decides what
    /// to do with non-200s). Returns `Err` only for transport-level failures
    /// (DNS, connection, timeout, TLS).
    pub async fn fetch(&self, url: &str) -> Result<Page> {
        let response = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, random_user_agent())
            .header(reqwest::header::ACCEPT, ACCEPT_HEADER)
            .header(reqwest::header::ACCEPT_LANGUAGE, ACCEPT_LANGUAGE)
            .send()
            .await
            .with_context(|| format!("fetching {url}"))?;

        let status_code = response.status().as_u16();
        let final_url = response.url().to_string();
        let html = response
            .text()
            .await
            .with_context(|| format!("reading response body for {url}"))?;

        let title = extract_title(&html);

        Ok(Page {
            current_url: final_url,
            status_code,
            title,
            html,
        })
    }
}

impl PageSource for HttpFetcher {
    fn fetch(&self, url: &str) -> impl Future<Output = Result<Page>> + Send {
        // Delegate to the inherent method on HttpFetcher.
        HttpFetcher::fetch(self, url)
    }
}

/// Pick a user-agent string at random from [`USER_AGENTS`].
fn random_user_agent() -> &'static str {
    let idx = fastrand::usize(0..USER_AGENTS.len());
    USER_AGENTS[idx]
}

/// Extract the `<title>` text from raw HTML.
///
/// Returns `"Error: Title not found"` when the document has no `<title>` or
/// it is empty — matches `praw-python-archive/helpers/utils.py:188`.
fn extract_title(html: &str) -> String {
    static TITLE_SELECTOR: std::sync::LazyLock<Selector> =
        std::sync::LazyLock::new(|| Selector::parse("title").expect("title selector"));

    let doc = Html::parse_document(html);
    doc.select(&TITLE_SELECTOR)
        .next()
        .map(|el| el.text().collect::<String>())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "Error: Title not found".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_title_returns_title_text() {
        let html = r#"<html><head><title>Hello World</title></head></html>"#;
        assert_eq!(extract_title(html), "Hello World");
    }

    #[test]
    fn extract_title_returns_sentinel_when_missing() {
        assert_eq!(extract_title("<html></html>"), "Error: Title not found");
    }
}
