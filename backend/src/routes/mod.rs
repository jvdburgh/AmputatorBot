//! HTTP route handlers.
//!
//! Each module is one logical endpoint. [`router`] assembles them into the
//! Axum [`Router`] that `main.rs` serves.

// File-local allow: the OpenAPI assembly below references the deprecated
// Legacy* schemas + the deprecated v1 `legacy_convert::handler`. We want the
// deprecation real in the spec (Scalar uses it for the badge), but the spec-
// assembly call sites here are intentional and shouldn't warn.
#![allow(deprecated)]

use std::path::Path;

use axum::{Json, Router, response::Html, routing::get};
use tower_http::services::ServeDir;
use utoipa::OpenApi;

use crate::models::CanonicalType;
use crate::routes::convert::{Canonical, ConvertBody, ConvertResponse, Link, UrlMeta};
use crate::routes::error::{ErrorResponse, HealthResponse, LegacyErrorResponse};
use crate::routes::legacy_convert::{LegacyCanonical, LegacyLink, LegacyUrlMeta};
use crate::routes::stats::StatsResponse;
use crate::state::AppState;

pub mod convert;
pub mod convert_engine;
pub mod error;
pub mod legacy_convert;
pub mod query_parser;
pub mod stats;

/// OpenAPI spec for the **default** v2 API. Served as JSON at
/// `/api/openapi-v2.json` and loaded by Scalar via the multi-source config in
/// `res/scalar.html`.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "AmputatorBot API",
        version = "5.0.0",
        description = "Strip AMP from URLs. Free, no auth. AMP URLs are recognizable by `/amp/` in the path — e.g. `google.com/amp/...`, `bbc.com/news/amp/...`.",
        contact(name = "AmputatorBot", url = "https://www.amputatorbot.com"),
        license(name = "GPL-3.0-or-later")
    ),
    paths(
        health,
        stats::handler,
        convert::handler,
    ),
    components(schemas(
        Link,
        Canonical,
        UrlMeta,
        CanonicalType,
        StatsResponse,
        ConvertBody,
        ConvertResponse,
        HealthResponse,
        ErrorResponse,
    )),
    tags(
        (name = "convert", description = "Canonical resolution"),
        (name = "system", description = "Health + aggregate stats"),
    )
)]
pub struct ApiDocV2;

/// OpenAPI spec for the **legacy** v1 API. Served as JSON at
/// `/api/openapi-v1.json` and reachable from Scalar via the version dropdown.
/// All schemas + the endpoint carry `deprecated: true` so the v1 surface
/// renders with the Deprecated badge.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "AmputatorBot API (v1 legacy)",
        version = "5.0.0",
        description = "Deprecated. Kept alive for existing third-party integrations. New callers should use POST /api/v2/convert — select `v2` in the version dropdown.",
        contact(name = "AmputatorBot", url = "https://www.amputatorbot.com"),
        license(name = "GPL-3.0-or-later")
    ),
    paths(legacy_convert::handler),
    components(schemas(
        LegacyLink,
        LegacyCanonical,
        LegacyUrlMeta,
        CanonicalType,
        LegacyErrorResponse,
    )),
    tags(
        (name = "convert", description = "Canonical resolution (legacy v1)"),
    )
)]
pub struct ApiDocV1;

const SCALAR_HTML: &str = include_str!("../../res/scalar.html");

async fn scalar_page() -> Html<&'static str> {
    Html(SCALAR_HTML)
}

async fn openapi_v2_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDocV2::openapi())
}

async fn openapi_v1_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDocV1::openapi())
}

/// Build the application router. Takes ownership of [`AppState`] and produces
/// a fully-configured `Router` ready for [`axum::serve`].
///
/// `static_dir`, when `Some` and pointing at an existing directory, is mounted
/// as the router's fallback so non-API paths serve the Astro static bundle
/// (including `/favicon.svg`, which Scalar's `favicon` config points at).
/// When `None` or missing the binary runs API-only — convenient for `cargo
/// run` without a website build, with the trade-off that the docs favicon
/// 404s in that mode.
pub fn router(state: AppState, static_dir: Option<&Path>) -> Router {
    let api = Router::new()
        // Utility endpoints (health, stats) — new in v7, not part of the
        // legacy contract.
        .route("/api/v2/health", get(health))
        .route("/api/v2/stats", get(stats::handler))
        // v1 is GET-only — the legacy contract was always query-string-based.
        // POST callers (and anyone building new integrations) should use v2.
        .route("/api/v1/convert", get(legacy_convert::handler))
        .route("/api/v2/convert", axum::routing::post(convert::handler))
        .with_state(state)
        // Single Scalar page with a version dropdown. The HTML's
        // data-configuration declares two `sources` pointing at the JSON
        // endpoints below; Scalar renders v2 by default and lets the user
        // switch to v1.
        .route("/api/docs", get(scalar_page))
        .route("/api/openapi-v2.json", get(openapi_v2_json))
        .route("/api/openapi-v1.json", get(openapi_v1_json));

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
