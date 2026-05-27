use std::net::SocketAddr;
use std::path::PathBuf;

use amputatorbot_backend::canonical::{HttpFetcher, PgDatabase};
use amputatorbot_backend::routes;
use amputatorbot_backend::state::AppState;
use amputatorbot_backend::stats::Stats;
use anyhow::Context;
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

    let fetcher = HttpFetcher::new().context("building HTTP fetcher")?;
    let stats = Stats::new(pool.clone());
    let db = PgDatabase::new(pool);
    let state = AppState::new(fetcher, db, stats);

    // Optional. When unset, the binary runs API-only (handy for `cargo run`
    // without a website build). The Dockerfile sets this to /app/static where
    // the Astro `dist/` lands.
    let static_dir = std::env::var_os("STATIC_DIR").map(PathBuf::from);
    let app = routes::router(state, static_dir.as_deref());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "starting amputatorbot-backend");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
