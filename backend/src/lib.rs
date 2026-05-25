//! AmputatorBot Rust backend library.
//!
//! `src/main.rs` is the Axum-based binary; this `lib.rs` exposes the modules
//! that tests, the migration tool, and any future internal binaries depend on.

pub mod canonical;
pub mod models;
pub mod readability;
