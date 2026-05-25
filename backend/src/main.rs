use std::net::SocketAddr;

use anyhow::Context;
use axum::{Json, Router, routing::get};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set (e.g. postgres://amputatorbot:amputatorbot@localhost:5432/amputatorbot)")?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .with_context(|| format!("failed to connect to Postgres at {database_url}"))?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .context("sqlx migrations failed")?;
    tracing::info!("database migrations applied");

    // `pool` is unused until the next task wires it into router state via `Database`.
    let _pool = pool;

    let app = Router::new().route("/api/v1/health", get(health));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "starting amputatorbot-backend");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
