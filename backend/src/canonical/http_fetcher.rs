//! [`HttpFetcher`] — production [`crate::canonical::PageSource`] impl.
//! Shared `reqwest::Client` (15s timeout, 10 redirects, rustls TLS) with a
//! per-request rotating UA.
//!
//! **Known limit:** plain reqwest+rustls has a non-browser TLS fingerprint
//! so Cloudflare-fronted publishers (sky.com, bbc.com…) 403 us regardless
//! of headers. `wreq` with browser emulation was tried and abandoned —
//! beat passive fingerprinting but not JS challenges, and the BoringSSL
//! build cost wasn't worth the marginal gain. The `GUESS_AND_CHECK`
//! URL-transform method is the fallback when the publisher blocks us.

use std::future::Future;
use std::time::Duration;

use anyhow::{Context, Result};
use scraper::{Html, Selector};

use crate::canonical::{Page, PageSource};

/// UAs to rotate through per request. All Firefox (Mozilla-only, matching the
/// project's anti-Google-AMP stance), spread across recent stable + ESR
/// versions and Linux/macOS/Windows/Android so each request looks plausible.
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

/// Cheap to clone (the underlying `reqwest::Client` is `Arc`-internally).
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

    /// `Ok(Page)` on any HTTP response (status code surfaced for the caller
    /// to decide on); `Err` only on transport failures (DNS, TCP, TLS, timeout).
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
        HttpFetcher::fetch(self, url)
    }
}

fn random_user_agent() -> &'static str {
    USER_AGENTS[fastrand::usize(0..USER_AGENTS.len())]
}

/// Returns `"Error: Title not found"` when absent or empty — sentinel value
/// matters because `TCO_PAGETITLE` reads this and we want a clear miss.
fn extract_title(html: &str) -> String {
    static TITLE_SELECTOR: std::sync::LazyLock<Selector> =
        std::sync::LazyLock::new(|| Selector::parse("title").unwrap());

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
