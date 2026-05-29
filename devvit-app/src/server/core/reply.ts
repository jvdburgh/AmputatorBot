// Reddit reply markdown for the Devvit bot path.
//
// Ports `praw-python-archive/helpers/reddit/reddit_comment_generator.py` and tracks
// `backend/src/reply.rs` (the human-facing variant). Two locked deviations
// from the legacy template (see "Reply markdown (locked)" in the v7 plan):
//
//   1. The disclaimer line changes from
//        "These should load faster, but AMP is controversial because of …"
//      to
//        "AMP is supposed to be faster, but it's controversial because of …"
//      Reddit users mostly know AMP exists and is bad; the new wording
//      doesn't pretend otherwise.
//
//   2. The footer drops the `I'm a bot` opener (Reddit's App badge now
//      surfaces bot identity) and the `Summon: u/AmputatorBot` link
//      (doesn't work in Devvit's per-install model). Footer becomes
//      `Why & About | r/AmputatorBot | Source`, with the install's
//      `customFooter` appended when set.

import type { Canonical, Link } from '../backend/types.ts';

const FAQ_LINK =
  'https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot';
const SUB_LINK = 'https://reddit.com/r/AmputatorBot';
const SOURCE_LINK = 'https://github.com/KilledMufasa/AmputatorBot';

export type TriggerType = 'comment' | 'post';

export type BuildReplyOptions = {
  // 'comment' → "you shared", 'post' → "OP posted". Mirrors the
  // `Type.SUBMISSION` branch in `reddit_comment_generator.py`.
  triggerType: TriggerType;
  // Optional per-install footer extension. Rendered inside the superscript
  // group as ` | <customFooter>` to match the rest of the line's style.
  customFooter?: string;
};

// Build the Reddit reply markdown. Returns null when nothing replyable was
// found — the caller (the trigger handler) treats null as "skip this entry".
export function buildReply(links: Link[], options: BuildReplyOptions): string | null {
  // Per-AMP-origin entries, in order. The Rust port collects these into
  // `entries` + `latest_entry`; we do the same so plural-vs-singular formatting
  // works identically.
  const entries: string[] = [];
  let latestEntry = '';
  let nCached = 0;

  for (const link of links) {
    if (link.origin.isAmp !== true) continue;

    if (link.canonical) {
      const altBlock = altCanonicalFor(link, link.canonical);
      const url = link.canonical.url ?? '';
      const entry = `**[${url}](${url})**${altBlock}`;
      latestEntry = entry;
      entries.push(entry);
    } else if (link.ampCanonical) {
      const url = link.ampCanonical.url ?? '';
      const ampTho = ' ^(Still AMP, but no longer cached - unable to process further)';
      latestEntry = `**[${url}](${url})**${ampTho}`;
      entries.push(latestEntry);
    }

    if (link.origin.isCached === true) {
      nCached += 1;
    }
  }

  if (entries.length === 0) return null;

  const { who, what } = subject(options.triggerType);
  const nAmp = entries.length;

  const introWhy = `AMP is supposed to be faster, but it's controversial because of [concerns over privacy and the Open Web](${FAQ_LINK}).`;
  const cachedNote = buildCachedNote(nAmp, nCached, who, what);

  let introWhoWhat: string;
  let introMaybe: string;
  let canonicalText: string;
  if (nAmp === 1) {
    introWhoWhat = `It looks like ${who} ${what} an AMP link. `;
    introMaybe = '\n\nMaybe check out **the canonical page** instead: ';
    canonicalText = latestEntry;
  } else {
    introWhoWhat = `It looks like ${who} ${what} some AMP links. `;
    introMaybe = '\n\nMaybe check out **the canonical pages** instead: ';
    canonicalText = entries.map((e) => `\n\n- ${e}`).join('');
  }

  const customFooterPart = options.customFooter ? `^( | )${options.customFooter}` : '';
  const outro = `\n\n*****\n\n ^([Why & About](${FAQ_LINK})^( | )[r/AmputatorBot](${SUB_LINK})^( | )[Source](${SOURCE_LINK})${customFooterPart})`;

  return `${introWhoWhat}${introWhy}${cachedNote}${introMaybe}${canonicalText}${outro}`;
}

function subject(t: TriggerType): { who: string; what: string } {
  if (t === 'post') return { who: 'OP', what: 'posted' };
  return { who: 'you', what: 'shared' };
}

// First non-AMP canonical whose domain differs from the primary —
// ports `reddit_comment_generator.py:23-24` and `backend/src/reply.rs`'s
// `alt_canonical_for`. Empty string when there's no cross-domain alt.
function altCanonicalFor(link: Link, primary: Canonical): string {
  const alt = link.canonicals.find((c) => c.isAmp === false && c.domain !== primary.domain);
  if (!alt) return '';
  const domain = capitalize(alt.domain ?? '');
  const url = alt.url ?? '';
  return ` | ${domain} canonical: **[${url}](${url})**`;
}

function buildCachedNote(nAmp: number, nCached: number, who: string, what: string): string {
  if (nCached === 0) return '';
  let nNote: string;
  if (nAmp === 1 && nCached === 1) nNote = 'the one';
  else if (nAmp === nCached) nNote = 'the ones';
  else nNote = 'some of the ones';
  return ` Fully cached AMP pages (like ${nNote} ${who} ${what}), are [especially problematic](${FAQ_LINK}).`;
}

// ASCII capitalize — matches Python's `str.capitalize()` (uppercase first char,
// lowercase the rest). Domain strings come from the `psl` crate on the Rust
// side, so they're always ASCII; this is fine.
function capitalize(s: string): string {
  if (s.length === 0) return '';
  return s.charAt(0).toUpperCase() + s.slice(1).toLowerCase();
}
