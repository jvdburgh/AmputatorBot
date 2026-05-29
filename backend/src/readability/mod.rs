//! Article-text extraction + similarity scoring.
//!
//! Used by the `GUESS_AND_CHECK` canonical-finding method to decide whether
//! a guessed canonical URL points at the same article as the AMP origin.
//!
//! ## How this maps to the legacy Python bot
//!
//! `praw-python-archive/helpers/article_comparer.py` does:
//!
//! ```python
//! article1 = Article(url1).download().parse()
//! article2 = Article(url2).download().parse()
//! return SequenceMatcher(None, article1.text, article2.text).ratio()
//! ```
//!
//! Two pieces map directly:
//!
//! - **Text extraction** (`newspaper3k.Article(...).parse().text`) →
//!   [`extract_article_text`], wrapping `dom_smoothie::Readability`. Both
//!   are Mozilla-Readability-shaped algorithms; the output is the article's
//!   main text content with chrome (nav, ads, footers) stripped.
//!
//! - **Similarity ratio** (`difflib.SequenceMatcher(...).ratio()`) →
//!   [`article_similarity`], wrapping `textdistance::nstr::ratcliff_obershelp`.
//!   Python's `SequenceMatcher` IS the Ratcliff-Obershelp algorithm (a.k.a.
//!   Gestalt pattern matching), so both return values in the same `[0.0, 1.0]`
//!   range with the same scoring. The legacy `>0.6` / `>0.35` thresholds
//!   transfer directly — no need to retune.

use dom_smoothie::Readability;

/// Extract the main article text from an HTML document.
///
/// Returns `None` when the document doesn't parse, has no recognizable
/// article content, or readability extraction otherwise fails.
///
/// Thin wrapper over `dom_smoothie::Readability` — the only thing canonical-
/// finding cares about is the article text, so we don't expose the rest of
/// the `Article` struct (title, byline, lang, …).
pub fn extract_article_text(html: &str) -> Option<String> {
    let mut readability = Readability::new(html, None, None).ok()?;
    let article = readability.parse().ok()?;
    let text = article.text_content.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

/// Compute the similarity between two article texts on `[0.0, 1.0]`.
///
/// Uses the Ratcliff-Obershelp algorithm, byte-identical in scoring to
/// Python's `difflib.SequenceMatcher(None, a, b).ratio()`. The legacy bot
/// uses `> 0.6` as "very likely the same article" and `> 0.35` as "possibly
/// the same article, double-check"; those thresholds carry over without
/// retuning since the score range and distribution are the same.
pub fn article_similarity(a: &str, b: &str) -> f64 {
    textdistance::nstr::ratcliff_obershelp(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_strings_have_similarity_one() {
        assert!((article_similarity("hello world", "hello world") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn disjoint_strings_have_low_similarity() {
        let s = article_similarity(
            "the quick brown fox jumps over the lazy dog",
            "supercalifragilistic expialidocious",
        );
        assert!(s < 0.35, "expected disjoint to score below 0.35, got {s}");
    }

    #[test]
    fn near_duplicate_articles_score_above_six_tenths() {
        // The Python bot's high-confidence threshold. Two articles that share
        // most of their text but differ in trivial ways (one extra paragraph,
        // a couple of typo fixes) should land above 0.6 — matching the
        // legacy behavior that triggers an automatic accept.
        let amp = "Tesla unveiled the Model 3 production line today in Fremont. \
                   Workers are assembling vehicles inside a temporary tent erected outside the main factory. \
                   Elon Musk announced the milestone on Twitter. \
                   Production rates have climbed to over 5,000 cars per week.";
        let non_amp = "Tesla unveiled the Model 3 production line today in Fremont. \
                       Workers are assembling vehicles inside a temporary tent erected outside the main factory. \
                       Elon Musk announced the milestone on Twitter. \
                       Production rates have climbed to roughly 5000 cars per week, sources said.";
        let s = article_similarity(amp, non_amp);
        assert!(
            s > 0.6,
            "near-duplicate articles should score > 0.6, got {s}"
        );
    }

    #[test]
    fn extract_text_returns_main_content() {
        // dom_smoothie's Readability strips chrome and returns the body.
        // We don't assert exact text (different HTML→text strategies might
        // differ on whitespace), just that we get something article-shaped.
        let html = r#"<!doctype html>
            <html>
              <head><title>Test Article</title></head>
              <body>
                <nav>Site navigation here that should be stripped</nav>
                <article>
                  <h1>The Main Article Headline</h1>
                  <p>This is the first paragraph of the article body. It contains
                  enough words to satisfy any minimum-length heuristic the
                  readability extractor might apply when deciding whether this
                  block is article content.</p>
                  <p>A second paragraph keeps the body length reasonable so the
                  extractor recognizes this as a real article rather than a
                  thin page.</p>
                </article>
                <footer>Footer content, also stripped</footer>
              </body>
            </html>"#;

        let text = extract_article_text(html).expect("should extract article text");
        assert!(
            text.contains("first paragraph"),
            "should contain article body, got: {text}"
        );
        assert!(
            !text.contains("Site navigation"),
            "should strip nav, got: {text}"
        );
    }

    #[test]
    fn extract_text_returns_none_on_empty_html() {
        // Empty/minimal HTML has no article — dom_smoothie may parse it but
        // produce empty text. We normalize both cases to None.
        let result = extract_article_text("");
        // Either Err or empty text both collapse to None.
        assert!(result.is_none() || result.as_deref() == Some(""));
    }
}
