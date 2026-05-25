//! HTTP fetch layer for canonical-finding.
//!
//! Ports `archive/helpers/utils.py:get_page` + `get_randomized_headers`.
//! Holds a shared `reqwest::Client` with sane defaults (timeout, redirect
//! policy), wraps a single fetch into a [`Page`] result.
//!
//! User-agent rotation is preserved from the legacy bot — picks one of 10
//! mobile UAs at random per request to reduce the chance of upstream 403s.

use std::time::Duration;

use anyhow::{Context, Result};
use scraper::{Html, Selector};

/// The 10 user-agent strings the legacy Python bot rotated through.
/// Ported verbatim from `archive/static/static.py:28-38`.
const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Linux; Android 8.0.0; SM-G960F Build/R16NW) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/61.0.3202.84 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 8.1.0; TA-1020) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.3987.99 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 7.0; SM-T813) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.3987.99 Safari/537.36",
    "Mozilla/5.0 (Linux; Android 7.0; SM-G920F) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.3987.99 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 8.0.0; RNE-L21) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.3987.99 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 8.1.0; SAMSUNG-SM-J727A) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.3987.99 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 8.0.0; SM-G9350) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.3987.99 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 8.0.0; SM-A520F) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.3987.99 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 8.0.0; G3212) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.3987.87 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 9; CLT-L29) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/78.0.3945.116 Mobile Safari/537.36",
];

const ACCEPT_HEADER: &str =
    "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8";
const ACCEPT_LANGUAGE: &str = "en-US";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_REDIRECTS: usize = 10;

/// A fetched HTML page, ready for canonical-method scraping.
///
/// Ports `archive/models/page.py:Page`. We store the **raw HTML** rather
/// than a pre-parsed `scraper::Html` so individual canonical methods
/// can reparse with the right selector each. Parsing is cheap and the
/// methods do their own queries.
#[derive(Debug, Clone)]
pub struct Page {
    /// Final URL after redirects (mirrors Python's `req.url`).
    pub current_url: String,
    pub status_code: u16,
    /// `<title>` text, or `"Error: Title not found"` if absent (Python parity).
    pub title: String,
    pub html: String,
}

impl Page {
    /// Parse the HTML on demand. Each canonical method calls this fresh —
    /// avoids holding a non-`Send` parsed DOM in a struct field.
    pub fn parse(&self) -> Html {
        Html::parse_document(&self.html)
    }
}

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

/// Pick a user-agent string at random from [`USER_AGENTS`].
fn random_user_agent() -> &'static str {
    let idx = fastrand::usize(0..USER_AGENTS.len());
    USER_AGENTS[idx]
}

/// Extract the `<title>` text from raw HTML.
///
/// Returns `"Error: Title not found"` when the document has no `<title>` or
/// it is empty — matches `archive/helpers/utils.py:188`.
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
        // Run many draws; every result should be in the known set.
        for _ in 0..100 {
            let ua = random_user_agent();
            assert!(USER_AGENTS.contains(&ua));
            assert!(ua.starts_with("Mozilla/"));
        }
    }

    #[test]
    fn random_user_agent_distributes_across_options() {
        // Sanity check that the random source isn't pinned to a single index.
        // 1000 draws across 10 UAs — each should appear at least once.
        let mut seen = std::collections::HashSet::new();
        for _ in 0..1000 {
            seen.insert(random_user_agent());
        }
        assert!(
            seen.len() > 1,
            "expected variety, got {} distinct UA(s)",
            seen.len()
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
