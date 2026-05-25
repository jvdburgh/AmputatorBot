//! Canonical-finding engine.
//!
//! This module ports `archive/helpers/canonical_methods.py` and
//! `archive/helpers/utils.py:get_canonicals` from the legacy Python bot.
//!
//! The 11 canonical methods (10 + DATABASE cache lookup) are tried in
//! priority order — see [`crate::models::CanonicalType::ALL`].

pub mod amp_detect;

pub use amp_detect::{is_amp_url, is_cached_amp};
