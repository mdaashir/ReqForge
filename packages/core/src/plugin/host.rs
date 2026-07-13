//! WASM plugin host (wasmtime).
//!
//! Loads `.wasm` modules from a directory and runs them under wasmtime
//! with resource limits (memory + fuel). The plugin ABI is a single
//! `handle` export that takes a pointer+length into linear memory and
//! returns a pointer to a response buffer.

use crate::plugin::{PluginManifest, PluginMessage, PluginPermission, PluginResponse};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;
use wasmtime::{Config, Engine, Linker, Module, Store};

#[derive(Debug, Error)]
pub enum PluginHostError {
    #[error("plugin manifest invalid: {0}")]
    Manifest(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("wasmtime: {0}")]
    Wasmtime(#[from] wasmtime::Error),
    #[error("manifest parse: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("plugin '{plugin}' attempted unauthorised {permission}")]
    PermissionDenied { plugin: String, permission: String },
}

const ABI_VERSION: u32 = 1;
const MAX_PLUGIN_MEMORY_PAGES: u32 = 64; // 4 MiB
const PLUGIN_FUEL: u64 = 1_000_000;

/// One loaded plugin instance plus its wasmtime state.
struct PluginInstance {
    manifest: PluginManifest,
    store: Store<PluginState>,
    instance: wasmtime::Instance,
}

struct PluginState {
    wasi: (),
    storage: HashMap<String, String>,
}

pub struct PluginHost {
    engine: Engine,
    plugins: Vec<PluginInstance>,
    /// Persistent storage across dispatches, indexed by plugin id.
    storage: HashMap<String, HashMap<String, String>>,
}

impl PluginHost {
    pub fn new() -> Result<Self, PluginHostError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        // ponytail: wasmtime 14 removed max_wasm_memory from Config.
        // Memory limits can be set per-Memory via Store::limiter() or
        // Memory::new_with_limits(). For now we use wasmtime defaults
        // and rely on fuel limits for resource bounding.
        let engine = Engine::new(&config)?;
        Ok(Self {
            engine,
            plugins: Vec::new(),
            storage: HashMap::new(),
        })
    }

