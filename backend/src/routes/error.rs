//! Error + ack response shapes — typed so the OpenAPI spec can document them.
//!
//! The v1 endpoint emits snake_case keys; the v2 endpoint emits camelCase.
//! Both ship inline via `serde_json::json!(...)` in their handlers — the
//! structs here exist purely so `#[utoipa::path(..., responses(...))]`
//! annotations can reference them by type rather than hand-writing the
//! JSON-schema fragments.

use serde::Serialize;

/// Deprecated v1 error response. snake_case keys, matches the legacy
/// `/api/v1/convert` shape byte-for-byte.
#[allow(dead_code)]
#[deprecated = "Use ErrorResponse (v2 camelCase)"]
#[derive(Serialize, utoipa::ToSchema)]
#[schema(deprecated)]
pub struct LegacyErrorResponse {
    /// Short string identifier, e.g. `"error_no_amp"`, `"api_error_required_field_missing"`.
    pub result_code: String,
    /// Human-readable explanation.
    pub error_message: String,
}

/// v2 error response. camelCase keys.
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
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
