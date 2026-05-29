//! Error + ack response shapes — typed so the OpenAPI spec can document them.
//!
//! v1 and v2 emit slightly different shapes (snake_case vs. camelCase) and
//! the handlers historically produced them via `serde_json::json!(...)` inline.
//! These structs are pure documentation aids — `utoipa` consumes them via
//! `ToSchema`, and they serialize byte-identical to the legacy JSON output so
//! the public contract stays put.
//!
//! No handler actually constructs these (they'd be slower than the inline
//! `json!` macro and offer nothing functional in return). They live here so
//! `#[utoipa::path(..., responses(...))]` annotations can reference them by
//! type rather than hand-writing JSON-schema fragments.

use serde::Serialize;

/// v1 error response. snake_case keys, matches the legacy `/api/v1/convert`
/// shape byte-for-byte.
#[derive(Serialize, utoipa::ToSchema)]
pub struct ErrorResponseV1 {
    /// Short string identifier, e.g. `"error_no_amp"`, `"api_error_required_field_missing"`.
    pub result_code: String,
    /// Human-readable explanation.
    pub error_message: String,
}

/// v2 error response. camelCase keys, matches the v2 convention.
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponseV2 {
    pub result_code: String,
    pub error_message: String,
}

/// Health endpoint payload — `GET /api/v2/health`.
#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    /// Always `true` on a 200. Present so monitoring tools can match on the
    /// body rather than just the status code if they prefer.
    pub ok: bool,
    /// Cargo package version (`CARGO_PKG_VERSION`). Useful for confirming
    /// which build is running in production.
    pub version: String,
}