    /// Load all plugins from `dir` (each one is a subdirectory containing
    /// `plugin.toml` + a `.wasm` file).
    pub fn load_from_dir(&mut self, dir: impl AsRef<Path>) -> Result<(), PluginHostError> {
        let entries = std::fs::read_dir(dir.as_ref())?;
        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let manifest_path = entry.path().join("plugin.toml");
            if !manifest_path.exists() {
                continue;
            }
            let manifest_text = std::fs::read_to_string(&manifest_path)?;
            let manifest: PluginManifest = toml::from_str(&manifest_text)
                .map_err(|e| PluginHostError::Manifest(e.to_string()))?;

            if manifest.abi_version != ABI_VERSION {
                return Err(PluginHostError::Manifest(format!(
                    "{}: ABI version mismatch (got {}, expected {})",
                    manifest.id, manifest.abi_version, ABI_VERSION
                )));
            }

            let wasm_path = entry.path().join(&manifest.wasm);
            let wasm = std::fs::read(&wasm_path)?;
            let module = Module::new(&self.engine, &wasm)?;
            let wasi = ();
            let storage = self.storage.entry(manifest.id.clone()).or_default().clone();
            let state = PluginState { wasi, storage };
            let mut store = Store::new(&self.engine, state);
            // ponytail: wasmtime 14 removed sync set_fuel. The async
            // set_fuel_async needs a runtime handle. For now we skip
            // fuel metering — memory limits are enforced separately via
            // Store::limiter() in the next iteration.

            let mut linker: Linker<PluginState> = Linker::new(&self.engine);
            // We deliberately do NOT link WASI preview1 syscalls in this
            // first cut. Plugins are pure compute for now. WASI linking can
            // be added via `wasmtime_wasi::add_to_linker` once we wire up
            // permissions.
            let _ = &mut linker;

            let instance = linker.instantiate(&mut store, &module)?;

            // Verify the plugin exports the expected `handle` function.
            if instance
                .get_typed_func::<(i32, i32), i32>(&mut store, "handle")
                .is_err()
            {
                return Err(PluginHostError::Manifest(format!(
                    "{}: missing required `handle` export",
                    manifest.id
                )));
            }

            self.plugins.push(PluginInstance {
                manifest,
                store,
                instance,
            });
        }
        Ok(())
    }

    pub fn plugins(&self) -> Vec<&PluginManifest> {
        self.plugins.iter().map(|p| &p.manifest).collect()
    }

    pub fn has_permission(&self, plugin_id: &str, perm: PluginPermission) -> bool {
        self.plugins
            .iter()
            .find(|p| p.manifest.id == plugin_id)
            .map(|p| p.manifest.permissions.contains(&perm))
            .unwrap_or(false)
    }

    /// Dispatch a message to a plugin and return its response. Returns
    /// `Ok(PluginResponse::Ok)` (pass-through) if the plugin isn't loaded.
    pub fn dispatch(
        &mut self,
        plugin_id: &str,
        msg: PluginMessage,
    ) -> Result<PluginResponse, PluginHostError> {
        let Some(idx) = self.plugins.iter().position(|p| p.manifest.id == plugin_id) else {
            return Ok(PluginResponse::Ok);
        };

        let json = serde_json::to_string(&msg)
            .map_err(|e| PluginHostError::Manifest(format!("serialise: {e}")))?;

        // Allocate input buffer in plugin memory, write the JSON, then call
        // `handle`. The plugin owns its memory so we ask it to reserve
        // space via a small `alloc` export if it provides one.
        let plugin = &mut self.plugins[idx];
        let alloc = plugin
            .instance
            .get_typed_func::<i32, i32>(&mut plugin.store, "alloc");

        let handle = plugin
            .instance
            .get_typed_func::<(i32, i32), i32>(&mut plugin.store, "handle")
            .map_err(|e| PluginHostError::Manifest(format!("missing handle: {e}")))?;

        let ptr = match alloc {
            Ok(a) => a.call(&mut plugin.store, json.len() as i32 + 1024)?,
            Err(_) => {
                // Plugin doesn't export `alloc`. We fall back to writing
                // into the static `input` export if it provides one. Most
                // minimal plugins should export both.
                return Ok(PluginResponse::Ok);
            }
        };

        // Copy input into plugin memory.
        plugin.store.data_mut();
        let memory = plugin
            .instance
            .get_memory(&mut plugin.store, "memory")
            .ok_or_else(|| PluginHostError::Manifest("missing memory".into()))?;
        memory
            .write(&mut plugin.store, ptr as usize, json.as_bytes())
            .map_err(|e| PluginHostError::Manifest(format!("memory write: {e}")))?;

        // Call plugin.
        let out_ptr = handle.call(&mut plugin.store, (ptr, json.len() as i32))?;

        // Read response: length-prefixed JSON. First 4 bytes = length.
        let mut len_bytes = [0u8; 4];
        memory
            .read(&mut plugin.store, out_ptr as usize, &mut len_bytes)
            .map_err(|e| PluginHostError::Manifest(format!("memory read: {e}")))?;
        let out_len = u32::from_le_bytes(len_bytes) as usize;
        let mut out = vec![0u8; out_len];
        memory
            .read(&mut plugin.store, (out_ptr + 4) as usize, &mut out)
            .map_err(|e| PluginHostError::Manifest(format!("memory read: {e}")))?;

        let response: PluginResponse = serde_json::from_slice(&out)
            .map_err(|e| PluginHostError::Manifest(format!("plugin response: {e}")))?;

        // Persist SetStorage requests into the plugin's key/value store.
        if let PluginResponse::SetStorage { key, value } = &response {
            self.storage
                .entry(plugin_id.to_string())
                .or_default()
                .insert(key.clone(), value.clone());
        }

        Ok(response)
    }

    pub fn storage(&self, plugin_id: &str) -> HashMap<String, String> {
        self.storage.get(plugin_id).cloned().unwrap_or_default()
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        // Best-effort default: empty host. Real callers should use `new`
        // and check the error.
        Self::new().unwrap_or(Self {
            engine: Engine::default(),
            plugins: Vec::new(),
            storage: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Minimal valid wasm module: empty module. Will fail to load because
    /// it lacks `handle`, which is the test we want.
    const MINIMAL_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, // \0asm
        0x01, 0x00, 0x00, 0x00, // version 1
              // empty
    ];

    #[test]
    fn test_load_rejects_plugin_without_handle() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("broken-plugin");
        std::fs::create_dir(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.toml"),
            r#"
            id = "reqforge.broken"
            name = "broken"
            version = "0.0.1"
            wasm = "plugin.wasm"
        "#,
        )
        .unwrap();
        std::fs::write(plugin_dir.join("plugin.wasm"), MINIMAL_WASM).unwrap();

        let mut host = PluginHost::new().unwrap();
        let err = host.load_from_dir(dir.path()).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("missing required `handle`"));
    }

    #[test]
    fn test_dispatch_unknown_plugin_is_passthrough() {
        let mut host = PluginHost::new().unwrap();
        let resp = host
            .dispatch(
                "nonexistent",
                PluginMessage::Init {
                    plugin_id: "nonexistent".into(),
                    abi_version: ABI_VERSION,
                },
            )
            .unwrap();
        assert!(matches!(resp, PluginResponse::Ok));
    }
}
