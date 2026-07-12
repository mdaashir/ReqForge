# Auto-update + code signing

ReqForge uses Tauri's built-in updater, which verifies signatures against a
public key baked into `tauri.conf.json`.

## One-time key generation

Run this once per project. **Never commit the private key.**

```bash
cargo install tauri-cli --version "^2.0" --locked
cd apps/desktop/src-tauri
tauri signer generate --password "$KEY_PASSWORD" -w ~/.tauri/reqforge.key
```

Then:

1. Copy the printed public key into `tauri.conf.json` → `plugins.updater.pubkey`.
2. Add `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
   to your CI secrets.
3. Update `release.yml` to pass them to `tauri build`:

   ```yaml
   - name: Build desktop bundle
     working-directory: apps/desktop
     env:
       TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
       TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
     run: pnpm tauri build
   ```

## macOS notarisation

For macOS releases, add these secrets and uncomment the notarisation step:

| Secret | Purpose |
|--------|---------|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` Developer ID Application cert |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` |
| `APPLE_SIGNING_IDENTITY` | "Developer ID Application: Your Name (TEAMID)" |
| `APPLE_ID` | Apple ID email |
| `APPLE_PASSWORD` | App-specific password |
| `APPLE_TEAM_ID` | Apple Developer Team ID |

## Windows code signing

Add `WINDOWS_CERTIFICATE` (base64 `.pfx`) and `WINDOWS_CERTIFICATE_PASSWORD`.
Pass them via `tauri build --sign`.

## Update server

The updater polls the URL in `tauri.conf.json → plugins.updater.endpoints[0]`
on startup. The CI `release.yml` workflow must publish a `latest.json`
manifest alongside the platform-specific bundles; `tauri build` does this
automatically when the signing env vars are present.
