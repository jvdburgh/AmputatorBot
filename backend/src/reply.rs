//! Reddit reply markdown — port of `praw-python-archive/helpers/reddit/reddit_comment_generator.py`.
//!
//! The function lives here as a reference implementation for M5, when the
//! Devvit (TypeScript) side will produce its own reply text. Keeping the
//! Rust port lets us A/B the formats and gives the API a path for the
//! `gc=true` query param if/when a caller actually needs it.
//!
//! Scope for M3: the `from_online=True` branch only. That's the one the
//! legacy code used for human-facing surfaces (the web form + summon
//! replies on the API side). M5 extends to the bot path (`from_online=False`)
//! once we're staring at real Reddit replies and can refresh the template.

use crate::models::{Canonical, Link};

/// Why include these links inline rather than read them from `static.py`?
/// Because the legacy strings have been stable for years, and they're the
/// only thing this reference port emits. M5 will rewrite the template
/// anyway, so a config layer would just be premature.
const FAQ_LINK: &str =
    "https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot";

/// Build the Reddit reply markdown for a set of resolved links.
///
/// Returns `None` when no link has a canonical (or AMP-canonical fallback)
/// to surface — matches the Python's "log a warning and return None"
/// behavior. Caller decides whether to log + skip or surface the absence.
///
/// Output is the human-facing/`from_online=True` template only — the
/// "human | Generated with AmputatorBot" footer variant. The bot footer
/// variant lands with M5.
pub fn build_reply(links: &[Link]) -> Option<String> {
    // Collect everything we'll need to assemble the canonical-text block.
    // One entry per AMP-origin link that produced any canonical (real or AMP-only).
    let mut entries: Vec<String> = Vec::new();
    let mut latest_entry = String::new();
    let mut n_cached = 0usize;

    for link in links {
        let origin_is_amp = link.origin.is_amp == Some(true);
        if !origin_is_amp {
            continue;
        }

        if let Some(canonical) = &link.canonical {
            let alt = alt_canonical_for(link, canonical);
            let alt_link = alt
                .map(|a| {
                    let domain = capitalize(a.domain.as_deref().unwrap_or(""));
                    let url = a.url.as_deref().unwrap_or("");
                    format!(" | {domain} canonical: **[{url}]({url})**")
                })
                .unwrap_or_default();
            let url = canonical.url.as_deref().unwrap_or("");
            latest_entry = format!("**[{url}]({url})**{alt_link}");
            entries.push(latest_entry.clone());
        } else if let Some(amp_canonical) = &link.amp_canonical {
            let url = amp_canonical.url.as_deref().unwrap_or("");
            let amp_tho = " ^(Still AMP, but no longer cached - unable to process further)";
            latest_entry = format!("**[{url}]({url})**{amp_tho}");
            entries.push(latest_entry.clone());
        }

        if link.origin.is_cached == Some(true) {
            n_cached += 1;
        }
    }

    if entries.is_empty() {
        return None;
    }

    let n_amp = entries.len();
    let intro_why = format!(
        "These should load faster, but AMP is controversial because of \
         [concerns over privacy and the Open Web]({FAQ_LINK})."
    );

    // Singular vs. plural framing — match the legacy phrasing exactly.
    let (intro_who_what, intro_maybe, canonical_text) = if n_amp == 1 {
        (
            "It looks like you shared an AMP link. ",
            "\n\nMaybe check out **the canonical page** instead: ",
            latest_entry,
        )
    } else {
        let joined = entries
            .iter()
            .map(|e| format!("\n\n- {e}"))
            .collect::<String>();
        (
            "It looks like you shared some AMP links. ",
            "\n\nMaybe check out **the canonical pages** instead: ",
            joined,
        )
    };

    let cached_note = build_cached_note(n_amp, n_cached);

    // Bot/human footer — `from_online=true` arm only.
    let outro = format!(
        "\n\n*****\n\n ^(I'm a human | Generated with AmputatorBot | )\
         [^(Why & About)]({FAQ_LINK})"
    );

    Some(format!(
        "{intro_who_what}{intro_why}{cached_note}{intro_maybe}{canonical_text}{outro}"
    ))
}

/// Pick a cross-domain alternate canonical to surface alongside the primary.
///
/// Ports the legacy `c_alt` selection (`reddit_comment_generator.py:23-24`):
/// the first non-AMP canonical whose domain differs from the chosen one.
fn alt_canonical_for<'a>(link: &'a Link, primary: &Canonical) -> Option<&'a Canonical> {
    link.canonicals
        .iter()
        .find(|c| c.is_amp == Some(false) && c.domain != primary.domain)
}

/// "Fully cached AMP pages…" sentence. Empty string when no cached AMPs in
/// the batch. Pluralization mirrors the legacy phrasing exactly so output
/// is byte-identical on the common cases.
fn build_cached_note(n_amp: usize, n_cached: usize) -> String {
    if n_cached == 0 {
        return String::new();
    }
    let n_note = if n_amp == 1 && n_cached == 1 {
        "the one"
    } else if n_amp == n_cached {
        "the ones"
    } else {
        "some of the ones"
    };
    format!(
        " Fully cached AMP pages (like {n_note} you shared), \
         are [especially problematic]({FAQ_LINK})."
    )
}

