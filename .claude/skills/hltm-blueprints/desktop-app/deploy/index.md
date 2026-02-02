# Deploy Module

## Build Commands

```bash
# Development
cargo tauri dev

# Production build
cargo tauri build

# Build specific target
cargo tauri build --target universal-apple-darwin  # macOS universal
cargo tauri build --target x86_64-pc-windows-msvc  # Windows
```

## macOS Signing

```bash
#!/bin/bash
# build.sh

APP_NAME="MyApp"
BUNDLE_ID="com.myapp.desktop"
TEAM_ID="YOUR_TEAM_ID"  # From Apple Developer account

# Build
cargo tauri build --target universal-apple-darwin

# Sign
codesign --force --deep --sign "Developer ID Application: Your Name ($TEAM_ID)" \
    --entitlements src-tauri/entitlements.plist \
    --options runtime \
    "target/release/bundle/macos/$APP_NAME.app"

# Create DMG
create-dmg \
    --volname "$APP_NAME" \
    --window-size 600 400 \
    --icon "$APP_NAME.app" 150 150 \
    --app-drop-link 450 150 \
    "$APP_NAME.dmg" \
    "target/release/bundle/macos/$APP_NAME.app"

# Sign DMG
codesign --sign "Developer ID Application: Your Name ($TEAM_ID)" "$APP_NAME.dmg"
```

## Notarization (macOS)

```bash
# Submit for notarization
xcrun notarytool submit "$APP_NAME.dmg" \
    --apple-id "your@email.com" \
    --team-id "$TEAM_ID" \
    --password "@keychain:AC_PASSWORD" \
    --wait

# Staple ticket
xcrun stapler staple "$APP_NAME.dmg"

# Verify
spctl -a -vvv "$APP_NAME.dmg"
```

## Verification Commands

```bash
# Check code signature
codesign -dvvv /path/to/MyApp.app

# Check entitlements
codesign -d --entitlements :- /path/to/MyApp.app

# Gatekeeper check
spctl -a -vvv /path/to/MyApp.app

# Check notarization
stapler validate /path/to/MyApp.app
```

## Tauri Config for Build

```json
// src-tauri/tauri.conf.json
{
  "bundle": {
    "active": true,
    "targets": ["dmg", "app"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "identifier": "com.myapp.desktop",
    "macOS": {
      "entitlements": "./entitlements.plist",
      "minimumSystemVersion": "13.0",
      "frameworks": [],
      "signingIdentity": "-"
    }
  }
}
```

## Icon Generation

```bash
# From 1024x1024 PNG source
mkdir -p icons
sips -z 32 32 icon.png --out icons/32x32.png
sips -z 128 128 icon.png --out icons/128x128.png
sips -z 256 256 icon.png --out icons/128x128@2x.png

# Create .icns for macOS
iconutil -c icns icons/icon.iconset -o icons/icon.icns
```

## GitHub Actions

```yaml
# .github/workflows/build.yml
name: Build

on:
  push:
    tags: ['v*']

jobs:
  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-action@stable

      - name: Install Tauri CLI
        run: cargo install tauri-cli

      - name: Build
        run: cargo tauri build --target universal-apple-darwin

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: macos-app
          path: target/release/bundle/macos/*.app
```
