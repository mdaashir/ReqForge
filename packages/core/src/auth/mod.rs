//! Authentication providers
//!
//! Implements common authentication schemes for HTTP requests:
//! - API Key (header/query)
//! - Bearer token
//! - Basic auth
//! - OAuth 2.0 (with PKCE)
//! - JWT (HS256/RS256)

pub mod api_key;
pub mod aws_sig_v4;
mod basic;
mod bearer;
mod jwt;
mod jwt_token;
mod oauth2;
pub mod pkce;
mod types;
pub mod vault;

pub use basic::BasicAuth;
pub use bearer::BearerAuth;
pub use jwt::JwtAuth;
pub use jwt_token::{decode_unverified, sign_hs256, sign_rs256, verify_hs256, Claims, JwtHeader, JwtKey};
pub use oauth2::OAuth2Auth;
pub use types::{ApiKeyAuth, ApiKeyLocation, AuthCredentials, AuthProvider, AuthType};
