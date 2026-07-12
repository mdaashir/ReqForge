# Header Juggler — ReqForge Plugin Sample

A minimal WASM plugin for ReqForge that demonstrates the plugin ABI.

## What it does

Intercepts outgoing HTTP requests and adds `X-Header-Juggler: processed`.

## Building

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/header_juggler.wasm .
```

Output: ~4 KB `.wasm`.

## Requirements

ReqForge must be built with `plugins` feature to load WASM plugins:
```bash
cargo build --features plugins
```

## Testing locally

```bash
cp path/to/header_juggler/ ~/.reqforge/workspace/plugins/
# Enable via ReqForge → Plugins → Header Juggler
# All requests will get: X-Header-Juggler: processed
```

## ABI

The plugin exports:
- `alloc(size: i32) -> i32` — allocate memory (optional, falls back to static buffer)
- `handle(input_len, input_ptr) -> i32` — process JSON message, return JSON response

## ABI

The plugin exports two functions:

- `alloc(size: i32) -> i32` — request memory from the host (optional, falls back to static buffer if missing)
- `handle(input_len: i32, input_ptr: i32) -> i32` — process a JSON message and return a JSON response

The host writes a JSON `PluginMessage` into linear memory at `input_ptr`,
calls `handle`, then reads back a JSON `PluginResponse` from the plugin's
output buffer (4-byte length prefix + UTF-8 JSON).
