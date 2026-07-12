//! Server error types.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub type ServerResult<T> = Result<T, ServerError>;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("config: {0}")]
    Config(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("auth: {0}")]
    Auth(String),

    #[error("jwt: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("not found")]
    NotFound,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("addr parse: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    #[error("payload too large")]
    PayloadTooLarge,

    #[error("internal: {0}")]
    Internal(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ServerError::Config(_) | ServerError::Internal(_) | ServerError::Other(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            ServerError::Auth(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            ServerError::NotFound => (StatusCode::NOT_FOUND, "not found".into()),
            ServerError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ServerError::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, self.to_string()),
            ServerError::Io(_) | ServerError::Db(_) | ServerError::Jwt(_) | ServerError::AddrParse(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage failure".into(),
            ),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
