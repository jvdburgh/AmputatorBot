use serde::{Deserialize, Serialize};

/// How a canonical URL was discovered.
///
/// Ports `praw-python-archive/models/link.py:CanonicalType`. Variant order matches the
/// Python enum, which is also the priority order canonical-finding tries
/// methods in (`praw-python-archive/helpers/utils.py:get_canonicals` iterates over
/// `CanonicalType` directly).
///
/// JSON serialization uses `SCREAMING_SNAKE_CASE` because Python's
/// `jsons.dump` serializes enum members via `.name` (uppercase identifier),
/// not `.value`. The live API response shows e.g. `"type": "DATABASE"`,
/// `"type": "GOOGLE_MANUAL_REDIRECT"`.
///
/// SQL binding goes through `sqlx::Type` against the Postgres `canonical_type`
/// enum — same `SCREAMING_SNAKE_CASE` literals on both sides.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema,
)]
#[sqlx(type_name = "canonical_type", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CanonicalType {
    Rel,
    Canurl,
    OgUrl,
    GoogleManualRedirect,
    GoogleJsRedirect,
    BingOriginalUrl,
    SchemaMainentity,
    TcoPagetitle,
    MetaRedirect,
    GuessAndCheck,
    Database,
}

impl CanonicalType {
    /// Iterate over every variant in priority order (matches the Python
    /// `for method in CanonicalType:` loop in `get_canonicals`).
    pub const ALL: [CanonicalType; 11] = [
        CanonicalType::Rel,
        CanonicalType::Canurl,
        CanonicalType::OgUrl,
        CanonicalType::GoogleManualRedirect,
        CanonicalType::GoogleJsRedirect,
        CanonicalType::BingOriginalUrl,
        CanonicalType::SchemaMainentity,
        CanonicalType::TcoPagetitle,
        CanonicalType::MetaRedirect,
        CanonicalType::GuessAndCheck,
        CanonicalType::Database,
    ];
}
