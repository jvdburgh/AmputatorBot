// Metadata for every CanonicalType the Rust backend returns. Drives both
// the in-result "how we found it" card and the homepage methods explainer.
// Source of truth for the enum: backend/src/models/canonical_type.rs and
// the Postgres `canonical_type` enum in migrations/001_initial.sql.

import type { SnippetLang } from '@/lib/highlight';

export interface MethodInfo {
  label: string;
  summary: string;
  explanation: string;
  snippet?: string;
  snippetLanguage?: SnippetLang;
}

export const CANONICAL_METHODS: Record<string, MethodInfo> = {
  REL: {
    label: 'HTML canonical tag',
    summary: 'The page declared its own canonical URL.',
    explanation:
      'The AMP page\'s <head> contains a <link rel="canonical"> tag. The publisher is telling browsers and search engines directly which URL is the real one, so we use it as-is.',
    snippet: '<link rel="canonical" href="https://example.com/article" />',
    snippetLanguage: 'html',
  },
  CANURL: {
    label: 'canurl URL parameter',
    summary: 'The AMP URL carries a canurl pointing at the real page.',
    explanation:
      'Some CMSes append a "canurl" query parameter to the AMP URL pointing at the canonical. When it\'s there, the publisher has already given us the answer inside the link itself.',
    snippet: 'https://amp.example.com/article?canurl=https://example.com/article',
    snippetLanguage: 'url',
  },
  OG_URL: {
    label: 'OpenGraph URL',
    summary: "Pulled from the page's social-sharing metadata.",
    explanation:
      'Most pages also declare their canonical URL through OpenGraph — the <meta property="og:url"> tag is what drives social-media preview cards. Slightly less authoritative than rel="canonical", but it almost always agrees.',
    snippet: '<meta property="og:url" content="https://example.com/article" />',
    snippetLanguage: 'html',
  },
  GOOGLE_MANUAL_REDIRECT: {
    label: 'Google AMP cache URL',
    summary: 'Decoded straight from the Google AMP cache URL.',
    explanation:
      "Google's AMP cache URLs (www.google.com/amp/s/...) embed the publisher's domain and path right in the URL. We decode the original URL straight from the URL string — no page fetch involved.",
    snippet: 'https://www.google.com/amp/s/example.com/article/amp/',
    snippetLanguage: 'url',
  },
  GOOGLE_JS_REDIRECT: {
    label: 'Google AMP page JS redirect',
    summary: "Extracted from the cached AMP page's redirect handler.",
    explanation:
      "When the URL itself doesn't decode cleanly, we fetch the cached page. The AMP page exposes its source URL inside the JavaScript that handles the redirect back to the publisher — we pull it out with a regex.",
    snippet: 'const redirectUrl = "https://example.com/article";',
    snippetLanguage: 'js',
  },
  BING_ORIGINAL_URL: {
    label: 'Bing AMP cache',
    summary: 'Extracted from a Bing AMP cache marker.',
    explanation:
      'Bing runs its own AMP cache and embeds the publisher\'s canonical URL in an inline JSON object on the cached page. We regex for the `"originalUrl"` key.',
    snippet: '{"originalUrl": "https://example.com/article", ...}',
    snippetLanguage: 'json',
  },
  SCHEMA_MAINENTITY: {
    label: 'Schema.org metadata',
    summary: 'From a JSON-LD <script> block on the page.',
    explanation:
      'The page embeds schema.org metadata inside a <script type="application/ld+json"> block, which identifies the main article and its canonical URL. A useful fallback when neither rel="canonical" nor OpenGraph turn up.',
    snippet:
      '{"@context": "https://schema.org", "@type": "Article", "mainEntity": "https://example.com/article"}',
    snippetLanguage: 'json',
  },
  TCO_PAGETITLE: {
    label: 't.co title heuristic',
    summary: "Read off Twitter's t.co interstitial page.",
    explanation:
      "Twitter's t.co shortener doesn't return the destination URL in a redirect header — it serves an interstitial page that puts the destination URL in the <title>. We parse it out of there.",
    snippet: '<title>https://example.com/article — t.co</title>',
    snippetLanguage: 'html',
  },
  META_REDIRECT: {
    label: 'Meta refresh redirect',
    summary: 'A <meta http-equiv="refresh"> on the page points at the real URL.',
    explanation:
      'Some pages skip JavaScript and use the old-school <meta http-equiv="refresh"> tag to redirect to the canonical. We read the tag and follow it.',
    snippet: '<meta http-equiv="refresh" content="0; url=https://example.com/article" />',
    snippetLanguage: 'html',
  },
  GUESS_AND_CHECK: {
    label: 'Heuristic URL transformation',
    summary:
      'Strip AMP markers from the URL; the orchestrator separately verifies via article-text comparison.',
    explanation:
      'When no explicit canonical signal turns up, we transform the URL itself: strip `/amp/` path segments, decode `google.com/amp/s/` and `cdn.ampproject.org` cache wrappers, drop `amp.` subdomains. The "check" happens separately — every candidate URL is fetched and its article text compared to the AMP page\'s. VERIFIED means the article match succeeded; UNCONFIRMED means we couldn\'t verify (e.g. the publisher blocked our fetch).',
    snippet:
      'https://news.sky.com/story/amp/article  →  https://news.sky.com/story/article\n(orchestrator then fetches + compares article text to determine confidence)',
    snippetLanguage: 'url',
  },
  DATABASE: {
    label: 'Cached from a previous run',
    summary: 'AmputatorBot resolved this AMP URL within the past year.',
    explanation:
      'This exact AMP URL has come through the bot in the past year. Rather than refetching the page, we return the canonical we resolved most recently. Entries older than a year are ignored (publishers move slugs and restructure paths) and fall through to a fresh resolution.',
  },
};

export function describeMethod(type: string | null | undefined): MethodInfo {
  if (!type) return { label: 'Unknown', summary: '', explanation: 'No method recorded.' };
  return (
    CANONICAL_METHODS[type] ?? {
      label: type,
      summary: '',
      explanation: `No description for ${type}.`,
    }
  );
}

// The order matches the resolver's priority in backend/src/canonical/methods/.
// REL is tried first; DATABASE is consulted as a fast-path cache outside the
// regular method chain.
export const METHODS_IN_ORDER: string[] = [
  'REL',
  'CANURL',
  'OG_URL',
  'GOOGLE_MANUAL_REDIRECT',
  'GOOGLE_JS_REDIRECT',
  'BING_ORIGINAL_URL',
  'SCHEMA_MAINENTITY',
  'TCO_PAGETITLE',
  'META_REDIRECT',
  'GUESS_AND_CHECK',
  'DATABASE',
];
