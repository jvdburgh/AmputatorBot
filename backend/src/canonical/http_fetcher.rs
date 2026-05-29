//! [`HttpFetcher`] — the production [`crate::canonical::PageSource`] impl.
//!
//! Wraps a shared `reqwest::Client` with sane defaults (timeout, redirect
//! policy, rustls TLS), rotating user-agent per request to reduce upstream
//! 403s. Ports `praw-python-archive/helpers/utils.py:get_page` + `get_randomized_headers`.

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
/// Verified against Mozilla's product-details API on 2026-05-25:
/// - Firefox 151    — current stable
/// - Firefox 150.0  — one version back, still plausible to encounter
/// - Firefox 140    — current ESR
///
/// All Firefox — Mozilla-only, matching the project's anti-AMP ideological
/// alignment (AMP is a Google initiative; we're not pretending to be Chrome).
///
/// Platform coverage:
/// - Linux x86_64 / aarch64 / Ubuntu
/// - macOS 14.6 / 14.7 / 15.0 (Sonoma & Sequoia)
/// - Windows NT 10.0 (10/11) — Win64 + WOW64 variants
/// - Android 14 + 15, Mobile + Tablet form factors
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
/// - Up to 10 redirects followed (Python defaults to ~30 but the URLs we
///   chase rarely need that depth; capping smaller is safer)
/// - rustls TLS backend (from `Cargo.toml`)
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
    fn random_user_agent_returns_a_known_ua() {
        for _ in 0..100 {
            let ua = random_user_agent();
            assert!(USER_AGENTS.contains(&ua));
            assert!(ua.starts_with("Mozilla/"));
        }
    }

    #[test]
    fn random_user_agent_distributes_across_options() {
        // Sanity check that the random source isn't pinned to a single index.
        // With N UAs and 1000 draws, statistically every UA should appear; we
        // assert the weaker invariant that we see *most* of them — leaves
        // headroom for fastrand's PRNG without making the test flaky.
        let mut seen = std::collections::HashSet::new();
        for _ in 0..1000 {
            seen.insert(random_user_agent());
        }
        assert!(
            seen.len() >= USER_AGENTS.len() * 3 / 4,
            "expected variety; got {} of {} UAs",
            seen.len(),
            USER_AGENTS.len()
        );
    }

    #[test]
    fn extract_title_handles_normal_html() {
        let html = r#"<!doctype html><html><head><title>Article Title</title></head><body>...</body></html>"#;
        assert_eq!(extract_title(html), "Article Title");
    }

    #[test]
    fn extract_title_trims_whitespace() {
        let html = "<html><head><title>  spaces all around  </title></head></html>";
        assert_eq!(extract_title(html), "spaces all around");
    }

    #[test]
    fn extract_title_falls_back_when_missing() {
        let html = "<html><body>no head, no title</body></html>";
        assert_eq!(extract_title(html), "Error: Title not found");
    }

    #[test]
    fn extract_title_falls_back_when_empty() {
        let html = "<html><head><title></title></head></html>";
        assert_eq!(extract_title(html), "Error: Title not found");
    }

    #[test]
    fn fetcher_constructs_without_error() {
        let _ = HttpFetcher::new().expect("client should build");
    }
}
