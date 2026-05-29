//! `GET /api/v2/stats` — public aggregate counters.
//!
//! Currently a single value (`convertedTotal`) the homepage's tech section
//! reads to display "X AMP links converted since 2019". Backed by a cached
//! `SELECT COUNT(*) FROM links` in [`crate::stats::Stats`] with a 1h TTL.
//!
//! Camelcase response key matches the v2 convention; v1 callers see the
//! same shape since stats is a new endpoint without a legacy precedent.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use serde_json::json;

use crate::state::AppState;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    /// Total rows in `links` — every resolution (successful and failed)
    /// the bot has recorded since 2019.
    pub converted_total: i64,
}

#[utoipa::path(
    get,
    path = "/api/v2/stats",
    tag = "system",
    responses(
        (status = 200, description = "Aggregate counters (cached 1h)", body = StatsResponse),
        (status = 500, description = "Stats unavailable", body = crate::routes::error::ErrorResponse),
    )
)]
pub async fn handler(
    State(state): State<AppState>,
) -> Result<Json<StatsResponse>, (StatusCode, Json<serde_json::Value>)> {
    match state.stats.converted_total().await {
        Ok(converted_total) => Ok(Json(StatsResponse { converted_total })),
        Err(err) => {
            tracing::error!(?err, "stats count failed");
            // Keys mirror the v2 `ErrorResponse` schema this 500 documents.
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "errorMessage": "stats unavailable",
                    "resultCode": "error_stats_unavailable",
                })),
            ))
        }
    }
}
