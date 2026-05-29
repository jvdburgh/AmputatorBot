use serde::{Deserialize, Serialize};

/// Where a row in the `links` cache came from.
///
/// Ports `praw-python-archive/models/type.py:Type`, narrowed to the values new code can
/// actually produce. The Postgres `entry_type` enum keeps two extra
/// historical values (`TEST`, `TWEET`) so legacy imports preserve their
/// origin — but Rust never writes them, so they don't appear here.
///
/// `sqlx::Type` with `type_name = "entry_type"` lets sqlx encode/decode this
/// against the matching Postgres ENUM directly. `rename_all` maps Rust's
/// `Api` → SQL's `'API'`. The same `SCREAMING_SNAKE_CASE` form is also used
/// in JSON output via serde, matching the legacy API's casing.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema,
)]
#[sqlx(type_name = "entry_type", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntryType {
    /// Direct API caller — the default when no `X-AmputatorBot-Entry-Type`
    /// header is present.
    Api,
    /// Reddit comment processed by the Devvit app.
    Comment,
    /// Reddit post (submission) processed by the Devvit app.
    Submission,
    /// Modmail / mention / summon, processed by the Devvit app.
    Mention,
    /// Website ConverterForm (M4) — a human pasting a URL into the form.
    Online,
}
