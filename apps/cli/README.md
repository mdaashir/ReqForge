# reqforge CLI

Run ReqForge collections from the command line. Useful for CI/CD and headless test execution.

## Build

```bash
cargo build --release -p reqforge-cli
```

The binary will be at `target/release/reqforge`.

## Commands

### `reqforge list`

List all collections in the workspace.

```bash
reqforge list
reqforge list --format json
```

### `reqforge info`

Print workspace info.

```bash
reqforge info
```

### `reqforge validate`

Validate every collection in the workspace.

```bash
reqforge validate
```

### `reqforge run`

Run a collection or a single request.

```bash
reqforge run --collection <id>
reqforge run --collection <id> --request "Get Users"
reqforge run --collection <id> --env Production --format json
```

Exit codes:
- `0` — all requests passed
- `1` — validation error or runtime error
- `2` — one or more requests failed (only for `run`)

## Output formats

- `human` (default) — coloured, table-formatted output
- `json` — JSON for scripting
- `junit` — JUnit XML (planned; falls back to JSON for now)

## Example: CI integration

```yaml
# .github/workflows/api-tests.yml
- name: Run API tests
  run: |
    cargo build --release -p reqforge-cli
    ./target/release/reqforge run --collection core-api --format junit > results.xml
```
