//! REST route handlers for the plugin marketplace.

use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub q: Option<String>,
    pub tag: Option<String>,
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "reqforge-plugins",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

pub async fn list_plugins(
    State(state): State<AppState>,
    Query(params): Query<ListQuery>,
) -> Json<serde_json::Value> {
    let q = params.q.as_deref().unwrap_or("");
    let tag = params.tag.as_deref();
    let results = state.catalog.search(q, tag);
    Json(serde_json::json!({
        "total": results.len(),
        "plugins": results,
    }))
}

pub async fn get_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    match state.catalog.get(&id) {
        Some(entry) => Ok(Json(serde_json::to_value(entry).unwrap())),
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "plugin not found"})),
        )),
    }
}

pub async fn get_plugin_versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    match state.catalog.get(&id) {
        Some(entry) => Ok(Json(serde_json::json!({
            "id": entry.id,
            "versions": entry.versions,
        }))),
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "plugin not found"})),
        )),
    }
}
