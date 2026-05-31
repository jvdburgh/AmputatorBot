use std::cmp::Ordering;
use std::collections::HashMap;

use futures::future::join_all;
use url::Url;

use super::amp_detect::{is_amp_url, is_cached_amp};
use super::database::Database;
use super::domain::extract_domain;
use super::methods::guess_and_check::url_is_on_cache_host;
use super::methods::{CanonicalFlags, MethodContext, database, try_method};
use super::page_source::PageSource;
use super::resolve_opts::ResolveOpts;
use crate::models::{Canonical, CanonicalType, ConfidenceLevel, Link, UrlMeta};
use crate::readability::{article_similarity, extract_article_text};

pub async fn resolve<P: PageSource, D: Database>(
    fetcher: &P,
    db: &D,
    origin_url: &str,
    opts: ResolveOpts,
) -> Link {
    let origin = build_origin_meta(origin_url);
    let mut link = Link::new(origin.clone());

    if origin.is_valid != Some(true) || origin.is_amp != Some(true) {
        return link;
    }

    let mut next_url = origin_url.to_string();
    let mut flags = CanonicalFlags {
        use_db: opts.use_db,
        use_gac: opts.use_gac,
        use_mr: opts.use_mr,
    };

    for _depth in 0..opts.max_depth {
        let Ok(page) = fetcher.fetch(&next_url).await else {
            tracing::warn!(%next_url, "fetch failed, ending resolve loop");
            return finalize(link, origin.is_cached);
        };

        // Cache hosts (google.com/amp/s/..., *.cdn.ampproject.org) serve an
        // interstitial here, not the article — comparing its boilerplate to a
        // real canonical's text is meaningless. Skip article-sim; fall back
        // to method + URL signals.
        let origin_article_text = if url_is_on_cache_host(&next_url) {
            None
        } else {
            extract_article_text(&page.html)
        };

        let attributions =
            gather_attributions(&page, &next_url, origin_url, flags, fetcher, db).await;
        if attributions.is_empty() {
            return finalize(link, origin.is_cached);
        }

        let canonicals = score_candidates(
            attributions,
            origin_article_text.as_deref(),
            &next_url,
            fetcher,
        )
        .await;

        for c in &canonicals {
            if c.is_amp == Some(false) {
                flags.use_db = false;
                flags.use_gac = false;
                flags.use_mr = false;
            }
        }
        link.canonicals.extend(canonicals);

        link.canonicals.sort_by(|a, b| {
            b.confidence_score
                .unwrap_or(0.0)
                .partial_cmp(&a.confidence_score.unwrap_or(0.0))
                .unwrap_or(Ordering::Equal)
        });

        if let Some(idx) = link.canonicals.iter().position(|c| c.is_amp == Some(false)) {
            let best = link.canonicals[idx].clone();
            mark_alternates(&mut link, &best);
            link.canonical = Some(best);
            return link;
        }

        let best_amp_url = link.canonicals[0].url.clone().unwrap_or_default();
        if best_amp_url == next_url {
            return finalize(link, origin.is_cached);
        }
        next_url = best_amp_url;
    }

    finalize(link, origin.is_cached)
}

fn build_origin_meta(url: &str) -> UrlMeta {
    let is_valid = Url::parse(url).is_ok();
    if !is_valid {
        return UrlMeta {
            url: Some(url.to_string()),
            is_valid: Some(false),
            ..UrlMeta::default()
        };
    }

    let is_amp = is_amp_url(url);
    UrlMeta {
        domain: if is_amp { extract_domain(url) } else { None },
        is_amp: Some(is_amp),
        is_cached: if is_amp {
            Some(is_cached_amp(url))
        } else {
            None
        },
        is_valid: Some(true),
        url: Some(url.to_string()),
    }
}

