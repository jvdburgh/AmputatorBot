//! Canonical-finding engine.
//!
//! This module ports `archive/helpers/canonical_methods.py` and
//! `archive/helpers/utils.py:get_canonicals` from the legacy Python bot.
//!
//! The 11 canonical methods (10 + DATABASE cache lookup) are tried in
//! priority order — see [`crate::models::CanonicalType::ALL`].

pub mod amp_detect;
pub mod http;
pub mod methods;
pub mod url_extract;

pub use amp_detect::{is_amp_url, is_cached_amp};
pub use http::{HttpFetcher, Page};
pub use methods::{CanonicalFlags, MethodContext, try_method};
pub use url_extract::{extract_urls, remove_markdown};
