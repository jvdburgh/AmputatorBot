// TypeScript shapes for the v2 API the Devvit app calls. Mirrors the Rust
// types in `backend/src/models/` — if the backend response shape ever
// changes, these must change in lockstep, and the handler test mocks will
// catch the drift loudly.
//
// v2 response keys are camelCase recursively (the Rust serde types are
// snake_case; the backend transforms keys at the edge before returning).

export type CanonicalType =
  | 'REL'
  | 'CANURL'
  | 'OG_URL'
  | 'GOOGLE_MANUAL_REDIRECT'
  | 'GOOGLE_JS_REDIRECT'
  | 'BING_ORIGINAL_URL'
  | 'SCHEMA_MAINENTITY'
  | 'TCO_PAGETITLE'
  | 'META_REDIRECT'
  | 'GUESS_AND_CHECK'
  | 'DATABASE';

// What the bot sets on the `X-AmputatorBot-Entry-Type` request header so
// the backend can tag the resulting `links` row with the call's origin.
// `API` is the backend default when the header is absent; the bot itself
// only ever sends `COMMENT` / `SUBMISSION` from triggers.
export type EntryType = 'API' | 'COMMENT' | 'SUBMISSION' | 'MENTION' | 'ONLINE';

export type UrlMeta = {
  domain: string | null;
  isAmp: boolean | null;
  isCached: boolean | null;
  isValid: boolean | null;
  url: string | null;
};

export type ConfidenceLevel = 'VERIFIED' | 'LIKELY' | 'UNCONFIRMED';

export type Canonical = {
  domain: string | null;
  isAlt: boolean;
  isAmp: boolean | null;
  isCached: boolean | null;
  isValid: boolean | null;
  type: CanonicalType | null;
  url: string | null;
  urlSimilarity: number | null;
  articleSimilarity: number | null;
  confidenceScore: number | null;
  confidenceLevel: ConfidenceLevel | null;
};

export type Link = {
  ampCanonical: Canonical | null;
  canonical: Canonical | null;
  canonicals: Canonical[];
  origin: UrlMeta;
};

// Always returned on 200 OK from `POST /api/v2/convert`. `comment` is
// populated when the request had `generateMarkdownComment: true` and the
// resolver found at least one AMP URL; `null` otherwise.
export type ConvertResponseV2 = {
  links: Link[];
  comment: string | null;
};
