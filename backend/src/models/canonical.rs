use serde::{Deserialize, Serialize};

use super::{CanonicalType, ConfidenceLevel};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Canonical {
    pub domain: Option<String>,
    pub is_alt: bool,
    pub is_amp: Option<bool>,
    pub is_cached: Option<bool>,
    pub is_valid: Option<bool>,
    #[serde(rename = "type")]
    pub type_: Option<CanonicalType>,
    pub url: Option<String>,
    pub url_similarity: Option<f64>,
    pub article_similarity: Option<f64>,
    pub confidence_score: Option<f64>,
    pub confidence_level: Option<ConfidenceLevel>,
}

impl Canonical {
    pub fn for_method(method: CanonicalType) -> Self {
        Self {
            domain: None,
            is_alt: false,
            is_amp: None,
            is_cached: None,
            is_valid: None,
            type_: Some(method),
            url: None,
            url_similarity: None,
            article_similarity: None,
            confidence_score: None,
            confidence_level: None,
        }
    }
}
