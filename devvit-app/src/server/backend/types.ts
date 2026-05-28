// TypeScript shapes mirroring the Rust types in `backend/src/models/`.
//
// Only what the Devvit app actually consumes — the full struct is in
// `backend/src/models/link.rs` (and `canonical.rs`, `canonical_type.rs`,
// `url_meta.rs`, `entry_type.rs`). If the backend response shape ever
// changes, these types must change in lockstep and the snapshot tests in
// `reply.test.ts` will catch the drift loudly.
//
// v2 response keys are camelCase (the Rust serde types are snake_case;
// v2 transforms keys at the edge in `backend/src/routes/convert_v2.rs`).

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

// `EntryType` enum — what the v2 body accepts for `entryType`.
// TEST + TWEET exist in the backend enum for legacy CSV rows; the bot never
// emits those.
export type EntryType = 'API' | 'COMMENT' | 'SUBMISSION' | 'MENTION' | 'ONLINE';

export type UrlMeta = {
  domain: string | null;
  isAmp: boolean | null;
  isCached: boolean | null;
  isValid: boolean | null;
  url: string | null;
};

export type Canonical = {
  domain: string | null;
  isAlt: boolean;
  isAmp: boolean | null;
  isCached: boolean | null;
  isValid: boolean | null;
  type: CanonicalType | null;
  url: string | null;
  urlSimilarity: number | null;
};

export type Link = {
  ampCanonical: Canonical | null;
  canonical: Canonical | null;
  canonicals: Canonical[];
  origin: UrlMeta;
};
