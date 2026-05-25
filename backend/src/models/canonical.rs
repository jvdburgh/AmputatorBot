use serde::{Deserialize, Serialize};

use super::CanonicalType;

/// A discovered canonical URL plus metadata about how it was found.
///
/// Ports `archive/models/link.py:Canonical` (which subclasses Python's
/// `UrlMeta`). In Rust we inline the `UrlMeta` fields rather than embedding
/// the struct, so `serde_json` produces a flat JSON object matching the
/// legacy API output.
///
/// Field order matches the Python `jsons.dump` output (alphabetical).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
