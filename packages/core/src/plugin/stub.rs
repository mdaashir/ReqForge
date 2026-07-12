//! Stub plugin host used when the `plugins` feature is disabled.

use crate::plugin::{PluginManifest, PluginMessage, PluginPermission, PluginResponse};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginHostError {
    #[error("plugin support is not compiled in (build reqforge-core with --features plugins)")]
    Disabled,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub struct PluginHost {
    plugins: Vec<PluginManifest>,
}

impl PluginHost {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn load_from_dir(&mut self, _dir: impl AsRef<Path>) -> Result<(), PluginHostError> {
        Err(PluginHostError::Disabled)
    }

    pub fn plugins(&self) -> &[PluginManifest] {
        &self.plugins
    }

    pub fn has_permission(&self, plugin_id: &str, _perm: PluginPermission) -> bool {
        self.plugins.iter().any(|p| p.id == plugin_id)
    }

    pub fn dispatch(
        &self,
        _plugin_id: &str,
        _msg: PluginMessage,
    ) -> Result<PluginResponse, PluginHostError> {
        Ok(PluginResponse::Ok)
    }

    /// Persisted key/value store scoped to a plugin. Returns an empty map
    /// in the no-op implementation.
    pub fn storage(&self, _plugin_id: &str) -> HashMap<String, String> {
        HashMap::new()
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}
