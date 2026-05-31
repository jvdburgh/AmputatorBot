//! Reddit reply markdown — the single source of truth for the template the
//! bot posts and the website's "generate Reddit comment" copy-paste box.
//!
//! Originally ported from `praw-python-archive/helpers/reddit/reddit_comment_generator.py`,
//! refreshed during M5 to match the locked v7 template:
//!
//!   - Intro hedges on AMP's speed claim — "AMP is supposed to be faster" —
//!     rather than asserting it.
//!   - When any origin URL was cached, the qualifier ("especially cached
//!     pages like the one you shared") is woven into the same clause as
//!     the privacy/Open-Web link, not appended as a separate sentence.
//!   - Footer drops the legacy "I'm a bot" opener (Reddit's App badge does
//!     bot disclosure now) and the "Summon: u/AmputatorBot" link (doesn't
//!     work in Devvit's per-install model). Footer becomes
//!     `Why & About | r/AmputatorBot | Source`, with the install's
//!     `customFooter` appended on the bot path when the mod set one.
//!
//! Variant selection is by [`EntryType`]:
//!
//! | EntryType  | Voice     | Custom footer? |
//! |------------|-----------|----------------|
//! | Submission | OP posted | yes            |
//! | Comment    | you shared| yes            |
//! | Mention    | you shared| yes            |
//! | Online     | you shared| no             |
//! | Api        | you shared| no             |
//!
//! There used to be a separate TS port at `devvit-app/src/server/core/reply.ts`
//! that the Devvit bot used. It was removed when the API grew a
//! `generateMarkdownComment` field so the resolver can return the markdown
//! alongside the canonical-finding results.

use crate::models::{Canonical, ConfidenceLevel, EntryType, Link};

const FAQ_LINK: &str =
    "https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot";
const SUB_LINK: &str = "https://reddit.com/r/AmputatorBot";
const SOURCE_LINK: &str = "https://github.com/jvdburgh/AmputatorBot";

#[derive(Debug, Clone)]
pub struct BuildReplyOptions {
    /// Where the call originated. Picks the "OP posted" vs "you shared"
    /// voice and decides whether `custom_footer` is allowed (bot paths only).
    pub entry_type: EntryType,
    /// Optional per-install mod-supplied addendum, rendered inside the
    /// superscript footer as ` | <text>`. Ignored on non-bot entry types
    /// (`Api`, `Online`) since those don't have install settings.
    pub custom_footer: Option<String>,
}

