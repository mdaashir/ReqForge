//! REST surface for doc management.

use crate::auth::{issue_token, AuthUser};
use crate::db::{Db, DocMeta};
use crate::error::{ServerError, ServerResult};
use crate::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

/// Liveness check. Does not require auth.
pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    /// Long-lived refresh token the desktop exchanges for a short-lived
    /// access token. In production this would talk to the ReqForge
    /// account service; here we accept any non-empty string so the
    /// server can be self-hosted without an account backend.
    pub refresh_token: String,
    /// Optional user identifier to bind to the issued JWT.
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
}

const TOKEN_TTL_SECS: i64 = 3600;

pub async fn issue_token_handler(
    State(state): State<AppState>,
    Json(req): Json<TokenRequest>,
) -> ServerResult<impl IntoResponse> {
    if req.refresh_token.trim().is_empty() {
        return Err(ServerError::BadRequest("refresh_token is required".into()));
    }
    let user_id = req.user_id.unwrap_or_else(|| {
        // Hash the refresh token to derive a stable user id when none
        // was supplied. Not cryptographic — just an identifier.
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        req.refresh_token.hash(&mut h);
        format!("user-{:x}", h.finish())
    });
    state.db.upsert_user_async(&user_id, &user_id).await?;

    let token = issue_token(
        &state.config.jwt_secret,
        &user_id,
        Some(&user_id),
        TOKEN_TTL_SECS,
    )?;
    Ok((
        StatusCode::OK,
        Json(TokenResponse {
            access_token: token,
            token_type: "Bearer",
            expires_in: TOKEN_TTL_SECS,
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateDocRequest {
    pub doc_id: Option<String>,
    /// Optional initial state (Yrs-encoded update). If absent, an empty
    /// doc is created.
    pub initial_state: Option<Vec<u8>>,
}

pub async fn create_doc(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateDocRequest>,
) -> ServerResult<impl IntoResponse> {
    let doc_id = req
        .doc_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let state_bytes = req.initial_state.unwrap_or_default();

    if state_bytes.len() as u64 > state.config.max_doc_size_bytes {
        return Err(ServerError::PayloadTooLarge);
    }

    state
        .db
        .upsert_doc_state_async(&doc_id, &claims.sub, &state_bytes)
        .await?;

    let meta = state
        .db
        .get_doc_meta_async(&doc_id)
        .await?
        .ok_or(ServerError::NotFound)?;
    Ok((StatusCode::CREATED, Json(meta)))
}

pub async fn list_docs(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> ServerResult<Json<Vec<DocMeta>>> {
    let docs: Vec<DocMeta> = state.db.list_docs_for_owner(&claims.sub)?;
    Ok(Json(docs))
}

pub async fn get_doc_meta(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(doc_id): Path<String>,
) -> ServerResult<Json<DocMeta>> {
    let meta = state
        .db
        .get_doc_meta(&doc_id)?
        .ok_or(ServerError::NotFound)?;
    if meta.owner != claims.sub {
        return Err(ServerError::NotFound);
    }
    Ok(Json(meta))
}

pub async fn delete_doc(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(doc_id): Path<String>,
) -> ServerResult<impl IntoResponse> {
    let meta = state
        .db
        .get_doc_meta(&doc_id)?
        .ok_or(ServerError::NotFound)?;
    if meta.owner != claims.sub {
        return Err(ServerError::NotFound);
    }
    state.db.delete_doc(&doc_id)?;
    Ok(StatusCode::NO_CONTENT)
}
