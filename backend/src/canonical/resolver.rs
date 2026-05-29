//! [`resolve`] — the canonical-finding orchestration loop.
//!
//! Ports `praw-python-archive/helpers/utils.py:get_canonicals` (with the per-URL setup
//! from `get_url_info` folded in). Given a URL and the runtime flags, returns
//! a [`Link`] with `canonicals[]`, the best `canonical`, and optionally
//! `amp_canonical` if the origin was an AMP-cache URL we couldn't unwind.
//!
//! Walk-through per [`ResolveOpts::max_depth`] iterations:
//!
//! 1. Fetch the current URL's HTML.
//! 2. For each [`CanonicalType`] in [`CanonicalType::ALL`] order:
//!    - run the method, get candidate URL(s)
//!    - take the first candidate that's (a) different from the input URL
//!      and (b) parses cleanly
//!    - build a [`Canonical`] with metadata: `url_similarity`,
//!      `is_amp`, `is_cached`, `domain`, `is_valid=true`
//!    - if the canonical is **non-AMP**, disable the resource-heavy gated
//!      methods (`use_db`/`use_gac`/`use_mr`) for the rest of this and
//!      subsequent iterations.
//! 3. Sort accumulated canonicals by `url_similarity` descending.
//! 4. If any canonical is non-AMP: pick the best, mark cross-domain
//!    alternates, return.
//! 5. Else (all canonicals are still AMP): if the best one equals the
//!    current URL we made no progress → return (potentially with
//!    `amp_canonical` set if origin was AMP-cache). Otherwise recurse with
//!    the best AMP canonical as the new current URL.

use std::cmp::Ordering;

use url::Url;

use super::amp_detect::{is_amp_url, is_cached_amp};
use super::database::Database;
use super::domain::extract_domain;
use super::methods::{CanonicalFlags, MethodContext, database, guess_and_check, try_method};
use super::page_source::PageSource;
use super::resolve_opts::ResolveOpts;
use crate::models::{Canonical, CanonicalType, Link, UrlMeta};
use crate::readability::article_similarity;

/// Resolve canonicals for `origin_url`.
///
/// Returns a [`Link`] populated with whatever canonicals the bot found.
/// **Never returns an error** — transport failures, AMP-detection misses,
/// and dead-end depth loops all surface as a `Link` whose `canonical`
/// stays `None`. Matches the Python contract.
pub async fn resolve<P: PageSource, D: Database>(
    fetcher: &P,
    db: &D,
    origin_url: &str,
    opts: ResolveOpts,
) -> Link {
    let origin = build_origin_meta(origin_url);
    let mut link = Link::new(origin.clone());

    // Bail if origin isn't a valid AMP URL — non-AMP URLs don't need
    // canonical-finding at all.
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
        // Fetch the page. Transport failures end the loop (Python returns
        // the link as-is too).
        let Ok(page) = fetcher.fetch(&next_url).await else {
            tracing::warn!(%next_url, "fetch failed, ending resolve loop");
            return finalize(link, origin.is_cached);
        };

        // Each method that finds a valid candidate appends one Canonical.
        for method in CanonicalType::ALL {
            let ctx = MethodContext {
                page: &page,
                url: &next_url,
                original_url: origin_url,
                flags,
            };
            if let Some(canonical) = run_method(method, &ctx, fetcher, db, origin_url).await {
                let is_non_amp = canonical.is_amp == Some(false);
                link.canonicals.push(canonical);

                // Disable resource-heavy methods once a non-AMP canonical
                // is in hand — they're not worth re-running on subsequent
                // depth iterations either.
                if is_non_amp {
                    flags.use_db = false;
                    flags.use_gac = false;
                    flags.use_mr = false;
                }
            }
        }

        if link.canonicals.is_empty() {
            return finalize(link, origin.is_cached);
        }

        // Stable sort by url_similarity descending — ties keep discovery order.
        link.canonicals.sort_by(|a, b| {
            b.url_similarity
                .unwrap_or(0.0)
                .partial_cmp(&a.url_similarity.unwrap_or(0.0))
                .unwrap_or(Ordering::Equal)
        });

        if let Some(best_solved_idx) = link.canonicals.iter().position(|c| c.is_amp == Some(false))
        {
            let best = link.canonicals[best_solved_idx].clone();
            mark_alternates(&mut link, &best);
            link.canonical = Some(best);
            return link;
        }

        // All canonicals still AMP. If the best one matches the current URL,
        // recursing would loop — terminate. Otherwise chase the best.
        let best_amp_url = link.canonicals[0].url.clone().unwrap_or_default();
        if best_amp_url == next_url {
            return finalize(link, origin.is_cached);
        }
        next_url = best_amp_url;
    }

    finalize(link, origin.is_cached)
}

