//! JWT bearer authentication for the sync server.
//!
//! Tokens are HS256-signed by the desktop client using a shared secret
//! bootstrapped at install time. The server validates the signature and
//! extracts the `sub` claim as the owner identifier.

use crate::error::{ServerError, ServerResult};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    #[serde(default)]
    pub email: Option<String>,
}

pub fn issue_token(
    secret: &str,
    sub: &str,
    email: Option<&str>,
    ttl_secs: i64,
) -> ServerResult<String> {
    if secret.len() < 32 {
        return Err(ServerError::Auth(
            "JWT secret must be at least 32 bytes".into(),
        ));
    }
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        + ttl_secs;
    let claims = Claims {
        sub: sub.to_string(),
        exp,
        email: email.map(|s| s.to_string()),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn verify_token(secret: &str, token: &str) -> ServerResult<Claims> {
    let mut validation = Validation::default();
    validation.leeway = 30;
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| ServerError::Auth(format!("invalid token: {e}")))?;
    Ok(data.claims)
}

/// Extractor that pulls a Bearer token from the Authorization header
/// and returns the decoded claims. 401 if missing or invalid.
pub struct AuthUser(pub Claims);

#[axum::async_trait]
impl FromRequestParts<crate::AppState> for AuthUser {
    type Rejection = ServerError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .ok_or_else(|| ServerError::Auth("missing Authorization header".into()))?;
        let header_str = header
            .to_str()
            .map_err(|_| ServerError::Auth("invalid Authorization header".into()))?;
        let token = header_str
            .strip_prefix("Bearer ")
            .or_else(|| header_str.strip_prefix("bearer "))
            .ok_or_else(|| ServerError::Auth("expected Bearer scheme".into()))?;
        let claims = verify_token(&state.config.jwt_secret, token)?;
        Ok(AuthUser(claims))
    }
}
