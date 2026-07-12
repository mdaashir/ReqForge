# Header Juggler — ReqForge Plugin Sample

A minimal plugin for ReqForge that demonstrates the WASM plugin ABI.

## What it does

Intercepts every outgoing HTTP request and adds an `X-Header-Juggler: processed` header.

## Building

```bash
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/header_juggler.wasm .
```

The output `.wasm` is ~4 KB.

## Testing locally

Copy the plugin into your ReqForge workspace:

```bash
cp -R plugins/header-juggler ~/.reqforge/workspace/plugins/
```

Open ReqForge → Plugins → Header Juggler → Enable. Send any request and
check that `X-Header-Juggler: processed` appears in the response headers.

## ABI

The plugin exports two functions:

- `alloc(size: i32) -> i32` — request memory from the host (optional, falls back to static buffer if missing)
- `handle(input_len: i32, input_ptr: i32) -> i32` — process a JSON message and return a JSON response

The host writes a JSON `PluginMessage` into linear memory at `input_ptr`,
calls `handle`, then reads back a JSON `PluginResponse` from the plugin's
output buffer (4-byte length prefix + UTF-8 JSON).
