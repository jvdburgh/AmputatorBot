use serde::{Deserialize, Serialize};

use super::CanonicalType;

/// A discovered canonical URL plus metadata about how it was found.
///
/// `type` records which of the 11 canonical-finding methods produced this
/// candidate (`REL`, `OG_URL`, `GOOGLE_MANUAL_REDIRECT`, etc.). `isAmp`
/// flips false once the resolver has confirmed the URL is no longer AMP;
/// `urlSimilarity` is the article-text similarity score against the origin
/// when guess-and-check produced the candidate.
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
}

impl Canonical {
    /// Construct a new Canonical for a given method, with all UrlMeta-derived
    /// fields unset (the canonical-finding pipeline fills them in once a
    /// candidate URL is identified).
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
        }
    }
}
