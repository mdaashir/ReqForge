//! Self-hosted sync server for ReqForge.
//!
//! Exposes a Y-WebSocket-compatible endpoint at `ws://host/v1/ws/{doc_id}`
//! and a small REST surface for doc management. SQLite-backed for
//! durability across restarts.

mod auth;
mod config;
mod db;
mod error;
mod http;
mod ws;

use crate::config::Config;
use crate::error::ServerResult;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Arc<db::Db>,
}

#[tokio::main]
async fn main() -> ServerResult<()> {
    init_tracing();

    let config = Arc::new(Config::from_env()?);
    let db = Arc::new(db::Db::open(&config.database_url).await?);
    db.migrate_async().await?;

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
    };

    let app = Router::new()
        .route("/health", get(http::health))
        .route("/v1/docs", get(http::list_docs).post(http::create_doc))
        .route(
            "/v1/docs/:doc_id",
            get(http::get_doc_meta).delete(http::delete_doc),
        )
        .route("/v1/ws/:doc_id", get(ws::ws_handler))
        .route("/v1/auth/token", get(http::issue_token_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = config.bind_addr.parse()?;
    tracing::info!(%addr, "reqforge-sync listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::{prelude::*, EnvFilter};
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}
