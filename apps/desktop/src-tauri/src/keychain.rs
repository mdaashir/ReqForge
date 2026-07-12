//! OS credential store integration via `keyring-rs`.
//!
//! Provides secure storage for API tokens, environment variables, and
//! other credentials. Backed by:
//! - macOS: Keychain
//! - Linux: Secret Service (libsecret) — requires gnome-keyring or kwallet
//! - Windows: Credential Manager
//!
//! All operations use a single service name `reqforge` so multiple
//! keychain entries can coexist without collision.

use serde::{Deserialize, Serialize};

const SERVICE: &str = "reqforge";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialMeta {
    pub account: String,
    pub created_at_ms: u64,
}

/// Save a credential to the OS keychain.
pub async fn keychain_set(account: String, value: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let entry =
            keyring::Entry::new(SERVICE, &account).map_err(|e| format!("keychain entry: {e}"))?;
        entry
            .set_password(&value)
            .map_err(|e| format!("keychain set: {e}"))
    })
    .await
    .map_err(|e| format!("keychain task: {e}"))?
}

/// Retrieve a credential from the OS keychain. Returns `None` if the
/// account does not exist.
pub async fn keychain_get(account: String) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || {
        let entry =
            keyring::Entry::new(SERVICE, &account).map_err(|e| format!("keychain entry: {e}"))?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("keychain get: {e}")),
        }
    })
    .await
    .map_err(|e| format!("keychain task: {e}"))?
}

/// Delete a credential from the OS keychain. Returns `false` if it
/// didn't exist (idempotent — doesn't error).
pub async fn keychain_delete(account: String) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let entry =
            keyring::Entry::new(SERVICE, &account).map_err(|e| format!("keychain entry: {e}"))?;
        match entry.delete_password() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(format!("keychain delete: {e}")),
        }
    })
    .await
    .map_err(|e| format!("keychain task: {e}"))?
}

/// List all accounts we have stored. The OS keychain APIs don't
/// support enumeration, so we maintain a side-index file in the
/// workspace directory.
pub async fn keychain_list(workspace_root: String) -> Result<Vec<CredentialMeta>, String> {
    let index_path = std::path::PathBuf::from(&workspace_root)
        .join(".reqforge")
        .join("keychain_index.json");

    let bytes = match tokio::fs::read(&index_path).await {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("read index: {e}")),
    };

    serde_json::from_slice(&bytes).map_err(|e| format!("parse index: {e}"))
}

/// Internal helper to record a credential in our side-index so
/// `keychain_list` can enumerate accounts.
pub async fn record_account(workspace_root: &str, account: &str) -> Result<(), String> {
    let dir = std::path::PathBuf::from(workspace_root).join(".reqforge");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("mkdir: {e}"))?;
    let index_path = dir.join("keychain_index.json");

    let mut entries: Vec<CredentialMeta> = match tokio::fs::read(&index_path).await {
        Ok(b) => serde_json::from_slice(&b).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    if !entries.iter().any(|e| e.account == account) {
        entries.push(CredentialMeta {
            account: account.to_string(),
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        });
        let bytes = serde_json::to_vec_pretty(&entries).map_err(|e| format!("serialize: {e}"))?;
        tokio::fs::write(&index_path, bytes)
            .await
            .map_err(|e| format!("write index: {e}"))?;
    }

    Ok(())
}

pub async fn keychain_set_with_index(
    workspace_root: String,
    account: String,
    value: String,
) -> Result<(), String> {
    keychain_set(account.clone(), value.clone()).await?;
    record_account(&workspace_root, &account).await
}
