//! `GUESS_AND_CHECK` — the article-similarity fallback method.
//!
//! When the cheap methods (REL, OG_URL, etc.) all fail, mutate the AMP URL
//! by stripping each AMP keyword (e.g. `amp.`, `/amp/`, `?amp=1`) in turn
//! and fetch the result. If the mutated URL serves a page whose article
//! text is sufficiently similar to the AMP page's article text, accept it
//! as the canonical.
//!
//! Async because it issues an extra HTTP fetch per candidate. Not dispatched
//! via [`super::try_method`] — the orchestrator awaits it directly, keeping
//! the sync dispatch fn sync.
//!
//! Gated by `ctx.flags.use_gac`. The orchestrator turns this off after a
//! non-AMP canonical is found in any earlier method, mirroring the legacy
//! Python's `use_gac` flag.
//!
//! Ports `archive/helpers/canonical_methods.py:get_can_url_with_guess_and_check`.

use scraper::{Html, Selector};
use url::Url;

use super::MethodContext;
use crate::canonical::PageSource;
use crate::canonical::amp_detect::AMP_KEYWORDS;
use crate::readability::{article_similarity, extract_article_text};

/// Pure decision: given an origin AMP URL's HTML and a guessed canonical
/// URL's HTML, is the guess accepted as the canonical?
///
/// Returns `Some(guessed_url)` if accepted, `None` otherwise.
///
/// Acceptance rules (port of the Python thresholds verbatim):
/// 1. Both pages produce extractable article text.
/// 2. Article similarity `> 0.6` → high-confidence accept.
/// 3. Or article similarity `> 0.35` AND the guessed page contains
///    `<link rel="amphtml" href="<origin_url>">` (a mutual back-reference
///    confirming this is the same article).
pub fn evaluate_guess(
    origin_url: &str,
    origin_html: &str,
    guessed_url: &str,
    guessed_html: &str,
) -> Option<String> {
    let origin_text = extract_article_text(origin_html)?;
    let guessed_text = extract_article_text(guessed_html)?;
    let similarity = article_similarity(&origin_text, &guessed_text);

    tracing::debug!(%origin_url, %guessed_url, similarity, "guess_and_check evaluated");

    let accepted = similarity > 0.6
        || (similarity > 0.35 && guessed_page_links_back_to_amp(guessed_html, origin_url));

    accepted.then(|| guessed_url.to_string())
}

/// Does the guessed page declare the origin URL as its AMP variant via
/// `<link rel="amphtml" href="...">`? A "yes" is strong evidence both URLs
/// describe the same article — the canonical page is pointing back at the
/// AMP page we started from.
fn guessed_page_links_back_to_amp(guessed_html: &str, origin_url: &str) -> bool {
    static SELECTOR: std::sync::LazyLock<Selector> = std::sync::LazyLock::new(|| {
        Selector::parse("link[rel=amphtml]").expect("amphtml selector")
    });

    let doc = Html::parse_document(guessed_html);
    doc.select(&SELECTOR)
        .filter_map(|el| el.value().attr("href"))
        .any(|href| href == origin_url)
}

