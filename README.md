# ReqForge

A modern, open-source API development platform. Desktop-first (Tauri) + CLI companion, built with Rust + React.

## Features

| Category | Capabilities |
|----------|-------------|
| **Protocols** | HTTP/1.1-3, GraphQL, WebSocket, gRPC-Web, SSE, MQTT, Kafka, TCP/UDP, SOAP |
| **Request editor** | Method, URL, headers, query params, body (JSON/XML/form/binary/GraphQL), auth, scripts |
| **Auth** | API Key, Bearer, Basic, OAuth 2.0 (PKCE), JWT (HS256/RS256), AWS Signature V4 |
| **Testing** | 9 assertion types + JSON Schema validation + snapshot testing + 4 report formats |
| **Scripting** | Pre-request / post-response scripts via Rhai engine with `rf.*` API |
| **Collections** | YAML file-based, unlimited nesting, drag-and-drop reorder, Git-friendly |
| **Environments** | Multi-level variables (local > collection > env > global), dynamic variables, secrets |
| **Import** | Postman v2.1, cURL, Insomnia v4, Bruno, HAR, OpenAPI (stub) |
| **History** | SQLite-backed with search, filtering, replay |
| **Mock server** | In-process (Axum), dynamic rules, request matching, delay injection |
| **Plugin system** | WASM sandbox (wasmtime), capability-based permissions, hook ABI |
| **CRDT Sync** | Yjs-compatible via WebSocket, multi-client broadcast, presence awareness |
| **File watcher** | Auto-detect collection/environment changes, auto-commit to Git |
| **CLI** | Run collections, test, mock server, import/export, validate, info |
| **Desktop** | Tauri 2, OS keychain, OAuth browser flow, 23 IPC commands |
| **UI** | Monaco editor, drag-and-drop tree, command palette, light/dark/system themes |

## Architecture

```
reqforge/
├── apps/
│   ├── desktop/          # Tauri 2 desktop app (Rust + React)
│   └── cli/              # CLI companion (Rust)
├── packages/
│   ├── core/             # reqforge-core — shared Rust library
│   │   └── src/          # auth, collection, crypto, environment, error,
│   │                     # history, import, loadtest, mock, plugin, protocol,
│   │                     # request, samples, scripting, storage, sync, testing, watcher
│   └── ui/               # React components, hooks, stores
├── services/
│   ├── sync/             # Y-WebSocket sync server (Axum + Yrs)
│   ├── plugins/          # Plugin marketplace API (Axum)
│   └── telemetry/        # Crash/usage ingestion (Axum + SQLite)
├── e2e/                  # Playwright end-to-end tests
├── web/                  # Documentation site (Astro)
├── plugins/
│   └── header-juggler/   # Sample WASM plugin
├── docs/
│   ├── architecture/adr/ # Architecture Decision Records (3)
│   ├── core/             # Product vision
│   └── developement/     # Blueprint documents (7 parts)
└── .github/workflows/    # CI, build, release, security audit
```

## Quick Start

```bash
# Build core library
cd packages/core && cargo build

# Run tests
cargo test

# CLI
cargo run -p reqforge-cli -- run --url https://httpbin.org/get

# Desktop app (requires Tauri prerequisites)
cd apps/desktop && cargo tauri dev
```

## Project Status

All P0 blueprint requirements implemented. 142 unit + 9 property tests passing.
See [BLUEPRINT.md](docs/developement/BLUEPRINT.md) for full scope.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, code quality, commit conventions, and architectural guidelines.

## License

AGPL-3.0 — see [LICENSE](LICENSE).
