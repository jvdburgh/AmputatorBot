//! Domain types for canonical-finding and the public API.
//!
//! Field ordering inside structs follows the legacy Python API's JSON output
//! (alphabetical), so `serde_json` produces a byte-identical response shape.

pub mod canonical;
pub mod canonical_type;
pub mod confidence_level;
pub mod entry_type;
pub mod link;
pub mod url_meta;

pub use canonical::Canonical;
pub use canonical_type::CanonicalType;
pub use confidence_level::ConfidenceLevel;
pub use entry_type::EntryType;
pub use link::Link;
pub use url_meta::UrlMeta;
