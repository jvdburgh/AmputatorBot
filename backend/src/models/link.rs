use serde::{Deserialize, Serialize};

use super::{Canonical, UrlMeta};

/// One resolved URL. The response emits an array of these, one per URL the
/// resolver detected in the request — typically a single entry, but a
/// chat-style body with multiple URLs produces multiple entries.
///
/// - `origin`: metadata about the URL as received from the caller.
/// - `canonicals`: every candidate canonical found, sorted by similarity
///   descending. Often a single entry, sometimes several.
/// - `canonical`: the best non-AMP canonical picked from `canonicals`. Null
///   when no non-AMP canonical could be reached.
/// - `ampCanonical`: when the only canonicals found are themselves AMP and
///   the origin was cached, this surfaces the AMP canonical so callers get
///   *something* better than the cached form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Link {
    pub amp_canonical: Option<Canonical>,
    pub canonical: Option<Canonical>,
    pub canonicals: Vec<Canonical>,
    pub origin: UrlMeta,
}

impl Link {
    pub fn new(origin: UrlMeta) -> Self {
        Self {
            amp_canonical: None,
            canonical: None,
            canonicals: Vec::new(),
            origin,
        }
    }
}
