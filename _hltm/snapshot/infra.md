# Infrastructure

## Environments

| Env | Description | Deploy method |
|-----|-------------|--------------|
| dev | Local development | `cargo check` / `cargo tauri dev` |
| prod | macOS app bundle | `cargo tauri build` / `./build.sh` |

No staging environment. Desktop app distributed directly.

## CI/CD

No CI/CD pipeline configured (no `.github/workflows/`, no `.gitlab-ci.yml`). Builds are local.

Build script: `build.sh` — production build with code signing and notarization.

## Config

- `src-tauri/tauri.conf.json`: Tauri app config (window size, bundle settings, CSP, sidecar)
- `src-tauri/tauri.dev.conf.json`: Dev-specific overrides
- `src-tauri/Cargo.toml`: Rust dependencies
- `package.json`: JS dependencies, scripts
- App settings at runtime: `~/Library/Application Support/com.skopanev.nbp/settings.json`
- Secrets: API keys stored in macOS Keychain (service: `com.skopanev.nbp`). Dev mode fallback: `.dev-credentials.json` (gitignored)
- Entitlements: `src-tauri/entitlements.plist` (mic access, screen recording, network)

## Monitoring

- Logs: Rust `log` crate, visible in dev console during `cargo tauri dev`
- Health: `src/ui-health-check.js` — frontend UI health validation
- No remote monitoring — local desktop app
