//! HTTP route handlers.
//!
//! Each module is one logical endpoint. [`router`] assembles them into the
//! Axum [`Router`] that `main.rs` serves.

use std::path::Path;

use axum::{Json, Router, routing::get};
use serde_json::json;
use tower_http::services::ServeDir;

use crate::state::AppState;

pub mod convert;
pub mod convert_v2;
pub mod query_parser;
pub mod stats;

/// Build the application router. Takes ownership of [`AppState`] and produces
/// a fully-configured `Router` ready for [`axum::serve`].
///
/// `static_dir`, when `Some` and pointing at an existing directory, is mounted
/// as the router's fallback so non-API paths serve the Astro static bundle.
/// When `None` or missing the binary runs API-only — convenient for `cargo
/// run` without a website build.
pub fn router(state: AppState, static_dir: Option<&Path>) -> Router {
    let api = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/stats", get(stats::handler))
        .route(
            "/api/v1/convert",
            get(convert::handler).post(convert::handler),
        )
        .route("/api/v2/convert", axum::routing::post(convert_v2::handler))
        .with_state(state);

    match static_dir {
        Some(dir) if dir.is_dir() => {
            tracing::info!(path = %dir.display(), "serving static files from fallback");
            api.fallback_service(ServeDir::new(dir))
        }
        Some(dir) => {
            tracing::warn!(
                path = %dir.display(),
                "STATIC_DIR set but missing; serving API only"
            );
            api
        }
        None => api,
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
