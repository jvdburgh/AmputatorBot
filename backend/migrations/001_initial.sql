-- Initial schema for AmputatorBot's canonical-URL cache.
--
-- Mirrors the legacy `URLConversions` table from the PythonAnywhere MySQL
-- database (columns + values), but with snake_case naming, real ENUMs, and
-- the indexes the legacy schema was missing.
--
-- Values for `canonical_type` match the SCREAMING_SNAKE_CASE serialization
-- of the Rust `CanonicalType` enum, byte-for-byte (see
-- `src/models/canonical_type.rs`).
--
-- See M3 §"Schema (locked)" in docs/amputatorbot-devvit-migration-plan-v7.md.

CREATE TYPE canonical_type AS ENUM (
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
    'DATABASE'
);

-- TEST and TWEET are present for legacy rows only; new code never inserts these.
CREATE TYPE entry_type AS ENUM (
    'ONLINE',
    'COMMENT',
    'SUBMISSION',
    'MENTION',
    'TEST',
    'TWEET',
    'API'
);

-- URL-length cap: 2048 chars. Matches the Sitemaps protocol cap (the only
-- widely-adopted formal standard for URL length), keeps indexed values
-- under Postgres' btree max (~2704 bytes), and rejects pathological URLs
-- that are almost always junk (tracking blobs, malformed forwards).
-- Mirrored in Rust as `canonical::MAX_URL_LEN`. Legacy rows longer than
-- this are filtered during `just db-seed`.
CREATE TABLE links (
    entry_id       BIGSERIAL     PRIMARY KEY,
    entry_type     entry_type,
    handled_utc    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    original_url   TEXT          NOT NULL CHECK (length(original_url) <= 2048),
    canonical_url  TEXT                   CHECK (canonical_url IS NULL OR length(canonical_url) <= 2048),
    canonical_type canonical_type,
    note           TEXT
);

-- Composite supports the DATABASE-method lookup
-- (`WHERE original_url = $1 ORDER BY handled_utc DESC LIMIT 1`) as an
-- index-only scan. Btree prefix matching means it also covers plain
-- `WHERE original_url = $1` filters, so a separate single-column index
-- on `original_url` would be redundant.
CREATE INDEX idx_links_original_url_handled ON links (original_url, handled_utc DESC);
CREATE INDEX idx_links_canonical_url        ON links (canonical_url);
CREATE INDEX idx_links_handled_utc          ON links (handled_utc);