/// ASCII-only capitalize. The legacy `domain.capitalize()` is Python's
/// equivalent — uppercase first char, lowercase the rest. Domain strings
/// in our data are ASCII-only (we extract via `psl`), so this is fine.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => {
            let mut out = first.to_ascii_uppercase().to_string();
            out.push_str(&chars.as_str().to_ascii_lowercase());
            out
        }
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Canonical, CanonicalType, UrlMeta};

    fn amp_origin(url: &str, cached: bool) -> UrlMeta {
        UrlMeta {
            url: Some(url.to_string()),
            domain: Some("example".to_string()),
            is_amp: Some(true),
            is_cached: Some(cached),
            is_valid: Some(true),
        }
    }

    fn canonical(url: &str, domain: &str, is_amp: bool) -> Canonical {
        Canonical {
            url: Some(url.to_string()),
            domain: Some(domain.to_string()),
            is_amp: Some(is_amp),
            is_cached: if is_amp { Some(false) } else { None },
            is_valid: Some(true),
            is_alt: false,
            type_: Some(CanonicalType::Rel),
            url_similarity: Some(1.0),
        }
    }

    #[test]
    fn returns_none_when_no_canonicals() {
        // Origin is AMP but no canonical was found — caller should skip
        // replying entirely.
        let link = Link {
            origin: amp_origin("https://www.google.com/amp/s/example.eu/x", false),
            canonical: None,
            canonicals: vec![],
            amp_canonical: None,
        };
        assert!(build_reply(&[link]).is_none());
    }

    #[test]
    fn returns_none_when_origin_not_amp() {
        // Non-AMP origin shouldn't appear in a reply.
        let link = Link {
            origin: UrlMeta {
                is_amp: Some(false),
                ..amp_origin("https://example.eu/x", false)
            },
            canonical: Some(canonical("https://other.example/y", "other", false)),
            canonicals: vec![],
            amp_canonical: None,
        };
        assert!(build_reply(&[link]).is_none());
    }

    #[test]
    fn singular_reply_one_canonical() {
        let canonical_url = "https://example.eu/article";
        let amp = "https://www.google.com/amp/s/example.eu/article";

        let link = Link {
            origin: amp_origin(amp, false),
            canonical: Some(canonical(canonical_url, "example", false)),
            canonicals: vec![canonical(canonical_url, "example", false)],
            amp_canonical: None,
        };

        let reply = build_reply(&[link]).expect("expected reply");

        assert!(
            reply.contains("It looks like you shared an AMP link."),
            "singular intro missing: {reply}"
        );
        assert!(
            reply.contains("the canonical page"),
            "singular noun missing: {reply}"
        );
        assert!(
            reply.contains(&format!("**[{canonical_url}]({canonical_url})**")),
            "canonical link missing: {reply}"
        );
        assert!(
            reply.contains("I'm a human | Generated with AmputatorBot"),
            "from_online footer missing: {reply}"
        );
    }

    #[test]
    fn plural_reply_multiple_canonicals() {
        let l1 = Link {
            origin: amp_origin("https://www.google.com/amp/s/example.eu/a", false),
            canonical: Some(canonical("https://example.eu/a", "example", false)),
            canonicals: vec![],
            amp_canonical: None,
        };
        let l2 = Link {
            origin: amp_origin("https://www.google.com/amp/s/example.eu/b", false),
            canonical: Some(canonical("https://example.eu/b", "example", false)),
            canonicals: vec![],
            amp_canonical: None,
        };

        let reply = build_reply(&[l1, l2]).expect("expected reply");

        assert!(
            reply.contains("It looks like you shared some AMP links."),
            "plural intro missing: {reply}"
        );
        assert!(
            reply.contains("the canonical pages"),
            "plural noun missing: {reply}"
        );
        assert!(reply.contains("- **[https://example.eu/a]"));
        assert!(reply.contains("- **[https://example.eu/b]"));
    }

    #[test]
    fn appends_cached_note_when_origin_was_cached() {
        let link = Link {
            origin: amp_origin("https://www.google.com/amp/s/example.eu/x", true),
            canonical: Some(canonical("https://example.eu/x", "example", false)),
            canonicals: vec![],
            amp_canonical: None,
        };

        let reply = build_reply(&[link]).expect("expected reply");
        assert!(
            reply.contains("Fully cached AMP pages (like the one you shared)"),
            "cached-note missing: {reply}"
        );
    }

    #[test]
    fn surfaces_cross_domain_alt_canonical() {
        let amp = "https://www.google.com/amp/s/example.eu/x";
        let primary = canonical("https://example.eu/x", "example", false);
        let alt = canonical("https://syndicated.partner.example/x", "syndicated", false);

        let link = Link {
            origin: amp_origin(amp, false),
            canonical: Some(primary.clone()),
            canonicals: vec![primary, alt],
            amp_canonical: None,
        };

        let reply = build_reply(&[link]).expect("expected reply");
        assert!(
            reply.contains("Syndicated canonical: **[https://syndicated.partner.example/x]"),
            "alt-canonical block missing: {reply}"
        );
    }

    #[test]
    fn falls_back_to_amp_canonical_when_no_real_canonical() {
        // Origin was a cached AMP page, the resolver couldn't reach a
        // non-AMP version — we still produce a reply pointing at the
        // best AMP we found.
        let amp_origin_url = "https://www.google.com/amp/s/example.eu/dead-end";
        let amp_canonical_url = "https://example.eu/dead-end/amp/";

        let link = Link {
            origin: amp_origin(amp_origin_url, true),
            canonical: None,
            canonicals: vec![],
            amp_canonical: Some(canonical(amp_canonical_url, "example", true)),
        };

        let reply = build_reply(&[link]).expect("expected reply");
        assert!(
            reply.contains(&format!(
                "**[{amp_canonical_url}]({amp_canonical_url})** \
                 ^(Still AMP, but no longer cached - unable to process further)"
            )),
            "amp-canonical fallback missing: {reply}"
        );
    }

    #[test]
    fn capitalize_handles_empty_and_unicode() {
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("example"), "Example");
        assert_eq!(capitalize("nyt"), "Nyt");
    }
}
