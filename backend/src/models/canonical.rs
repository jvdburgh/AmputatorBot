use serde::{Deserialize, Serialize};

/// How a canonical URL was discovered.
///
/// Ports `archive/models/link.py:CanonicalType`. Variant order matches the
/// Python enum, which is also the priority order canonical-finding tries
/// methods in (`archive/helpers/utils.py:get_canonicals` iterates over
/// `CanonicalType` directly).
///
/// JSON serialization uses `SCREAMING_SNAKE_CASE` because Python's
/// `jsons.dump` serializes enum members via `.name` (uppercase identifier),
/// not `.value`. The live API response shows e.g. `"type": "DATABASE"`,
/// `"type": "GOOGLE_MANUAL_REDIRECT"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
