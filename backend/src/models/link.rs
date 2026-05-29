use serde::{Deserialize, Serialize};

use super::{Canonical, UrlMeta};

/// The full result of resolving a single URL through canonical-finding.
///
/// Ports `praw-python-archive/models/link.py:Link`. One `Link` is emitted per URL the
/// API saw in the input (the live `/api/v1/convert` returns an array of
/// these).
///
/// - `origin`: metadata about the URL as received from the caller.
/// - `canonicals`: every candidate canonical found, in priority order then
///   sorted by `url_similarity` descending.
/// - `canonical`: the best non-AMP canonical, picked from `canonicals`.
/// - `amp_canonical`: when the only canonicals found are themselves AMP
///   (e.g. a cached Google AMP URL with no upstream non-AMP variant
///   reachable), and the origin was cached, this surfaces the AMP canonical
///   so callers still get *something* better than the cached form.
///
/// Field order matches the Python `jsons.dump` output (alphabetical).
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
