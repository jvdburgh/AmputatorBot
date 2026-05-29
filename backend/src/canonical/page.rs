//! [`Page`] — one fetched HTML response, ready for canonical-method scraping.

use scraper::Html;

/// A fetched HTML page.
///
/// Ports `praw-python-archive/models/page.py:Page`. We store the **raw HTML** rather
/// than a pre-parsed `scraper::Html` so individual canonical methods can
/// reparse with the right selector each. Parsing is cheap and the methods
/// do their own queries.
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
