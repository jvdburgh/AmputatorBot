//! HTTP route handlers.
//!
//! Each module is one logical endpoint. [`router`] assembles them into the
//! Axum [`Router`] that `main.rs` serves.

use axum::{Json, Router, routing::get};
use serde_json::json;

use crate::state::AppState;

pub mod convert;
pub mod convert_v2;
pub mod query_parser;

/// Build the application router. Takes ownership of [`AppState`] and produces
/// a fully-configured `Router` ready for [`axum::serve`].
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route(
            "/api/v1/convert",
            get(convert::handler).post(convert::handler),
        )
        .route("/api/v2/convert", axum::routing::post(convert_v2::handler))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