/// Build the Reddit reply markdown for a set of resolved links.
///
/// Returns `None` when no link has a canonical (or AMP-canonical fallback)
/// to surface — caller treats `None` as "skip / don't reply".
pub fn build_reply(links: &[Link], options: &BuildReplyOptions) -> Option<String> {
    // One entry per AMP-origin link that produced any canonical (real or AMP-only).
    let mut entries: Vec<String> = Vec::new();
    let mut latest_entry = String::new();
    let mut n_cached = 0usize;

    for link in links {
        if link.origin.is_amp != Some(true) {
            continue;
        }

        if let Some(canonical) = &link.canonical {
            let alt_link = alt_canonical_for(link, canonical)
                .map(|a| {
                    let domain = capitalize(a.domain.as_deref().unwrap_or(""));
                    let url = a.url.as_deref().unwrap_or("");
                    let alt_label = confidence_label(a.confidence_level);
                    format!(" | {domain} canonical: **[{url}]({url})**{alt_label}")
                })
                .unwrap_or_default();
            let url = canonical.url.as_deref().unwrap_or("");
            let label = confidence_label(canonical.confidence_level);
            latest_entry = format!("**[{url}]({url})**{label}{alt_link}");
            entries.push(latest_entry.clone());
        } else if let Some(amp_canonical) = &link.amp_canonical {
            let url = amp_canonical.url.as_deref().unwrap_or("");
            latest_entry = format!(
                "**[{url}]({url})** ^(Still AMP, but no longer cached - unable to process further)"
            );
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
    let (who, what) = subject(options.entry_type);
    let intro_why = build_intro_why(n_amp, n_cached, who, what);

    let (intro_who_what, intro_maybe, canonical_text) = if n_amp == 1 {
        (
            format!("It looks like {who} {what} an AMP link. "),
            "\n\nMaybe check out **the canonical page** instead: ".to_string(),
            latest_entry,
        )
    } else {
        let joined = entries
            .iter()
            .map(|e| format!("\n\n- {e}"))
            .collect::<String>();
        (
            format!("It looks like {who} {what} some AMP links. "),
            "\n\nMaybe check out **the canonical pages** instead: ".to_string(),
            joined,
        )
    };

    let custom_footer_part = match (
        allows_custom_footer(options.entry_type),
        &options.custom_footer,
    ) {
        (true, Some(text)) if !text.is_empty() => format!("^( | ){text}"),
        _ => String::new(),
    };
    let outro = format!(
        "\n\n*****\n\n ^([Why & About]({FAQ_LINK})^( | )[r/AmputatorBot]({SUB_LINK})^( | )[Source]({SOURCE_LINK}){custom_footer_part})"
    );

    Some(format!(
        "{intro_who_what}{intro_why}{intro_maybe}{canonical_text}{outro}"
    ))
}

/// "OP posted" for submissions, "you shared" everywhere else. Mirrors the
/// `Type.SUBMISSION` branch in the legacy `reddit_comment_generator.py`.
fn subject(entry: EntryType) -> (&'static str, &'static str) {
    match entry {
        EntryType::Submission => ("OP", "posted"),
        _ => ("you", "shared"),
    }
}

/// Only bot entry types accept a custom footer — the website and direct
/// API callers don't have per-install settings.
fn allows_custom_footer(entry: EntryType) -> bool {
    matches!(
        entry,
        EntryType::Comment | EntryType::Submission | EntryType::Mention
    )
}

/// Single intro sentence that absorbs the cached-pages qualifier inline.
/// Two reasons it lives in one clause rather than two sentences:
///
///   1. The previous two-sentence shape repeated the same FAQ link twice.
///   2. "Supposed to be faster" is a deliberate hedge — we never assert
///      AMP *is* faster, just that it's marketed that way. Slipping the
///      cached qualifier inside the same clause keeps the tone consistent.
fn build_intro_why(n_amp: usize, n_cached: usize, who: &str, what: &str) -> String {
    let why =
        format!("controversial because of [concerns over privacy and the Open Web]({FAQ_LINK}).");
    if n_cached == 0 {
        return format!("AMP is supposed to be faster, but it's {why}");
    }
    let n_note = if n_amp == 1 && n_cached == 1 {
        "the one"
    } else if n_amp == n_cached {
        "the ones"
    } else {
        "some of the ones"
    };
    format!(
        "AMP is supposed to be faster, but it — especially cached pages like {n_note} {who} {what} — is {why}"
    )
}

/// Pick a cross-domain alternate canonical to surface alongside the primary.
/// Ports the legacy `c_alt` selection (`reddit_comment_generator.py:23-24`):
/// the first non-AMP canonical whose domain differs from the chosen one.
fn alt_canonical_for<'a>(link: &'a Link, primary: &Canonical) -> Option<&'a Canonical> {
    link.canonicals
        .iter()
        .find(|c| c.is_amp == Some(false) && c.domain != primary.domain)
}

