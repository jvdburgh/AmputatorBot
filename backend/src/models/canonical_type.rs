use serde::{Deserialize, Serialize};

/// How a canonical URL was discovered. Variants are listed in the order the
/// resolver tries them — `REL` is the cheapest and most reliable; `DATABASE`
/// is the cache lookup short-circuit. JSON values are SCREAMING_SNAKE_CASE
/// strings (`"REL"`, `"GOOGLE_MANUAL_REDIRECT"`, …).
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
    pub fn confidence_weight(self) -> f64 {
        match self {
            Self::Rel | Self::Canurl | Self::OgUrl | Self::SchemaMainentity | Self::Database => 1.0,
            Self::GoogleManualRedirect
            | Self::GoogleJsRedirect
            | Self::BingOriginalUrl
            | Self::TcoPagetitle
            | Self::MetaRedirect => 0.7,
            Self::GuessAndCheck => 0.3,
        }
    }

    /// Iterate over every variant in priority order — the resolver's
    /// main loop walks this constant in sequence.
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