/// Build the origin's UrlMeta — valid? AMP? cached? domain?
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

/// Run a single method and return the first valid candidate as a fully-
/// populated `Canonical`. Mirrors the Python `get_canonical_with_soup`
/// behavior of returning one canonical per method invocation.
async fn run_method<P: PageSource, D: Database>(
    method: CanonicalType,
    ctx: &MethodContext<'_>,
    fetcher: &P,
    db: &D,
    origin_url: &str,
) -> Option<Canonical> {
    gather_candidates(method, ctx, fetcher, db)
        .await
        .into_iter()
        .find(|c| is_acceptable_canonical_url(c, ctx.url))
        .map(|c| build_canonical(method, &c, origin_url))
}

/// Dispatch to the method's actual scraper and return its candidate URL list.
///
/// Most methods are synchronous (just-HTML-scraping, handled by
/// [`try_method`]). `GUESS_AND_CHECK` is async because it issues additional
/// HTTP fetches per keyword mutation. `DATABASE` is async because it queries
/// Postgres.
async fn gather_candidates<P: PageSource, D: Database>(
    method: CanonicalType,
    ctx: &MethodContext<'_>,
    fetcher: &P,
    db: &D,
) -> Vec<String> {
    match method {
        CanonicalType::GuessAndCheck => guess_and_check::find(ctx, fetcher)
            .await
            .map(|u| vec![u])
            .unwrap_or_default(),
        CanonicalType::Database => database::find(ctx, db)
            .await
            .map(|u| vec![u])
            .unwrap_or_default(),
        _ => try_method(method, ctx),
    }
}

/// Gate every candidate URL through these checks before treating it as a
/// real canonical:
///
/// 1. **Not equal to the input URL** — same URL means the scraper found a self-reference.
/// 2. **Within the URL-length cap** — see [`crate::canonical::MAX_URL_LEN`]. Mirrors
///    the SQL `CHECK` so we never produce a canonical the DB would reject, and
///    filters out pathological tracking-blob URLs that can't be cached.
/// 3. **Parses as a URL** — discard syntactic garbage.
/// 4. **Has a non-trivial path** — reject root-only URLs (e.g. `https://example.eu`); these are false positives.
fn is_acceptable_canonical_url(candidate: &str, input_url: &str) -> bool {
    if candidate == input_url {
        return false;
    }
    if candidate.len() > super::MAX_URL_LEN {
        return false;
    }
    let Ok(parsed) = Url::parse(candidate) else {
        return false;
    };
    let path = parsed.path();
    !path.is_empty() && path != "/"
}

