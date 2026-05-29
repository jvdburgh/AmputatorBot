// Shape of `POST /api/v2/convert` responses. Mirrors `backend/src/models/`
// after the camelCase transform in `routes/convert_v2.rs::camelize`.
// Re-stated in TS rather than imported so the website can build without
// pointing at the Rust crate's types.

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

export interface UrlMeta {
  domain: string | null;
  isAmp: boolean | null;
  isCached: boolean | null;
  isValid: boolean | null;
  url: string | null;
}

export interface Canonical extends UrlMeta {
  isAlt: boolean;
  type: CanonicalType | null;
  urlSimilarity: number | null;
}

export interface Link {
  origin: UrlMeta;
  canonical: Canonical | null;
  canonicals: Canonical[];
  ampCanonical: Canonical | null;
}

export interface ConvertErrorBody {
  errorMessage: string;
  resultCode: string;
}

export interface ConvertRequestBody {
  query: string;
  guessAndCheck: boolean;
  maxDepth: number;
  redirect: boolean;
  generateMarkdownComment: boolean;
}

// Always returned on 200 OK. `comment` is the Reddit-formatted reply
// markdown the bot would post — populated when the request had
// `generateMarkdownComment: true` and the resolver found at least one
// AMP URL; `null` otherwise.
export interface ConvertResponseV2 {
  links: Link[];
  comment: string | null;
}
