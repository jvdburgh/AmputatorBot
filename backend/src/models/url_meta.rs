use serde::{Deserialize, Serialize};

/// Metadata about a URL — the base shape used by `Link.origin` in the
/// response.
///
/// All fields are nullable. `null` appears when the resolver couldn't
/// determine a value (e.g. `domain` is null when the URL couldn't be
/// parsed); `isValid: false` flags a malformed URL.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UrlMeta {
    pub domain: Option<String>,
    pub is_amp: Option<bool>,
    pub is_cached: Option<bool>,
    pub is_valid: Option<bool>,
    pub url: Option<String>,
}

impl UrlMeta {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
            ..Self::default()
        }
    }
}
