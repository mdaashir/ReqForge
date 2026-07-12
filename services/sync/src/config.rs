//! Server configuration loaded from environment variables.

use crate::error::{ServerError, ServerResult};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub jwt_secret: String,
    pub max_doc_size_bytes: u64,
    pub max_clients_per_doc: u32,
    pub ping_interval_ms: u64,
}

impl Config {
    pub fn from_env() -> ServerResult<Self> {
        Ok(Self {
            bind_addr: env::var("REQFORGE_BIND")
                .unwrap_or_else(|_| "0.0.0.0:7443".to_string()),
            database_url: env::var("REQFORGE_DB_URL")
                .unwrap_or_else(|_| "sqlite://reqforge-sync.db?mode=rwc".to_string()),
            jwt_secret: env::var("REQFORGE_JWT_SECRET")
                .map_err(|_| {
                    ServerError::Config(
                        "REQFORGE_JWT_SECRET must be set (≥32 random bytes)".into(),
                    )
                })?
                .trim()
                .to_string(),
            max_doc_size_bytes: env::var("REQFORGE_MAX_DOC_SIZE_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(16 * 1024 * 1024), // 16 MiB
            max_clients_per_doc: env::var("REQFORGE_MAX_CLIENTS_PER_DOC")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(64),
            ping_interval_ms: env::var("REQFORGE_PING_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30_000),
        })
    }
}