/// Run each method, dedupe candidate URLs across methods (highest-priority
/// method wins per URL), and return the ordered (method, url) pairs.
async fn gather_attributions<P: PageSource, D: Database>(
    page: &super::Page,
    next_url: &str,
    origin_url: &str,
    flags: CanonicalFlags,
    fetcher: &P,
    db: &D,
) -> Vec<(CanonicalType, String)> {
    let mut by_url: HashMap<String, CanonicalType> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for method in CanonicalType::ALL {
        let ctx = MethodContext {
            page,
            url: next_url,
            original_url: origin_url,
            flags,
        };
        for url in gather_candidates(method, &ctx, fetcher, db).await {
            if !is_acceptable_canonical_url(&url, next_url, &page.current_url) {
                continue;
            }
            if by_url.contains_key(&url) {
                continue;
            }
            by_url.insert(url.clone(), method);
            order.push(url);
        }
    }

    order
        .into_iter()
        .map(|url| {
            let method = by_url[&url];
            (method, url)
        })
        .collect()
}

async fn gather_candidates<P: PageSource, D: Database>(
    method: CanonicalType,
    ctx: &MethodContext<'_>,
    _fetcher: &P,
    db: &D,
) -> Vec<String> {
    match method {
        CanonicalType::Database => database::find(ctx, db)
            .await
            .map(|u| vec![u])
            .unwrap_or_default(),
        _ => try_method(method, ctx),
    }
}

/// For each (method, url) pair, fetch the candidate, extract article text,
/// compute article_similarity vs the origin's article text, and build a
/// fully-scored Canonical.
async fn score_candidates<P: PageSource>(
    attributions: Vec<(CanonicalType, String)>,
    origin_article_text: Option<&str>,
    next_url: &str,
    fetcher: &P,
) -> Vec<Canonical> {
    let fetches = attributions.iter().map(|(_, url)| fetcher.fetch(url));
    let pages = join_all(fetches).await;

    attributions
        .into_iter()
        .zip(pages)
        .map(|((method, url), page_result)| {
            let candidate_article_text =
                page_result.ok().and_then(|p| extract_article_text(&p.html));
            let article_sim = match (origin_article_text, candidate_article_text.as_deref()) {
                (Some(a), Some(b)) => Some(article_similarity(a, b)),
                _ => None,
            };
            let url_sim = article_similarity(&url, next_url);
            build_canonical(method, &url, url_sim, article_sim)
        })
        .collect()
}

fn is_acceptable_canonical_url(candidate: &str, input_url: &str, page_current_url: &str) -> bool {
    if candidate == input_url || candidate == page_current_url {
        return false;
    }
    if candidate.len() > super::MAX_URL_LEN {
        return false;
    }
    let Ok(parsed) = Url::parse(candidate) else {
        return false;
    };
    // Strip fragment before comparing with current_url — Google interstitials
    // have `<a href="#">` "go back" links that resolve to current_url + "#",
    // which would otherwise sneak through as a self-reference.
    let mut without_fragment = parsed.clone();
    without_fragment.set_fragment(None);
    if without_fragment.as_str() == page_current_url || without_fragment.as_str() == input_url {
        return false;
    }
    let path = parsed.path();
    !path.is_empty() && path != "/"
}

fn build_canonical(
    method: CanonicalType,
    candidate_url: &str,
    url_similarity: f64,
    article_similarity: Option<f64>,
) -> Canonical {
    let is_amp = is_amp_url(candidate_url);
    let score = compute_confidence_score(article_similarity, url_similarity, method);
    let mut canonical = Canonical::for_method(method);
    canonical.url = Some(candidate_url.to_string());
    canonical.url_similarity = Some(url_similarity);
    canonical.article_similarity = article_similarity;
    canonical.confidence_score = Some(score);
    canonical.confidence_level = Some(ConfidenceLevel::from_score(score));
    canonical.is_amp = Some(is_amp);
    canonical.is_cached = if is_amp {
        Some(is_cached_amp(candidate_url))
    } else {
        None
    };
    canonical.domain = extract_domain(candidate_url);
    canonical.is_valid = Some(true);
    canonical
}

fn compute_confidence_score(
    article_similarity: Option<f64>,
    url_similarity: f64,
    method: CanonicalType,
) -> f64 {
    let method_weight = method.confidence_weight();
    match article_similarity {
        Some(article) => 0.7 * article + 0.2 * method_weight + 0.1 * url_similarity,
        None => (0.4 * method_weight + 0.2 * url_similarity).min(0.6),
    }
}

