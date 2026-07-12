//! Telemetry ingestion server for ReqForge.
//!
//! Accepts anonymous usage counters (POST /v1/usage) and crash reports
//! (POST /v1/crash). Stores everything in SQLite for dashboard/reporting.
//! No personal information is stored unless the user explicitly includes
//! an email address in a crash report.

mod db;
mod routes;

use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<db::Db>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let db_path = std::env::var("REQFORGE_TELEMETRY_DB")
        .unwrap_or_else(|_| "reqforge-telemetry.db".into());
    let db = Arc::new(db::Db::open(&db_path).await?);

    let app = Router::new()
        .route("/health", axum::routing::get(routes::health))
        .route("/v1/usage", axum::routing::post(routes::ingest_usage))
        .route("/v1/crash", axum::routing::post(routes::ingest_crash))
        .layer(TraceLayer::new_for_http())
        .with_state(AppState { db });

    let bind = std::env::var("REQFORGE_BIND").unwrap_or_else(|_| "0.0.0.0:7445".into());
    let addr: SocketAddr = bind.parse()?;
    tracing::info!(%addr, "telemetry server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
