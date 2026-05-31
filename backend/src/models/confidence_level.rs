use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema,
)]
#[sqlx(type_name = "confidence_level", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConfidenceLevel {
    Verified,
    Likely,
    Unconfirmed,
}

impl ConfidenceLevel {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.65 {
            Self::Verified
        } else if score >= 0.35 {
            Self::Likely
        } else {
            Self::Unconfirmed
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Likely => "likely",
            Self::Unconfirmed => "unconfirmed",
        }
    }
}
