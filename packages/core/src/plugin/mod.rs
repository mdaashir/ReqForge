//! Plugin system — WASM sandboxed extensions for ReqForge.
//!
//! Plugins are `.wasm` files exposing a small JSON-in / JSON-out ABI.
//! The host (`PluginHost`) loads them with `wasmtime`, enforces resource
//! limits (memory, fuel, epoch interruption), and routes hook callbacks
//! (`on_request`, `on_response`) through them.
//!
//! ## ABI
//!
//! A plugin exports one function:
//!
//! ```wat
//! (func (export "handle") (param i32 i32) (result i32))
//! ```
//!
//! The host passes a pointer + length to a JSON-encoded `PluginMessage` in
//! the plugin's linear memory and reads back a pointer to a JSON
//! `PluginResponse`. The host owns all allocations; the plugin borrows.
//!
//! ## Hooks
//!
//! `on_request` fires before a request is sent. The plugin can mutate the
//! request or short-circuit by returning a synthetic response.
//! `on_response` fires after a response comes back. The plugin can mutate
//! the response or attach metadata.

pub mod api;
pub mod loader;
pub mod manifest;
pub mod runtime;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Plugin manifest as found in `plugin.toml` alongside the `.wasm` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Path to the wasm binary relative to the manifest.
    pub wasm: String,
    /// Optional permissions the plugin requests. The host may downgrade.
    #[serde(default)]
    pub permissions: Vec<PluginPermission>,
    /// ABI version this plugin targets. Plugins with incompatible ABIs are
    /// refused at load time.
    #[serde(default = "default_abi_version")]
    pub abi_version: u32,
}

fn default_abi_version() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    /// Can read and mutate outgoing requests.
    ReadRequests,
    /// Can read and mutate incoming responses.
    ReadResponses,
    /// Can fetch arbitrary URLs from the host.
    Network,
    /// Can persist data into the plugin's own key/value store.
    Storage,
    /// Can log messages back to the host's log.
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginMessage {
    /// Plugin handshake initiated by the host on load.
    Init {
        plugin_id: String,
        abi_version: u32,
    },
    /// Fired before a request is sent.
    OnRequest {
        request_id: String,
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: Option<String>,
    },
    /// Fired after a response is received.
    OnResponse {
        request_id: String,
        status: u16,
        url: String,
        headers: HashMap<String, String>,
        body: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginResponse {
    /// Pass-through: no change.
    Ok,
    /// Replace the request/response body.
    Replace {
        body: Option<String>,
        headers: Option<HashMap<String, String>>,
    },
    /// Cancel the operation.
    Cancel { reason: String },
    /// Log a message back to the host.
    Log { level: String, message: String },
    /// Persist a value in the plugin's key/value store.
    SetStorage { key: String, value: String },
}

#[cfg(feature = "plugins")]
mod host;
#[cfg(feature = "plugins")]
pub use host::{PluginHost, PluginHostError};

#[cfg(not(feature = "plugins"))]
mod stub;
#[cfg(not(feature = "plugins"))]
pub use stub::{PluginHost, PluginHostError};

/// Thread-safe handle to a loaded plugin host. Useful for sharing a
/// single `PluginHost` across request execution tasks.
pub type SharedPluginHost = Arc<PluginHost>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parses() {
        let toml = r#"
            id = "reqforge.my-plugin"
            name = "My Plugin"
            version = "0.1.0"
            wasm = "plugin.wasm"
            permissions = ["read_requests", "log"]
            abi_version = 1
        "#;
        let manifest: PluginManifest = toml::from_str(toml).unwrap();
        assert_eq!(manifest.id, "reqforge.my-plugin");
        assert_eq!(manifest.permissions.len(), 2);
    }

    #[test]
    fn test_message_round_trip() {
        let msg = PluginMessage::OnRequest {
            request_id: "req-1".into(),
            method: "GET".into(),
            url: "https://example.com".into(),
            headers: HashMap::from([("Accept".into(), "application/json".into())]),
            body: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: PluginMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            PluginMessage::OnRequest { method, .. } => assert_eq!(method, "GET"),
            _ => panic!("wrong variant"),
        }
    }
}