/// Build a fully-populated [`Canonical`] from a vetted candidate URL.
///
/// Caller is expected to have already passed [`is_acceptable_canonical_url`].
/// Fills:
/// - `url`               — the candidate
/// - `url_similarity`    — Ratcliff-Obershelp against `origin_url`
/// - `is_amp`            — via [`is_amp_url`]
/// - `is_cached`         — only set when `is_amp` is true
/// - `domain`            — eTLD+1 via the `psl` crate
/// - `is_valid = true`   — all candidates that make it here are valid
fn build_canonical(method: CanonicalType, candidate_url: &str, origin_url: &str) -> Canonical {
    let is_amp = is_amp_url(candidate_url);
    let mut canonical = Canonical::for_method(method);
    canonical.url = Some(candidate_url.to_string());
    canonical.url_similarity = Some(article_similarity(candidate_url, origin_url));
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

/// Mark `is_alt = true` on canonicals that represent an alternate cross-
/// domain canonical. Ports `praw-python-archive/helpers/utils.py:136-143`.
///
/// Logic: find the first non-AMP canonical whose domain differs from the
/// chosen best; if one exists, mark every canonical in the list (including
/// AMP ones) that shares the alternate's domain AND similarity score.
fn mark_alternates(link: &mut Link, best: &Canonical) {
    let alt = link
        .canonicals
        .iter()
        .find(|c| c.is_amp == Some(false) && c.domain != best.domain)
        .cloned();

    let Some(alt) = alt else { return };

    for c in &mut link.canonicals {
        if c.domain == alt.domain && c.url_similarity == alt.url_similarity {
            c.is_alt = true;
        }
    }
}

/// End-of-loop cleanup: if no non-AMP canonical was found and the origin
/// was an AMP-cache URL (Google/Bing/ampproject), surface the best AMP
/// canonical as `amp_canonical` so callers still have *something*. Mirrors
/// the Python's behavior in `get_canonicals:148-173`.
fn finalize(mut link: Link, origin_was_cached: Option<bool>) -> Link {
    if link.canonical.is_none() && origin_was_cached == Some(true) && !link.canonicals.is_empty() {
        link.amp_canonical = Some(link.canonicals[0].clone());
    }
    link
}

// ============================================================================
//                                  tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::future::{Future, ready};

    use anyhow::Result;

    use super::*;
    use crate::canonical::Page;
    use crate::canonical::database::Database;
    use crate::canonical::page_source::PageSource;

    /// In-memory `PageSource` for tests. Maps URL strings to pre-built
    /// `Page` objects; unknown URLs return an error (simulates a fetch
    /// failure for negative-path tests).
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

    /// Always-empty [`Database`] for resolver tests that don't exercise the
    /// DATABASE method's hit-path. (The DATABASE method's own tests live in
    /// `methods/database.rs` and use a populated MockDatabase there.)
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
        // Non-AMP origin should short-circuit before any fetch happens.
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
        // AMP page with a clean <link rel="canonical"> to the non-AMP target.
        let amp_url = "https://www.google.com/amp/s/example.eu/article";
        let target = "https://example.eu/article";
        let mock = MockPageSource::new().with(amp_url, &rel_canonical_html(target));

        let link = resolve(&mock, &EmptyDb, amp_url, ResolveOpts::default()).await;

        assert!(
            !link.canonicals.is_empty(),
            "expected at least one canonical"
        );
        let canonical = link.canonical.expect("should have picked a canonical");
        assert_eq!(canonical.url.as_deref(), Some(target));
        assert_eq!(canonical.type_, Some(CanonicalType::Rel));
        assert_eq!(canonical.is_amp, Some(false));
        assert_eq!(canonical.domain.as_deref(), Some("example"));
    }

    #[tokio::test]
    async fn recurses_when_canonical_is_still_amp() {
        // Depth-2 chase: origin is AMP-cache, the first <link rel="canonical">
        // points at another AMP URL (publisher's own /amp/ page), THAT page
        // points at the non-AMP article. The resolver should follow.
        let origin = "https://www.google.com/amp/s/example.eu/article/amp/";
        let intermediate_amp = "https://example.eu/article/amp/";
        let final_canonical = "https://example.eu/article/";

        let mock = MockPageSource::new()
            .with(origin, &rel_canonical_html(intermediate_amp))
            .with(intermediate_amp, &rel_canonical_html(final_canonical));

        let link = resolve(&mock, &EmptyDb, origin, ResolveOpts::default()).await;

        let canonical = link.canonical.expect("should resolve through the chain");
        assert_eq!(canonical.url.as_deref(), Some(final_canonical));
        assert_eq!(canonical.is_amp, Some(false));
    }

    #[tokio::test]
    async fn returns_amp_canonical_when_origin_cached_and_all_amp() {
        // Origin is on a Google AMP cache. After exhausting depth, every
        // canonical we found is still AMP → set amp_canonical so the
        // caller still has something better than the cached URL.
        let origin = "https://www.google.com/amp/s/example.eu/article/amp/";
        let stuck = "https://example.eu/article/amp/";
        let mock = MockPageSource::new()
            .with(origin, &rel_canonical_html(stuck))
            .with(stuck, &rel_canonical_html(stuck)); // self-loop forces dead-end

        let link = resolve(&mock, &EmptyDb, origin, ResolveOpts::default()).await;

        assert!(link.canonical.is_none(), "no non-AMP canonical exists");
        let amp = link.amp_canonical.expect("amp_canonical should fall back");
        assert_eq!(amp.url.as_deref(), Some(stuck));
    }

    #[tokio::test]
    async fn returns_empty_when_fetch_fails() {
        // Empty mock — every fetch fails. Origin parses + detects as AMP,
        // but we can never get HTML to scrape.
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
        // Real case from the parity report: an AMP product page whose
        // template-baked <link rel="canonical" href="/"> would otherwise
        // resolve to the homepage. We drop these.
        let amp = "https://www.example.eu/product/?id=7709294194&amp";
        let html = r#"<!doctype html><html><head>
            <link rel="canonical" href="https://www.example.eu/">
        </head><body>x</body></html>"#;
        let mock = MockPageSource::new().with(amp, html);

        let link = resolve(&mock, &EmptyDb, amp, ResolveOpts::default()).await;
        assert!(
            link.canonical.is_none(),
            "expected no canonical (root URL filtered), got {:?}",
            link.canonical.as_ref().and_then(|c| c.url.as_deref())
        );
        assert!(
            link.canonicals.is_empty(),
            "root URL should not appear in canonicals"
        );
    }

    #[tokio::test]
    async fn rejects_root_only_via_protocol_relative_path() {
        // Edge case: candidate href is `//www.example.eu` (protocol-relative
        // with no path). Should also be filtered.
        let amp = "https://amp.example.eu/article-x";
        let html = r#"<!doctype html><html><head>
            <link rel="canonical" href="//www.example.eu">
        </head></html>"#;
        let mock = MockPageSource::new().with(amp, html);

        let link = resolve(&mock, &EmptyDb, amp, ResolveOpts::default()).await;
        assert!(link.canonical.is_none());
    }

    #[tokio::test]
    async fn rejects_canonical_url_exceeding_max_length() {
        // A <link rel="canonical"> pointing at a 3000-character URL — over the
        // 2048-char cap. Must be filtered: producing it would (a) violate the
        // DB CHECK on insert and (b) almost always indicate a tracking-blob or
        // malformed redirect rather than a real article.
        let amp = "https://www.google.com/amp/s/example.eu/article";
        let long_path = "/article?q=".to_string() + &"x".repeat(3000);
        let too_long = format!("https://example.eu{long_path}");
        assert!(too_long.len() > crate::canonical::MAX_URL_LEN);
        let html = format!(
            r#"<!doctype html><html><head><link rel="canonical" href="{too_long}"></head><body>x</body></html>"#
        );
        let mock = MockPageSource::new().with(amp, &html);

        let link = resolve(&mock, &EmptyDb, amp, ResolveOpts::default()).await;
        assert!(
            link.canonical.is_none(),
            "too-long canonical should be filtered"
        );
        assert!(
            link.canonicals.is_empty(),
            "too-long canonical should not appear in canonicals"
        );
    }

    #[tokio::test]
    async fn sorts_canonicals_by_similarity_descending() {
        // Page with multiple canonical signals pointing at different URLs.
        // After the resolver runs, the most-similar URL to the AMP origin
        // should be link.canonical.
        let amp = "https://www.google.com/amp/s/example.eu/article-2024-tesla";
        let very_similar = "https://example.eu/article-2024-tesla";
        let less_similar = "https://example.eu/totally-different-path";
        let html = format!(
            r#"<!doctype html><html><head>
                <link rel="canonical" href="{very_similar}">
                <meta property="og:url" content="{less_similar}">
            </head><body>x</body></html>"#
        );
        let mock = MockPageSource::new().with(amp, &html);

        let link = resolve(&mock, &EmptyDb, amp, ResolveOpts::default()).await;
        let canonical = link.canonical.expect("should pick one");
        assert_eq!(canonical.url.as_deref(), Some(very_similar));

        // The less-similar one should still be in canonicals[], just not first.
        let urls: Vec<_> = link
            .canonicals
            .iter()
            .filter_map(|c| c.url.as_deref())
            .collect();
        assert!(urls.contains(&very_similar));
        assert!(urls.contains(&less_similar));
    }
}
