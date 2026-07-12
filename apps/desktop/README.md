# @reqforge/desktop

ReqForge desktop app — built with Tauri 2 + React 18 + TypeScript.

## Stack

- **Tauri 2** — desktop runtime (Rust + system webview)
- **React 18 + Vite** — frontend
- **@reqforge/ui** — shared component library
- **reqforge-core** — shared Rust core

## Development

```bash
# Install dependencies (workspace-wide)
pnpm install

# Start the Tauri dev shell
pnpm tauri:dev

# Build a release binary
pnpm tauri:build
```

## Architecture

The frontend lives in `src/` and the Rust backend in `src-tauri/`. Communication happens over the type-safe Tauri IPC bridge. Commands are defined in `src-tauri/src/lib.rs` and called from React via `invoke<T>("command_name", payload)`.

### Frontend entry points

- `src/main.tsx` — React root
- `src/App.tsx` — main UI shell, uses `@reqforge/ui` components
- `src/styles/global.css` — global styles + theme variables

### Backend entry points

- `src-tauri/src/main.rs` — process entry, calls `lib::run()`
- `src-tauri/src/lib.rs` — Tauri commands and builder

## IPC commands (23)

| Command | Description |
|---------|-------------|
| `init_workspace` | Initialise workspace at path |
| `bootstrap_workspace` | Create default workspace structure |
| `ping` | Health check |
| `get_app_version` | Return app version |
| `send_request` | Execute HTTP request via core engine |
| `run_tests` | Run test suite against a collection |
| `save_collection` | Save collection to YAML |
| `load_collection` | Load collection by ID |
| `list_collections` | List all collection IDs |
| `delete_collection` | Delete collection |
| `import_collection` | Import from Postman/curl/Insomnia |
| `import_environments` | Import environments from file |
| `record_history` | Write entry to history |
| `list_history` | List recent history (newest first) |
| `search_history` | Search history by URL |
| `clear_history` | Delete all history |
| `replay_history` | Re-load request from history |
| `save_environment` | Save environment to YAML |
| `load_environment` | Load environment by name |
| `list_environments` | List all environments |
| `delete_environment` | Delete environment |
| `start_oauth_flow` | Launch browser OAuth 2.0 PKCE flow |
| `keychain_set` / `get` / `delete` / `list` | OS keychain operations |

## Theming

Light, dark, and system themes via `localStorage` + `prefers-color-scheme`.
