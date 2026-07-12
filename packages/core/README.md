# reqforge-core

Shared Rust library powering both the ReqForge desktop app and the CLI.

## Modules

| Module | Purpose |
|--------|---------|
| `auth` | 6 auth providers: API Key, Bearer, Basic, OAuth2 PKCE, JWT (HS256/RS256), AWS SigV4 |
| `collection` | YAML file-based collections with folder nesting, runner, and atomic writes |
| `crypto` | AES-256-GCM at-rest encryption with iterated HMAC-SHA256 KDF |
| `environment` | Multi-level variable resolution (local > collection > env > global) + dynamic vars |
| `error` | Unified `Result<T, Error>` with `connection`, `auth`, `script`, `protocol` helpers |
| `history` | SQLite-backed request history with search and JSONL migration |
| `import` | Importers: Postman v2.1, cURL, Insomnia v4, Bruno, HAR (stub), OpenAPI (stub) |
| `loadtest` | Concurrent request load testing framework |
| `mock` | In-process Axum mock server with dynamic rules and delay injection |
| `plugin` | WASM sandboxed plugin runtime (wasmtime) with capability-based permissions |
| `protocol` | 10 protocol handlers: HTTP/1-3, GraphQL, WebSocket, gRPC-Web, SSE, MQTT, Kafka, TCP/UDP, SOAP |
| `request` | Request/Response types, executor with variable resolution + auth + scripting pipeline |
| `scripting` | Rhai-based pre-request/post-response scripts with `rf.*` API surface |
| `storage` | Aggregated access to collection, environment, history, and snapshot storage |
| `sync` | Yjs-compatible CRDT sync engine + file watcher + git auto-commit |
| `testing` | 9 assertion types, JSON Schema (Draft-07), snapshot testing, 4 report formats |
| `watcher` | `notify`-based file watcher with debounce + `git2` auto-commit integration |

## Usage

```rust
use reqforge_core::request::{Request, HttpMethod, RequestExecutor};

let executor = RequestExecutor::new()?;
let request = Request::new(HttpMethod::Get, "https://api.example.com/users");
let response = executor.execute(request).await?;
println!("Status: {}", response.status);
```

## Workspace layout

```
workspace/
├── collections/
│   ├── <id>/
│   │   └── collection.yaml
├── environments/
│   └── *.yaml
└── .reqforge/
    ├── history.db
    ├── snapshots/
    └── config.yaml
```

## Tests

```bash
cargo test -p reqforge-core  # 142 unit + 9 property tests
```
