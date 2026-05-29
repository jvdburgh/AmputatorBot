//! Extract URLs from comment/post bodies + strip Reddit markdown artifacts.
//!
//! Ports `praw-python-archive/helpers/utils.py:get_urls` (which used the `urlextract`
//! Python crate) and `remove_markdown`. For us, [`linkify`] does the URL
//! extraction — well-maintained Rust crate, drop-in functional replacement.

use std::collections::HashSet;

use linkify::{LinkFinder, LinkKind};

/// Trailing characters that get stripped from extracted URLs.
///
/// These appear when a URL is embedded in Reddit markdown — `[text](url)`
/// leaves a `)` at the end, `(see https://example.eu.)` leaves a `.`,
/// quote-wrapping leaves `"`, etc. The order doesn't matter; the Python
/// version is a tuple `praw-python-archive/helpers/utils.py:70-71`.
const TRAILING_MARKDOWN_CHARS: &[char] = &[
    '?', '(', ')', '[', ']', '\\', ',', '.', '"', '\u{201D}', // U+201D right-double-quote
    '`', '^', '*', '|', '>', '<', '{', '}', '~', ':', ';',
];

/// Extract all unique URLs from a chunk of text, in source order.
///
/// Each URL has trailing markdown punctuation stripped via [`remove_markdown`].
/// Duplicates are dropped (first occurrence wins). Mirrors the
/// `URLExtract(only_unique=True)` behavior of the legacy Python.
pub fn extract_urls(body: &str) -> Vec<String> {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);

    let mut seen = HashSet::new();
    finder
        .links(body)
        .map(|link| remove_markdown(link.as_str()))
        .filter(|url| !url.is_empty())
        .filter(|url| seen.insert(url.clone()))
        .collect()
}

/// Strip trailing markdown punctuation from a URL.
///
/// Ports `praw-python-archive/helpers/utils.py:remove_markdown`. The Python version
/// loops on `url.endswith(markdown_chars)`; we just pop trailing chars
/// while they're in the trailing-chars set.
pub fn remove_markdown(url: &str) -> String {
    let mut s = url.to_string();
    while let Some(last) = s.chars().last() {
        if TRAILING_MARKDOWN_CHARS.contains(&last) {
            s.pop();
        } else {
            break;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_single_url() {
        let urls = extract_urls("check this out: https://example.eu/article");
        assert_eq!(urls, vec!["https://example.eu/article"]);
    }

    #[test]
    fn deduplicates_repeated_urls() {
        let urls = extract_urls("https://example.eu and https://example.eu again");
        assert_eq!(urls, vec!["https://example.eu"]);
    }

    #[test]
    fn preserves_source_order_when_extracting_multiple() {
        let body = "first https://a.example then https://b.example, finally https://c.example";
        let urls = extract_urls(body);
        assert_eq!(
            urls,
            vec![
                "https://a.example",
                "https://b.example",
                "https://c.example"
            ]
        );
    }

    #[test]
    fn strips_trailing_markdown_punctuation() {
        assert_eq!(remove_markdown("https://example.eu."), "https://example.eu");
        assert_eq!(remove_markdown("https://example.eu,"), "https://example.eu");
        assert_eq!(remove_markdown("https://example.eu)"), "https://example.eu");
        assert_eq!(
            remove_markdown("https://example.eu?)"),
            "https://example.eu"
        );
        assert_eq!(
            remove_markdown("https://example.eu\u{201D}"),
            "https://example.eu"
        );
    }

    #[test]
    fn preserves_url_without_trailing_punctuation() {
        assert_eq!(remove_markdown("https://example.eu"), "https://example.eu");
        assert_eq!(
            remove_markdown("https://example.eu/article"),
            "https://example.eu/article"
        );
    }

    #[test]
    fn extracts_from_reddit_link_markdown() {
        // `[text](url)` syntax. linkify captures the URL inside the parens,
        // and remove_markdown strips trailing `)` + `,`.
        let body =
            "see [the article](https://example.eu/news), or directly: https://example.eu/raw";
        let urls = extract_urls(body);
        assert!(urls.contains(&"https://example.eu/news".to_string()));
        assert!(urls.contains(&"https://example.eu/raw".to_string()));
    }

    #[test]
    fn handles_text_without_urls() {
        assert!(extract_urls("no urls here just text").is_empty());
    }

    #[test]
    fn handles_empty_input() {
        assert!(extract_urls("").is_empty());
    }

    #[test]
    fn extracts_unencoded_amp_url_pasted_into_query_string() {
        // The kind of URL that's pasted directly into the API query string
        // (per v7 plan, unencoded URLs work as long as `q` is the last param).
        let body = "https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3/amp/";
        let urls = extract_urls(body);
        assert_eq!(urls.len(), 1);
        assert!(urls[0].contains("/amp/s/electrek.co"));
    }
}
