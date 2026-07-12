//! API Key authentication provider.
//!
//! Re-exports from `auth::types` where `ApiKeyAuth` is defined alongside
//! the `AuthProvider` trait to avoid circular deps.
pub use crate::auth::types::ApiKeyAuth as ApiKeyAuth;
