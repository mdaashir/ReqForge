# reqforge CLI

Run ReqForge collections from the command line for CI/CD and headless test execution.

## Build

```bash
cargo build --release -p reqforge-cli
```

Binary at `target/release/reqforge`.

## Commands

| Command | Description |
|---------|-------------|
| `run` | Execute a collection or single request |
| `test` | Run tests from a collection with pass/fail reporting |
| `mock` | Start a local mock server |
| `import` | Import collections (Postman, cURL, Insomnia) |
| `export` | Export collections (JSON, YAML) |
| `list` | List all collections |
| `info` | Show workspace info |
| `validate` | Validate all collections |
| `plugin` | Search and install marketplace plugins |

### `run`

```bash
reqforge run --collection <id>
reqforge run --collection <id> --env Production --format json
```

### `test`

```bash
reqforge test --collection <id>
reqforge test --collection <id> --env Staging
```

Exit codes: `0` = all passed, `1` = runtime error, `2` = assertions failed.

### `mock`

```bash
reqforge mock
reqforge mock --port 8080
```

### `import`

```bash
reqforge import --file collection.json
reqforge import --file collection.json --format postman --name "My API"
```

### `export`

```bash
reqforge export --collection <id>
reqforge export --collection <id> --format yaml --output ./backup.yaml
```

## Output formats

- `human` (default) — coloured table output
- `json` — structured JSON
- `junit` — JUnit XML for CI

## CI integration

```yaml
- name: Run API tests
  run: |
    cargo build --release -p reqforge-cli
    ./target/release/reqforge test --collection core-api --format junit > results.xml
```
