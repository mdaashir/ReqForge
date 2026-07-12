# ReqForge

A modern, open-source API development platform. Native desktop app (Tauri) + CLI, built with Rust.

## Features

| Category | Capabilities |
|----------|-------------|
| **Protocols** | HTTP/1.1-3 (H2 always, H3 feature-gated), GraphQL, WebSocket, gRPC-Web, SSE, MQTT, Kafka, raw TCP/UDP, SOAP |
| **Import** | Postman v2.1, cURL, Insomnia v4, Bruno `.bru` |
| **Export** | cURL, fetch, axios, Python, Go, Ruby, PowerShell, Java |
| **Auth** | Basic, Bearer, API Key, OAuth 2.0 (PKCE), JWT (HS256/RS256) |
| **Testing** | 7 assertion types: status, latency, body, header, JSON path, regex, content-type |
| **Editor** | Monaco editor with JSON Schema validation (Ajv) |
| **Storage** | File-based YAML collections and environments |
| **CLI** | Run, list, validate, info, plugin search, load test |
| **Plugins** | wasmtime sandbox, JSON-in/JSON-out ABI |
| **Sync** | Yrs CRDT via WebSocket, multi-client broadcast |
| **Collab** | Live cursors, presence awareness |
| **Telemetry** | Opt-in crash reports + usage counters |
| **Command Palette** | Fuzzy search with 20+ actions |
| **History** | Append-only JSONL, replay, filters, day grouping |
| **Themes** | Light / Dark / System |
| **CI/CD** | GitHub Actions: clippy, test, build, release matrix |

## Architecture

```
reqforge/
├── apps/
│   ├── desktop/          # Tauri 2 desktop app (Rust + React)
│   └── cli/              # CLI companion (Rust)
├── packages/
│   ├── core/             # Shared Rust library (modules: auth, collection,
│   │                     #   environment, error, history, import, plugin,
│   │                     #   protocol, request, samples, sync, telemetry, testing)
│   └── ui/               # React UI components + hooks + stores
├── services/
│   ├── sync/             # Y-WebSocket sync server (axum + yrs)
│   ├── plugins/          # Plugin marketplace API (axum)
│   └── telemetry/        # Crash/usage ingestion (axum + SQLite)
├── e2e/                  # Playwright end-to-end tests
├── web/                  # Landing page + docs (Astro)
├── plugins/
│   └── header-juggler/   # Sample WASM plugin
└── .github/workflows/    # CI + release pipelines
```

## Quick Start

```bash
pnpm install
pnpm build
pnpm test

# Desktop app
cd apps/desktop && pnpm tauri dev

# CLI
cargo run -p reqforge-cli -- run https://api.example.com/users
```

## License

MIT
