# reqforge-core

Shared Rust library powering both the ReqForge desktop app and the CLI. Provides protocol handlers, request execution, collection storage, environment resolution, authentication, and a testing engine.

## Modules

| Module | Purpose |
|---|---|
| `request` | HTTP request/response types and the `RequestExecutor` (uses `reqwest`) |
| `protocol` | Pluggable protocol handlers (`HttpHandler`, `GraphQLHandler`, `WebSocketHandler`) |
| `collection` | File-based collections stored as YAML, Git-friendly |
| `environment` | Variable resolution with `{{var}}` interpolation and dynamic vars (`$uuid`, etc.) |
| `auth` | API Key, Bearer, Basic, OAuth 2.0, and JWT providers |
| `testing` | Assertions and `TestRunner` for response validation |
| `error` | Unified `Result<T, Error>` type |

## Usage

```rust
use reqforge_core::prelude::*;

let executor = RequestExecutor::new()?;
let request = Request::new(HttpMethod::Get, "https://api.example.com/users");
let response = executor.execute(request).await?;

println!("Status: {}", response.status);
println!("Body: {}", response.body.text());
```

## Workspace layout

```
workspace/
├── collections/
│   ├── <id>/
│   │   └── collection.yaml
```

Each collection is stored as a YAML document inside its own directory, making it trivial to diff and version with Git.

## Tests

```bash
cargo test -p reqforge-core
```