fn mark_alternates(link: &mut Link, best: &Canonical) {
    let alt = link
        .canonicals
        .iter()
        .find(|c| c.is_amp == Some(false) && c.domain != best.domain)
        .cloned();

    let Some(alt) = alt else { return };

    for c in &mut link.canonicals {
        if c.domain == alt.domain && c.confidence_score == alt.confidence_score {
            c.is_alt = true;
        }
    }
}

fn finalize(mut link: Link, origin_was_cached: Option<bool>) -> Link {
    if link.canonical.is_none() && origin_was_cached == Some(true) && !link.canonicals.is_empty() {
        link.amp_canonical = Some(link.canonicals[0].clone());
    }
    link
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::future::{Future, ready};

    use anyhow::Result;

    use super::*;
    use crate::canonical::Page;
    use crate::canonical::database::Database;
    use crate::canonical::page_source::PageSource;

    struct MockPageSource {
        pages: HashMap<String, Page>,
    }

    impl MockPageSource {
        fn new() -> Self {
            Self {
                pages: HashMap::new(),
            }
        }

        fn with(mut self, url: &str, html: &str) -> Self {
            self.pages.insert(
                url.to_string(),
                Page {
                    current_url: url.to_string(),
                    status_code: 200,
                    title: "test".to_string(),
                    html: html.to_string(),
                },
            );
            self
        }

        /// Register a fetch that follows a redirect — `requested_url` is what
        /// the caller asks for; the returned Page reports `final_url` as its
        /// `current_url`, simulating reqwest's redirect-following behavior.
        fn with_redirect(mut self, requested_url: &str, final_url: &str, html: &str) -> Self {
            self.pages.insert(
                requested_url.to_string(),
                Page {
                    current_url: final_url.to_string(),
                    status_code: 200,
                    title: "test".to_string(),
                    html: html.to_string(),
                },
            );
            self
        }

        /// Register a fetch that returns a non-2xx response. Mirrors the
        /// publisher-bot-block case where reqwest still returns `Ok(Page)`
        /// (the HTTP fetcher doesn't error on 4xx/5xx) but the body is
        /// effectively empty for canonical-finding purposes.
        fn with_blocked(mut self, url: &str, status_code: u16) -> Self {
            self.pages.insert(
                url.to_string(),
                Page {
                    current_url: url.to_string(),
                    status_code,
                    title: "Error: Title not found".to_string(),
                    html: String::new(),
                },
            );
            self
        }
    }

    impl PageSource for MockPageSource {
        fn fetch(&self, url: &str) -> impl Future<Output = Result<Page>> + Send {
            let r = self
                .pages
                .get(url)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("MockPageSource: no page registered for {url}"));
            ready(r)
        }
    }

    struct EmptyDb;

    impl Database for EmptyDb {
        fn lookup_canonical(
            &self,
            _original_url: &str,
        ) -> impl Future<Output = Result<Option<String>>> + Send {
            ready(Ok(None))
        }
    }

    fn rel_canonical_html(canonical_href: &str) -> String {
        format!(
            r#"<!doctype html><html><head><link rel="canonical" href="{canonical_href}"></head><body>x</body></html>"#
        )
    }

    #[tokio::test]
    async fn returns_empty_for_non_amp_url() {
        let mock = MockPageSource::new();
        let link = resolve(
            &mock,
            &EmptyDb,
            "https://news.ycombinator.com/item?id=42",
            ResolveOpts::default(),
        )
        .await;

        assert!(link.canonicals.is_empty());
        assert!(link.canonical.is_none());
        assert_eq!(link.origin.is_amp, Some(false));
    }

    #[tokio::test]
    async fn returns_empty_for_malformed_url() {
        let mock = MockPageSource::new();
        let link = resolve(&mock, &EmptyDb, "not a url", ResolveOpts::default()).await;
        assert!(link.canonicals.is_empty());
        assert_eq!(link.origin.is_valid, Some(false));
    }

    #[tokio::test]
    async fn happy_path_rel_canonical_first_iteration() {
        let amp_url = "https://www.google.com/amp/s/example.eu/article";
        let target = "https://example.eu/article";
        let mock = MockPageSource::new()
            .with(amp_url, &rel_canonical_html(target))
            .with(target, "<html><body>x</body></html>");

        let link = resolve(&mock, &EmptyDb, amp_url, ResolveOpts::default()).await;

        assert!(!link.canonicals.is_empty());
        let canonical = link.canonical.expect("should have picked a canonical");
        assert_eq!(canonical.url.as_deref(), Some(target));
        assert_eq!(canonical.type_, Some(CanonicalType::Rel));
        assert_eq!(canonical.is_amp, Some(false));
        assert_eq!(canonical.domain.as_deref(), Some("example"));
        assert!(canonical.confidence_score.is_some());
        assert!(canonical.confidence_level.is_some());
    }

    #[tokio::test]
    async fn recurses_when_canonical_is_still_amp() {
        let origin = "https://www.google.com/amp/s/example.eu/article/amp/";
        let intermediate_amp = "https://example.eu/article/amp/";
        let final_canonical = "https://example.eu/article/";

        let mock = MockPageSource::new()
            .with(origin, &rel_canonical_html(intermediate_amp))
            .with(intermediate_amp, &rel_canonical_html(final_canonical))
            .with(final_canonical, "<html><body>x</body></html>");

        let link = resolve(&mock, &EmptyDb, origin, ResolveOpts::default()).await;

        let canonical = link.canonical.expect("should resolve through the chain");
        assert_eq!(canonical.url.as_deref(), Some(final_canonical));
        assert_eq!(canonical.is_amp, Some(false));
    }

    #[tokio::test]
    async fn returns_empty_when_fetch_fails() {
        let mock = MockPageSource::new();
        let link = resolve(
            &mock,
            &EmptyDb,
            "https://www.google.com/amp/s/example.eu/article",
            ResolveOpts::default(),
        )
        .await;

        assert!(link.canonicals.is_empty());
        assert!(link.canonical.is_none());
        assert_eq!(link.origin.is_amp, Some(true));
    }

    #[tokio::test]
    async fn rejects_root_only_canonical_as_false_positive() {
        let amp = "https://www.example.eu/product/?id=7709294194&amp";
        let html = r#"<!doctype html><html><head>
            <link rel="canonical" href="https://www.example.eu/">
        </head><body>x</body></html>"#;
        let mock = MockPageSource::new().with(amp, html);

        let link = resolve(&mock, &EmptyDb, amp, ResolveOpts::default()).await;
        let saw_root = link
            .canonicals
            .iter()
            .any(|c| c.url.as_deref() == Some("https://www.example.eu/"));
        assert!(
            !saw_root,
            "root-only REL canonical leaked through the filter"
        );
    }

    #[tokio::test]
    async fn rejects_root_only_via_protocol_relative_path() {
        let amp = "https://amp.example.eu/article-x";
        let html = r#"<!doctype html><html><head>
            <link rel="canonical" href="//www.example.eu">
        </head></html>"#;
        let mock = MockPageSource::new().with(amp, html);

        let link = resolve(&mock, &EmptyDb, amp, ResolveOpts::default()).await;
        // Both `https://www.example.eu` and `https://www.example.eu/` are
        // root-only and must be filtered. GAC may produce other candidates
        // (e.g. `example.eu/article-x` from `amp.` subdomain strip) which
        // is fine.
        let saw_root = link.canonicals.iter().any(|c| {
            let url = c.url.as_deref().unwrap_or("");
            url == "https://www.example.eu" || url == "https://www.example.eu/"
        });
        assert!(!saw_root, "root-only candidate leaked");
    }

    #[tokio::test]
    async fn rejects_canonical_url_exceeding_max_length() {
        let amp = "https://www.google.com/amp/s/example.eu/article";
        let long_path = "/article?q=".to_string() + &"x".repeat(3000);
        let too_long = format!("https://example.eu{long_path}");
        assert!(too_long.len() > crate::canonical::MAX_URL_LEN);
        let html = format!(
            r#"<!doctype html><html><head><link rel="canonical" href="{too_long}"></head><body>x</body></html>"#
        );
        let mock = MockPageSource::new().with(amp, &html);

        let link = resolve(&mock, &EmptyDb, amp, ResolveOpts::default()).await;
        // The oversized URL must NOT appear anywhere. GAC may produce other
        // valid candidates (cache-unwrap of the input).
        for c in &link.canonicals {
            let url = c.url.as_deref().unwrap_or("");
            assert!(
                url.len() <= crate::canonical::MAX_URL_LEN,
                "oversized URL leaked: {} chars",
                url.len()
            );
        }
    }

    #[tokio::test]
    async fn sorts_canonicals_by_confidence_descending() {
        let amp = "https://www.google.com/amp/s/example.eu/article-2024-tesla";
        let very_similar = "https://example.eu/article-2024-tesla";
        let less_similar = "https://example.eu/totally-different-path";
        let html = format!(
            r#"<!doctype html><html><head>
                <link rel="canonical" href="{very_similar}">
                <meta property="og:url" content="{less_similar}">
            </head><body>x</body></html>"#
        );
        let mock = MockPageSource::new()
            .with(amp, &html)
            .with(very_similar, "<html><body>x</body></html>")
            .with(less_similar, "<html><body>x</body></html>");

        let link = resolve(&mock, &EmptyDb, amp, ResolveOpts::default()).await;
        let canonical = link.canonical.expect("should pick one");
        assert_eq!(canonical.url.as_deref(), Some(very_similar));

        let urls: Vec<_> = link
            .canonicals
            .iter()
            .filter_map(|c| c.url.as_deref())
            .collect();
        assert!(urls.contains(&very_similar));
        assert!(urls.contains(&less_similar));
    }

    #[test]
    fn confidence_score_with_article_match() {
        let score = compute_confidence_score(Some(0.95), 0.93, CanonicalType::Rel);
        assert!(
            (0.95 - 0.001..=0.95 + 0.1).contains(&score),
            "expected ~0.95, got {score}"
        );
        assert_eq!(
            ConfidenceLevel::from_score(score),
            ConfidenceLevel::Verified
        );
    }

    #[test]
    fn confidence_score_capped_at_0_6_when_no_article() {
        // Even with perfect method weight + URL similarity, "no article
        // similarity computed" must never reach the "verified" tier.
        let score = compute_confidence_score(None, 1.0, CanonicalType::Rel);
        assert!(score <= 0.6, "expected ≤ 0.6, got {score}");
        assert_ne!(
            ConfidenceLevel::from_score(score),
            ConfidenceLevel::Verified
        );
    }

    #[test]
    fn confidence_score_unconfirmed_for_strip_only() {
        let score = compute_confidence_score(None, 0.4, CanonicalType::GuessAndCheck);
        assert_eq!(
            ConfidenceLevel::from_score(score),
            ConfidenceLevel::Unconfirmed
        );
    }

    /// Regression test for the Google interstitial self-reference bug:
    /// the interstitial page has an `<a href="#">` "go back" anchor that
    /// resolves against the page's URL → the resolved URL is the interstitial
    /// itself (with a `#` fragment). Fetched against itself it scores
    /// article_similarity = 1.0 trivially and would win as the canonical.
    /// Filter rejects candidates equal to either `next_url` or
    /// `page.current_url`, so the self-ref is dropped before scoring.
    #[tokio::test]
    async fn filters_self_reference_to_interstitial() {
        let requested = "https://www.google.com/amp/s/example.eu/article";
        let interstitial = "https://www.google.com/url?q=https://example.eu/article";
        let real_canonical = "https://example.eu/article";
        let interstitial_html = format!(
            r##"<html><body>
                <a href="{real_canonical}">go</a>
                <a href="#">back</a>
            </body></html>"##
        );
        let mock = MockPageSource::new()
            .with_redirect(requested, interstitial, &interstitial_html)
            .with(real_canonical, "<html><body>x</body></html>");

        let link = resolve(&mock, &EmptyDb, requested, ResolveOpts::default()).await;
        let canonical = link.canonical.expect("should pick a canonical");
        assert_eq!(canonical.url.as_deref(), Some(real_canonical));
        // The self-ref interstitial URL (with `#`) must NOT appear anywhere.
        for c in &link.canonicals {
            let url = c.url.as_deref().unwrap_or("");
            assert!(!url.contains("/url?q="), "self-ref leaked: {url}");
            assert!(!url.ends_with('#'), "self-ref-with-fragment leaked: {url}");
        }
    }

    /// Cache-host inputs must not produce a misleading 2%-ish article
    /// similarity. The cache returns an interstitial, not the article — so
    /// the comparison would be `tiny boilerplate vs real article` ≈ 0.
    /// Filter sets `article_similarity = None` so the result lands at
    /// `LIKELY` via the method+url formula rather than `UNCONFIRMED` via a
    /// meaninglessly-low similarity number.
    #[tokio::test]
    async fn cache_host_input_skips_article_similarity() {
        let requested = "https://www.google.com/amp/s/example.eu/article";
        let interstitial = "https://www.google.com/url?q=https://example.eu/article";
        let real_canonical = "https://example.eu/article";
        let interstitial_html = format!(
            r##"<html><body><p>Notice for redirect</p>
                <a href="{real_canonical}">go</a></body></html>"##
        );
        let canonical_html =
            "<html><body><article>This is the real article body content that goes on for a while and contains lots of words.</article></body></html>"
                .to_string();

        let mock = MockPageSource::new()
            .with_redirect(requested, interstitial, &interstitial_html)
            .with(real_canonical, &canonical_html);

        let link = resolve(&mock, &EmptyDb, requested, ResolveOpts::default()).await;
        let canonical = link.canonical.expect("should pick the canonical");
        assert_eq!(canonical.url.as_deref(), Some(real_canonical));
        // article_similarity stays None — comparison was structurally invalid.
        assert!(
            canonical.article_similarity.is_none(),
            "article_similarity should be None for cache-host origin, got {:?}",
            canonical.article_similarity
        );
        // Without article verification, max confidence is Likely (capped at 0.6
        // by the formula).
        assert_eq!(canonical.confidence_level, Some(ConfidenceLevel::Likely));
    }

    /// The "publisher blocked, strip wins" scenario for sky.com / bbc.com /
    /// other Cloudflare-fronted publishers. The resolver iterates to the
    /// publisher's URL on iteration 1, sky.com 403s, no fetch-based method
    /// can extract anything from the empty body, but GUESS_AND_CHECK's
    /// URL-only path-strip produces a non-AMP candidate. That candidate
    /// surfaces with `Unconfirmed` confidence (we couldn't verify article
    /// content) rather than being dropped.
    #[tokio::test]
    async fn strip_wins_unconfirmed_when_publisher_blocks_fetch() {
        let google_cache = "https://www.google.com/amp/s/news.example.com/story/amp/article-123";
        let publisher_amp = "https://news.example.com/story/amp/article-123";
        let publisher_clean = "https://news.example.com/story/article-123";
        // The interstitial URL doesn't need real percent-encoding for this
        // test — what matters is that it (a) differs from `google_cache` and
        // (b) the path matches `url?q=` so GMR's trigger fires.
        let interstitial =
            "https://www.google.com/url?q=https%3A//news.example.com/story/amp/article-123";

        let interstitial_html =
            format!(r#"<html><body><a href="{publisher_amp}">go</a></body></html>"#);

        let mock = MockPageSource::new()
            .with_redirect(google_cache, interstitial, &interstitial_html)
            // Iteration 1: publisher AMP page is blocked (403)
            .with_blocked(publisher_amp, 403)
            // The strip's candidate is ALSO blocked
            .with_blocked(publisher_clean, 403);

        let link = resolve(&mock, &EmptyDb, google_cache, ResolveOpts::default()).await;
        let canonical = link
            .canonical
            .expect("should surface the strip's non-AMP candidate");
        assert_eq!(canonical.url.as_deref(), Some(publisher_clean));
        assert_eq!(canonical.is_amp, Some(false));
        assert_eq!(
            canonical.confidence_level,
            Some(ConfidenceLevel::Unconfirmed),
            "blocked publisher + no article verification → Unconfirmed"
        );
        assert!(canonical.article_similarity.is_none());
    }
}