/// Async wrapper — runs the guess-and-check loop, fetching each mutation.
///
/// Returns the first accepted canonical, or `None` if every keyword mutation
/// either fails to fetch, returns non-200, or fails the similarity check.
pub async fn find<P: PageSource>(ctx: &MethodContext<'_>, fetcher: &P) -> Option<String> {
    if !ctx.flags.use_gac {
        return None;
    }

    for keyword in AMP_KEYWORDS {
        if !ctx.url.contains(keyword) {
            continue;
        }
        let guessed = ctx.url.replace(keyword, "");
        if Url::parse(&guessed).is_err() {
            continue;
        }

        let Ok(guessed_page) = fetcher.fetch(&guessed).await else {
            continue;
        };
        if guessed_page.status_code != 200 {
            continue;
        }

        if let Some(canonical) =
            evaluate_guess(ctx.url, &ctx.page.html, &guessed, &guessed_page.html)
        {
            return Some(canonical);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Wrap multi-paragraph article text in minimal HTML so dom_smoothie
    // recognizes it as article content (needs at least <html>/<body> + enough
    // paragraph text to clear its scoring heuristics).
    fn article_html(body: &str) -> String {
        format!(
            r#"<!doctype html><html><head><title>x</title></head><body><article>{body}</article></body></html>"#
        )
    }

    fn many_paragraphs() -> String {
        // Five paragraphs of unique content per article. dom_smoothie has
        // length-based heuristics; very short text gets rejected as "thin".
        let p = "<p>Tesla unveiled the Model 3 production line today in Fremont. Workers are assembling vehicles inside a temporary tent erected outside the main factory. Elon Musk announced the milestone on Twitter early in the morning. Production rates have climbed to over five thousand cars per week.</p>";
        format!("{p}{p}{p}{p}{p}")
    }

    #[test]
    fn accepts_when_similarity_above_high_threshold() {
        let origin = article_html(&many_paragraphs());
        let guessed = article_html(&many_paragraphs());
        let result = evaluate_guess(
            "https://amp.example.com/article",
            &origin,
            "https://example.com/article",
            &guessed,
        );
        assert_eq!(result.as_deref(), Some("https://example.com/article"));
    }

    #[test]
    fn rejects_when_similarity_below_low_threshold() {
        let origin = article_html(
            "<p>The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog.</p>",
        );
        let guessed = article_html(
            "<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam.</p>",
        );
        let result = evaluate_guess(
            "https://amp.example.com/x",
            &origin,
            "https://example.com/x",
            &guessed,
        );
        assert!(result.is_none());
    }

    #[test]
    fn accepts_in_mid_band_when_amphtml_links_back() {
        // Build two articles with moderate similarity. Pad with enough shared
        // content to land in 0.35-0.6 range, then prove the rel=amphtml back-
        // reference flips the verdict to accept.
        let shared = "Tesla announced today they have started Model 3 production using new robotic assembly lines in their Fremont factory.";
        let origin = article_html(&format!(
            "<p>{shared}</p><p>This AMP version has additional ad-network paragraphs and tracking widgets injected by the AMP cache that won't appear in the canonical desktop article view.</p><p>More junk text specific to the AMP path that the canonical doesn't contain.</p>"
        ));
        let guessed = format!(
            r#"<!doctype html><html><head><title>x</title><link rel="amphtml" href="https://amp.example.com/article"></head><body><article><p>{shared}</p><p>Desktop version with totally different supporting content compared to the AMP page, including charts and an editorial sidebar that aren't replicated on AMP.</p><p>Another desktop-only paragraph to keep similarity in the mid-band.</p></article></body></html>"#
        );

        // First sanity-check we're in the mid-band; if we're not the test is
        // misconfigured.
        let origin_text = extract_article_text(&origin).expect("origin extracts");
        let guessed_text = extract_article_text(&guessed).expect("guessed extracts");
        let s = article_similarity(&origin_text, &guessed_text);
        assert!(
            (0.35..=0.6).contains(&s),
            "test misconfigured — needs mid-band similarity, got {s}"
        );

        let result = evaluate_guess(
            "https://amp.example.com/article",
            &origin,
            "https://example.com/article",
            &guessed,
        );
        assert_eq!(result.as_deref(), Some("https://example.com/article"));
    }

    #[test]
    fn detects_amphtml_back_reference() {
        let html =
            r#"<html><head><link rel="amphtml" href="https://amp.example.com/x"></head></html>"#;
        assert!(guessed_page_links_back_to_amp(
            html,
            "https://amp.example.com/x"
        ));
        assert!(!guessed_page_links_back_to_amp(html, "https://other.com/x"));
    }

    #[test]
    fn detects_amphtml_back_reference_returns_false_when_absent() {
        let html =
            r#"<html><head><link rel="canonical" href="https://example.com/x"></head></html>"#;
        assert!(!guessed_page_links_back_to_amp(
            html,
            "https://amp.example.com/x"
        ));
    }
}
