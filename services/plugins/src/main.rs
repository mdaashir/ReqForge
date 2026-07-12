//! Self-hosted plugin marketplace for ReqForge.
//!
//! Provides a REST API for browsing, searching, and discovering community
//! plugins. The catalog is file-backed (JSON), seeded from a companion
//! `plugins/registry.json` file that can be updated via git or CI.
//!
//! Endpoints:
//! - GET /v1/plugins — list all, with optional ?q= and ?tag= filters
//! - GET /v1/plugins/:id — detail (includes installUrl)
//! - GET /v1/plugins/:id/versions — version history

mod catalog;
mod routes;

use crate::catalog::Catalog;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub catalog: Arc<Catalog>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let catalog_path =
        std::env::var("REQFORGE_PLUGIN_REGISTRY").unwrap_or_else(|_| "registry.json".into());
    let catalog = Arc::new(Catalog::load(&catalog_path)?);

    let app = Router::new()
        .route("/health", axum::routing::get(routes::health))
        .route(
            "/v1/plugins",
            axum::routing::get(routes::list_plugins),
        )
        .route(
            "/v1/plugins/:id",
            axum::routing::get(routes::get_plugin),
        )
        .route(
            "/v1/plugins/:id/versions",
            axum::routing::get(routes::get_plugin_versions),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(AppState {
            catalog: catalog.clone(),
        });

    let bind = std::env::var("REQFORGE_BIND").unwrap_or_else(|_| "0.0.0.0:7444".into());
    let addr: SocketAddr = bind.parse()?;
    tracing::info!(%addr, "plugin marketplace listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
