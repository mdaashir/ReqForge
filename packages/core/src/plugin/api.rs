//! Plugin API host functions exposed to WASM plugins.
//!
//! ponytail: define host functions (`host_http_request`, `host_storage_get`, etc.)
//! that plugins can import from the host environment. The current `PluginHost`
//! (in `host.rs`) has a message-passing ABI; this module would add typed wrappers.
//! Design: `PluginApi { registry: HashMap<String, Box<dyn Any>> }` with typed
//! get/set/call methods registered by the host before plugin instantiation.
