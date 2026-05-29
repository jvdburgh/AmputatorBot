//! HTTP route handlers.
//!
//! Each module is one logical endpoint. [`router`] assembles them into the
//! Axum [`Router`] that `main.rs` serves.

use std::path::Path;

use axum::{Json, Router, routing::get};
use tower_http::services::ServeDir;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::models::{Canonical, CanonicalType, Link, UrlMeta};
use crate::routes::convert_v2::{ConvertBodyV2, ConvertResponseV2};
use crate::routes::error::{ErrorResponseV1, ErrorResponseV2, HealthResponse};
use crate::routes::stats::StatsResponse;
use crate::state::AppState;

pub mod convert;
pub mod convert_v2;
pub mod error;
pub mod query_parser;
pub mod stats;

/// OpenAPI spec assembled from `#[utoipa::path]` annotations on each handler.
///
/// Consumed two ways:
/// - [`Scalar::with_url`] mounts a UI page at `/api/docs` that loads this spec.
/// - The spec itself is also fetchable at `/api/openapi.json` (the URL Scalar
///   uses to load it), so machines / other tooling can grab it directly.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "AmputatorBot API",
        version = "5.0.0",
        description = "Strip AMP from URLs. Free, no auth. The modern endpoint is POST /api/v2/convert (JSON in / JSON out, camelCase). GET /api/v1/convert stays alive for legacy third-party callers.",
        contact(name = "AmputatorBot", url = "https://www.amputatorbot.com"),
        license(name = "MIT")
    ),
    paths(
        health,
        stats::handler,
        convert::handler,
        convert_v2::handler,
    ),
    components(schemas(
        Link,
        Canonical,
        UrlMeta,
        CanonicalType,
        StatsResponse,
        ConvertBodyV2,
        ConvertResponseV2,
        HealthResponse,
        ErrorResponseV1,
        ErrorResponseV2,
    )),
    tags(
        (name = "convert", description = "Canonical resolution"),
        (name = "system", description = "Health + aggregate stats"),
    )
)]
pub struct ApiDoc;

/// Build the application router. Takes ownership of [`AppState`] and produces
/// a fully-configured `Router` ready for [`axum::serve`].
///
/// `static_dir`, when `Some` and pointing at an existing directory, is mounted
/// as the router's fallback so non-API paths serve the Astro static bundle.
/// When `None` or missing the binary runs API-only — convenient for `cargo
/// run` without a website build.
pub fn router(state: AppState, static_dir: Option<&Path>) -> Router {
    let api = Router::new()
        // Utility endpoints (health, stats) live under v2 — they're new in
        // this rewrite, not part of the legacy contract.
        .route("/api/v2/health", get(health))
        .route("/api/v2/stats", get(stats::handler))
        // v1 is GET-only — the legacy contract was always query-string-based.
        // POST callers (and anyone building new integrations) should use v2.
        .route("/api/v1/convert", get(convert::handler))
        .route("/api/v2/convert", axum::routing::post(convert_v2::handler))
        .with_state(state)
        // Scalar mounts the UI at /api/docs and serves the spec at
        // /api/openapi.json. Both paths are more specific than the API
        // routes above so there's no overlap.
        .merge(Scalar::with_url("/api/docs", ApiDoc::openapi()));

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

#[utoipa::path(
    get,
    path = "/api/v2/health",
    tag = "system",
    responses((status = 200, description = "Service is up", body = HealthResponse))
)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
