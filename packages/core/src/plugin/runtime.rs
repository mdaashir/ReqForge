//! Plugin runtime — WASM execution environment.
//!
//! Re-exports `PluginHost` from `host.rs` where the wasmtime-based
//! runtime is implemented.

#[cfg(feature = "plugins")]
pub use crate::PluginHost;

#[cfg(not(feature = "plugins"))]
/// Stub — plugins feature not enabled
pub struct PluginHost;
#[cfg(not(feature = "plugins"))]
impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginHost {
    pub fn new() -> Self { Self }
}

