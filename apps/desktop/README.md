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

## Implemented commands

- `init_workspace(path)` — initialise workspace at `path`
- `send_request(request)` — execute an HTTP request
- `save_collection(collection)` / `load_collection(id)` / `list_collections` / `delete_collection(id)`
- `ping`, `get_app_name`, `get_app_version` — utility

## Theming

The app supports light, dark, and system theme. The theme is stored in `localStorage` and applied to `document.documentElement` via the `dark` class.
