use serde::{Deserialize, Serialize};

/// Metadata about a URL — the base "shape" shared by `origin` and each
/// `Canonical` in the API response.
///
/// Ports `praw-python-archive/models/urlmeta.py:UrlMeta`. All fields are `Option` because
/// the legacy Python class defaulted them to `None` and the public JSON
/// response includes `null` for unset fields.
///
/// Field order matches the Python `jsons.dump` output (alphabetical) so the
/// serialized JSON is byte-identical to the legacy `/api/v1/convert` shape.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
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
