//! REST route handlers for the telemetry ingestion server.

use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub app_version: String,
    pub os: String,
    pub feature: String,
    pub count: u64,
    pub window_ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    pub app_version: String,
    pub os: String,
    pub message: String,
    pub email: Option<String>,
    pub stack: Vec<String>,
    pub crashed_at: i64,
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "reqforge-telemetry",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

pub async fn ingest_usage(
    State(state): State<AppState>,
    Json(events): Json<Vec<UsageEvent>>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    if events.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "no events"})),
        ));
    }
    state.db.insert_usage_events(&events).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn ingest_crash(
    State(state): State<AppState>,
    Json(report): Json<CrashReport>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    state.db.insert_crash_report(&report).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    Ok(StatusCode::ACCEPTED)
}
