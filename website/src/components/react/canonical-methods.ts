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
      'Inside the AMP page\'s <head> we find a <link rel="canonical"> tag. This is the strongest signal — the publisher is telling browsers and search engines what they consider the real URL.',
    snippet: '<link rel="canonical" href="https://example.com/article" />',
    snippetLanguage: 'html',
  },
  CANURL: {
    label: 'canurl URL parameter',
    summary: 'The AMP URL carries a canurl pointing at the real page.',
    explanation:
      'Some CMS-generated AMP variants append a "canurl" query parameter to the AMP URL itself. If we find one, we use its value directly — the publisher already encoded the answer in the link.',
    snippet: 'https://amp.example.com/article?canurl=https://example.com/article',
    snippetLanguage: 'url',
  },
  OG_URL: {
    label: 'OpenGraph URL',
    summary: "Pulled from the page's social-sharing metadata.",
    explanation:
      'Most pages declare their canonical URL through OpenGraph too — the <meta property="og:url"> tag drives every social-media preview card. Slightly less authoritative than rel="canonical", but reliable in practice.',
    snippet: '<meta property="og:url" content="https://example.com/article" />',
    snippetLanguage: 'html',
  },
  GOOGLE_MANUAL_REDIRECT: {
    label: 'Google AMP cache URL',
    summary: 'Decoded straight from the Google AMP cache URL.',
    explanation:
      "Google's AMP cache (www.google.com/amp/s/...) embeds the publisher's domain and path right in the URL. We can decode the original URL without ever fetching the page — fast and zero-cost.",
    snippet: 'https://www.google.com/amp/s/example.com/article/amp/',
    snippetLanguage: 'url',
  },
  GOOGLE_JS_REDIRECT: {
    label: 'Google AMP page JS redirect',
    summary: "Extracted from the cached AMP page's redirect handler.",
    explanation:
      "When URL-decoding the Google AMP cache URL doesn't yield a usable canonical, we fall back to fetching the cached page itself. The AMP page exposes its source URL inside the JavaScript that drives the back-to-publisher redirect — we regex it out.",
    snippet: 'const redirectUrl = "https://example.com/article";',
    snippetLanguage: 'js',
  },
  BING_ORIGINAL_URL: {
    label: 'Bing AMP cache',
    summary: 'Extracted from a Bing AMP cache marker.',
    explanation:
      'Bing operates its own AMP cache and embeds the publisher\'s canonical URL in an inline JSON object on the cached page. We regex for the `"originalUrl"` key.',
    snippet: '{"originalUrl": "https://example.com/article", ...}',
    snippetLanguage: 'json',
  },
  SCHEMA_MAINENTITY: {
    label: 'Schema.org metadata',
    summary: 'From a JSON-LD <script> block on the page.',
    explanation:
      'The page\'s schema.org metadata (in a <script type="application/ld+json"> block) identifies the main article and its canonical URL. Useful when neither rel="canonical" nor OpenGraph is present.',
    snippet:
      '{"@context": "https://schema.org", "@type": "Article", "mainEntity": "https://example.com/article"}',
    snippetLanguage: 'json',
  },
  TCO_PAGETITLE: {
    label: 't.co title heuristic',
    summary: "Read off Twitter's t.co interstitial page.",
    explanation:
      "Twitter's t.co shortener doesn't return the destination URL directly — it serves an interstitial page where the destination URL is in the <title>. We parse that.",
    snippet: '<title>https://example.com/article — t.co</title>',
    snippetLanguage: 'html',
  },
  META_REDIRECT: {
    label: 'Meta refresh redirect',
    summary: 'A <meta http-equiv="refresh"> on the page points at the real URL.',
    explanation:
      'Some pages skip JS and use the old-school <meta http-equiv="refresh"> tag to redirect to the canonical. We read that tag and follow it.',
    snippet: '<meta http-equiv="refresh" content="0; url=https://example.com/article" />',
    snippetLanguage: 'html',
  },
  GUESS_AND_CHECK: {
    label: 'Pattern guess + similarity check',
    summary: 'Stripped AMP markers from the URL, then verified the result is the same article.',
    explanation:
      'Last-resort fallback when no explicit canonical signal was present. We strip AMP-specific URL patterns (/amp, ?amp=1, the amp. subdomain, etc.) to guess what the canonical URL might be, then fetch the resulting page and compare its article text to the AMP page\'s article text using a Mozilla-Readability-port extractor. We only accept the guess if similarity is high enough (above 60% for "valid", above 35% for "low confidence").',
    snippet:
      "https://amp.example.com/article/amp/  →  https://example.com/article\n(then verify: extracted article text matches the AMP page's)",
    snippetLanguage: 'url',
  },
  DATABASE: {
    label: 'Cached from a previous run',
    summary: 'AmputatorBot has resolved this AMP URL before.',
    explanation:
      "We've seen this exact AMP URL come through the bot before. Rather than refetching the page, we return the canonical we resolved most recently.",
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
