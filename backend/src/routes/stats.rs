//! `GET /api/v1/stats` — public aggregate counters.
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
    /// Total rows in `links` — counts every resolution the bot has ever
    /// written, including the legacy CSV-imported corpus.
    pub converted_total: i64,
}

#[utoipa::path(
    get,
    path = "/api/v1/stats",
    tag = "system",
    responses(
        (status = 200, description = "Aggregate counters (cached 1h)", body = StatsResponse),
        (status = 500, description = "Stats unavailable", body = crate::routes::error::ErrorResponseV2),
    )
)]
pub async fn handler(
    State(state): State<AppState>,
) -> Result<Json<StatsResponse>, (StatusCode, Json<serde_json::Value>)> {
    match state.stats.converted_total().await {
        Ok(converted_total) => Ok(Json(StatsResponse { converted_total })),
        Err(err) => {
            tracing::error!(?err, "stats count failed");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"errorMessage": "stats unavailable"})),
            ))
        }
    }
}
