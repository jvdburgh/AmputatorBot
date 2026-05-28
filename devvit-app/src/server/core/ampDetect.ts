// AMP URL detection.
//
// Mirrors `backend/src/canonical/amp_detect.rs` — same 14 keywords, same denylist,
// same component-scoped scan (host / path / query checked separately). The Rust
// impl is the source of truth; if the two ever drift the resolver wins and this
// file gets updated to match.
//
// Why component-scoped: the legacy Python scanned the whole URL string with 14
// short keywords, which false-positived on `amputeestore.com` etc. (`/amp`
// matched `//amp` after the scheme). Parsing first and matching per-component
// fixes that without losing legitimate matches.

// 14 substring patterns from `archive/static/static.py:8-9`.
// Exported because GUESS_AND_CHECK mutates URLs by removing each keyword in
// turn — that code is in the Rust backend, but keeping the export here makes
// the parallel visible.
export const AMP_KEYWORDS = [
  '/amp',
  'amp/',
  '.amp',
  'amp.',
  '?amp',
  'amp?',
  '=amp',
  'amp=',
  '&amp',
  'amp&',
  '%amp',
  'amp%',
  '_amp',
  'amp_',
] as const;

// Domains hard-excluded from AMP detection regardless of URL shape.
// Ports `archive/static/static.py:10`.
const DENYLISTED_DOMAINS = [
  'video.twimg.kim',
  'bandcamp.com',
  'progonlymusic.com',
  'redd.it',
  'reddit.com',
  'spotify.com',
  'youtube.com',
  'youtu.be',
] as const;

function tryParseUrl(input: string): URL | null {
  try {
    return new URL(input);
  } catch {
    return null;
  }
}

function hasAmpKeyword(s: string): boolean {
  return AMP_KEYWORDS.some((kw) => s.includes(kw));
}

// Returns true if the URL appears to be an AMP URL.
//
// Rules (same as Rust):
//   1. Must parse as a URL.
//   2. Host gets checked against DENYLISTED_DOMAINS first — denied → not AMP.
//   3. Host, path, and query are each scanned for AMP_KEYWORDS. Any one hit → AMP.
export function isAmpUrl(input: string): boolean {
  const parsed = tryParseUrl(input);
  if (!parsed) return false;

  const host = parsed.hostname.toLowerCase();
  if (DENYLISTED_DOMAINS.some((d) => host.endsWith(d))) return false;

  if (hasAmpKeyword(host)) return true;

  const path = parsed.pathname.toLowerCase();
  if (hasAmpKeyword(path)) return true;

  // `search` includes the leading `?`; strip it so `?amp` and `amp?` keywords
  // don't both match the same `?amp=1` query (Rust uses `url.query()` which
  // returns the bare query string for the same reason).
  if (parsed.search.length > 1) {
    const query = parsed.search.slice(1).toLowerCase();
    if (hasAmpKeyword(query)) return true;
  }

  return false;
}

// Returns true if the URL is hosted on a known AMP cache
// (Google AMP, Bing AMP, or ampproject.{net,org}).
// Ports `archive/helpers/checker_utils.py:check_if_cached`.
export function isCachedAmp(input: string): boolean {
  const parsed = tryParseUrl(input);
  if (!parsed) return false;

  const host = parsed.hostname.toLowerCase();
  const path = parsed.pathname.toLowerCase();

  if (host.endsWith('ampproject.net') || host.endsWith('ampproject.org')) {
    return true;
  }

  const onGoogle = host.startsWith('www.google.');
  const onBing = host.startsWith('www.bing.');
  if ((onGoogle || onBing) && path.startsWith('/amp/')) {
    return true;
  }

  return false;
}
