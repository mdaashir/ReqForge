# Contributing to ReqForge

## Development setup

### Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.75+ | Core + CLI + Desktop |
| Node.js | 18+ | UI package |
| pnpm | 8+ | Workspace management |
| Tauri CLI | 2.x | Desktop builds |

### One-time setup

```bash
# Install pre-commit hooks
git config core.hooksPath .githooks

# Or use the pre-commit framework (cross-platform):
# pip install pre-commit && pre-commit install

# Install UI dependencies
pnpm install
```

### Build & test

```bash
# Core library
cargo build -p reqforge-core
cargo test -p reqforge-core --lib

# CLI
cargo build -p reqforge-cli
cargo run -p reqforge-cli -- --help

# Desktop (requires Tauri deps)
cd apps/desktop && cargo tauri dev

# UI typecheck
pnpm --filter @reqforge/ui typecheck
```

## Project structure

```
reqforge/
├── apps/
│   ├── desktop/          # Tauri 2 desktop (Rust + Vite/React)
│   │   ├── src/          #   React frontend
│   │   └── src-tauri/    #   Rust backend (IPC, keychain, OAuth)
│   └── cli/              # CLI companion (Rust, clap)
├── packages/
│   ├── core/             # reqforge-core shared library
│   │   └── src/          #   20 modules: auth, protocol, testing, etc.
│   └── ui/               # React component library
│       └── src/          #   47 components, 13 hooks, 7 stores
├── services/             # Optional backend services
│   ├── sync/             #   Y-WebSocket CRDT sync server
│   ├── plugins/          #   Plugin marketplace API
│   └── telemetry/        #   Anonymous usage ingestion
├── e2e/                  # Playwright end-to-end tests
└── docs/                 # Architecture + development docs
```

## Code quality

Every PR must pass:

```bash
# Format
cargo fmt --all -- --check

# Lint (core)
cargo clippy --package reqforge-core -- -D warnings

# Lint (CLI)
cargo clippy --manifest-path apps/cli/Cargo.toml -- -D warnings

# Test
cargo test -p reqforge-core --lib
```

These run automatically in CI (`.github/workflows/ci.yml`).

## Commit conventions

```
<type>(<scope>): <imperative subject>

<body>

<footer>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `ci`.
Scopes: `core`, `cli`, `desktop`, `ui`, `docs`, `ci`.

Examples:
```
feat(core): add HTTP/3 support using quinn
fix(cli): handle empty collection gracefully
docs: update ADR-0001 with Tauri 2 upgrade notes
```

## Architectural guidelines

1. **Core has no Tauri dependency** — `reqforge-core` must compile without `tauri`.
2. **Desktop commands are thin wrappers** — IPC commands in `lib.rs` delegate to core.
3. **Feature-gate heavy deps** — `kafka`, `mqtt`, `wasmtime`, `yrs`, `tonic` are optional.
4. **File-based storage is crash-safe** — atomic `.tmp` + `rename` pattern.
5. **All secrets go to OS keychain** — never store credentials in config files.
6. **Scripts are sandboxed** — Rhai engine caps variables (1024), call depth (64), operations (1M).
7. **Plugins are WASM only** — no native code, capability-based permissions.

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting.

## License

AGPL-3.0 — see [LICENSE](LICENSE).
