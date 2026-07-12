//! Plugin loader — discovers and loads plugins from the filesystem.
//!
//! ponytail: implement when plugin discovery is needed beyond the
//! explicit `PluginHost::new(data)` path. The plugin host in `host.rs`
//! already handles WASM instantiation; this module would add filesystem
//! scanning, dependency resolution, and version compatibility checks.
//! Design: `PluginLoader { plugins_dir: PathBuf }` scans for `.plugin.toml`
//! files and returns `Vec<PluginManifest>`.
