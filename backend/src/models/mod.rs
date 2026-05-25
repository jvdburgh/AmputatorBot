//! Domain types for canonical-finding and the public API.
//!
//! Field ordering inside structs follows the legacy Python API's JSON output
//! (alphabetical), so `serde_json` produces a byte-identical response shape.

pub mod canonical;
pub mod link;
pub mod url_meta;

pub use canonical::{Canonical, CanonicalType};
pub use link::Link;
pub use url_meta::UrlMeta;
