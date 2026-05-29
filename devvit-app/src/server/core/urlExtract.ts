// Extract URLs from comment/post bodies + strip Reddit markdown artifacts.
//
// Mirrors `backend/src/canonical/url_extract.rs`. The Rust port uses the
// `linkify` crate; here we use a regex sufficient for the Reddit-comment
// shapes the bot actually sees (anything more elaborate gets bounced through
// the backend's resolver anyway — this is just to decide which URLs from the
// comment to send for canonical-finding).
//
// If parity testing surfaces a real divergence with the Rust impl, swap this
// for the `linkify-it` npm package which closely tracks the Rust behavior.

// Trailing characters stripped from extracted URLs. Same set as the Rust
// `TRAILING_MARKDOWN_CHARS` slice; U+201D is the typographic right-double-
// quote that Reddit sometimes renders for smart-quoted pastes.
const TRAILING_MARKDOWN_CHARS = new Set([
  '?',
  '(',
  ')',
  '[',
  ']',
  '\\',
  ',',
  '.',
  '"',
  '”',
  '`',
  '^',
  '*',
  '|',
  '>',
  '<',
  '{',
  '}',
  '~',
  ':',
  ';',
]);

// Match http(s) URLs up to the next whitespace or angle-bracket. Trailing
// punctuation that we want to strip (`,`, `)`, `.`, etc.) lands inside the
// match and is removed by `removeMarkdown` below — matching what the Rust
// `linkify::LinkFinder` + `remove_markdown` pipeline produces.
const URL_RE = /https?:\/\/[^\s<>"]+/gi;

// Strip trailing markdown punctuation from a URL. Ports
// `praw-python-archive/helpers/utils.py:remove_markdown`.
export function removeMarkdown(url: string): string {
  let end = url.length;
  while (end > 0) {
    const ch = url.charAt(end - 1);
    if (!TRAILING_MARKDOWN_CHARS.has(ch)) break;
    end -= 1;
  }
  return url.slice(0, end);
}

// Extract all unique URLs from a chunk of text, in source order. Duplicates
// are dropped (first occurrence wins) — mirrors `URLExtract(only_unique=True)`
// in the legacy Python and the `HashSet`-dedupe pass in the Rust port.
export function extractUrls(body: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const match of body.matchAll(URL_RE)) {
    const cleaned = removeMarkdown(match[0]);
    if (cleaned.length === 0) continue;
    if (seen.has(cleaned)) continue;
    seen.add(cleaned);
    out.push(cleaned);
  }
  return out;
}