/// Inline confidence label rendered alongside each canonical URL in the
/// Reddit comment. Empty when `level` is `None` (e.g. legacy DB rows that
/// pre-date the confidence model).
fn confidence_label(level: Option<ConfidenceLevel>) -> String {
    match level {
        Some(ConfidenceLevel::Verified) => " ^(\u{2014} verified)".to_string(),
        Some(ConfidenceLevel::Likely) => " ^(\u{2014} likely)".to_string(),
        Some(ConfidenceLevel::Unconfirmed) => " ^(\u{2014} unconfirmed)".to_string(),
        None => String::new(),
    }
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
            article_similarity: Some(0.95),
            confidence_score: Some(0.95),
            confidence_level: Some(ConfidenceLevel::Verified),
        }
    }

    fn opts(entry: EntryType) -> BuildReplyOptions {
        BuildReplyOptions {
            entry_type: entry,
            custom_footer: None,
        }
    }

    #[test]
    fn returns_none_when_no_canonicals() {
        let link = Link {
            origin: amp_origin("https://www.google.com/amp/s/example.eu/x", false),
            canonical: None,
            canonicals: vec![],
            amp_canonical: None,
        };
        assert!(build_reply(&[link], &opts(EntryType::Comment)).is_none());
    }

    #[test]
    fn returns_none_when_origin_not_amp() {
        let link = Link {
            origin: UrlMeta {
                is_amp: Some(false),
                ..amp_origin("https://example.eu/x", false)
            },
            canonical: Some(canonical("https://other.example/y", "other", false)),
            canonicals: vec![],
            amp_canonical: None,
        };
        assert!(build_reply(&[link], &opts(EntryType::Comment)).is_none());
    }

    #[test]
    fn singular_comment_uses_you_shared_voice() {
        let amp = "https://www.google.com/amp/s/example.eu/article";
        let canon = "https://example.eu/article";
        let link = Link {
            origin: amp_origin(amp, false),
            canonical: Some(canonical(canon, "example", false)),
            canonicals: vec![canonical(canon, "example", false)],
            amp_canonical: None,
        };
        let reply = build_reply(&[link], &opts(EntryType::Comment)).expect("reply");
        assert!(reply.contains("It looks like you shared an AMP link."));
        assert!(reply.contains("the canonical page"));
        assert!(reply.contains(&format!("**[{canon}]({canon})**")));
        assert!(reply.contains(
            "AMP is supposed to be faster, but it's controversial because of [concerns over privacy and the Open Web]"
        ));
    }

    #[test]
    fn submission_uses_op_posted_voice() {
        let amp = "https://www.google.com/amp/s/example.eu/article";
        let canon = "https://example.eu/article";
        let link = Link {
            origin: amp_origin(amp, false),
            canonical: Some(canonical(canon, "example", false)),
            canonicals: vec![],
            amp_canonical: None,
        };
        let reply = build_reply(&[link], &opts(EntryType::Submission)).expect("reply");
        assert!(reply.contains("It looks like OP posted an AMP link."));
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
        let reply = build_reply(&[l1, l2], &opts(EntryType::Comment)).expect("reply");
        assert!(reply.contains("It looks like you shared some AMP links."));
        assert!(reply.contains("the canonical pages"));
        assert!(reply.contains("- **[https://example.eu/a]"));
        assert!(reply.contains("- **[https://example.eu/b]"));
    }

    #[test]
    fn cached_qualifier_woven_into_intro_when_origin_was_cached() {
        let link = Link {
            origin: amp_origin("https://www.google.com/amp/s/example.eu/x", true),
            canonical: Some(canonical("https://example.eu/x", "example", false)),
            canonicals: vec![],
            amp_canonical: None,
        };
        let reply = build_reply(&[link], &opts(EntryType::Comment)).expect("reply");
        assert!(reply.contains(
            "AMP is supposed to be faster, but it — especially cached pages like the one you shared — is controversial because of [concerns over privacy and the Open Web]"
        ));
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
        let reply = build_reply(&[link], &opts(EntryType::Comment)).expect("reply");
        assert!(reply.contains("Syndicated canonical: **[https://syndicated.partner.example/x]"));
    }

    #[test]
    fn falls_back_to_amp_canonical_when_no_real_canonical() {
        let amp_origin_url = "https://www.google.com/amp/s/example.eu/dead-end";
        let amp_canonical_url = "https://example.eu/dead-end/amp/";
        let link = Link {
            origin: amp_origin(amp_origin_url, true),
            canonical: None,
            canonicals: vec![],
            amp_canonical: Some(canonical(amp_canonical_url, "example", true)),
        };
        let reply = build_reply(&[link], &opts(EntryType::Comment)).expect("reply");
        assert!(reply.contains(
            "**[https://example.eu/dead-end/amp/](https://example.eu/dead-end/amp/)** ^(Still AMP, but no longer cached - unable to process further)"
        ));
    }

    #[test]
    fn custom_footer_appended_inside_superscript_for_bot_entry_types() {
        let amp = "https://www.google.com/amp/s/example.eu/x";
        let link = Link {
            origin: amp_origin(amp, false),
            canonical: Some(canonical("https://example.eu/x", "example", false)),
            canonicals: vec![],
            amp_canonical: None,
        };
        let reply = build_reply(
            &[link],
            &BuildReplyOptions {
                entry_type: EntryType::Comment,
                custom_footer: Some("[Modmail us](https://reddit.com/r/Subreddit)".to_string()),
            },
        )
        .expect("reply");
        assert!(reply.contains("^( | )[Modmail us](https://reddit.com/r/Subreddit))"));
    }

    #[test]
    fn custom_footer_ignored_for_non_bot_entry_types() {
        let amp = "https://www.google.com/amp/s/example.eu/x";
        let link = Link {
            origin: amp_origin(amp, false),
            canonical: Some(canonical("https://example.eu/x", "example", false)),
            canonicals: vec![],
            amp_canonical: None,
        };
        let reply = build_reply(
            &[link],
            &BuildReplyOptions {
                entry_type: EntryType::Online,
                custom_footer: Some("ignored".to_string()),
            },
        )
        .expect("reply");
        assert!(!reply.contains("ignored"));
    }

    #[test]
    fn capitalize_handles_empty_and_unicode() {
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("example"), "Example");
        assert_eq!(capitalize("nyt"), "Nyt");
    }
}
