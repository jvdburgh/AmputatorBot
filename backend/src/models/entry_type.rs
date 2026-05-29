use serde::{Deserialize, Serialize};

// Where a row in the `links` cache came from. Used internally to tag DB
// rows by source; the value never lands in API responses. Set via the
// `X-AmputatorBot-Entry-Type` request header — direct API callers can't
// set it (header defaults to `API` when missing), only the Devvit app and
// the website do, so external traffic can't pollute the per-source
// analytics.
//
// The Postgres `entry_type` enum keeps two extra historical values
// (`TEST`, `TWEET`) so CSV imports of the legacy bot's rows preserve
// their origin; new code never writes them so they don't appear here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "entry_type", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntryType {
    // Direct API caller — the default when no
    // `X-AmputatorBot-Entry-Type` header is present.
    Api,
    // Reddit comment processed by the Devvit app.
    Comment,
    // Reddit post (submission) processed by the Devvit app.
    Submission,
    // Modmail / mention / summon, processed by the Devvit app.
    Mention,
    // Website ConverterForm — a human pasting a URL into the form.
    Online,
}
