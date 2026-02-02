# Tech Stack

## Framework

```
tauri               # Desktop framework (Rust + Webview)
rust                # Backend language
```

## Frontend

```
vanilla-js          # No framework (lightweight)
html5               # Structure
css3                # Styling (CSS variables for theming)
```

## Backend (Rust)

```
serde               # Serialization
serde_json          # JSON handling
tokio               # Async runtime
uuid                # Unique IDs
```

## Audio (Optional)

```
cpal                # Cross-platform audio input
rodio               # Audio playback
hound               # WAV encoding
ogg                 # OGG Vorbis encoding
cidre               # macOS Core Audio (Process Taps)
```

## AI/ML (Optional)

```
whisper-rs          # Local Whisper transcription
reqwest             # HTTP client for cloud APIs
```

## Dev Tools

```
cargo               # Rust package manager
tauri-cli           # Build & dev server
```

## Tauri Config

```json
// src-tauri/tauri.conf.json
{
  "productName": "MyApp",
  "version": "0.1.0",
  "identifier": "com.myapp.desktop",
  "build": {
    "frontendDist": "../src"
  },
  "app": {
    "windows": [{
      "title": "My App",
      "width": 1200,
      "height": 800,
      "minWidth": 800,
      "minHeight": 600
    }]
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/icon.icns", "icons/icon.ico", "icons/icon.png"]
  }
}
```

## Quick Start

```bash
# Install Tauri CLI
cargo install tauri-cli

# Create new project
cargo tauri init

# Development
cargo tauri dev

# Build
cargo tauri build
```

## Cargo.toml Template

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2024"

[dependencies]
tauri = { version = "*", features = ["macos-private-api"] }
serde = { version = "*", features = ["derive"] }
serde_json = "*"
tokio = { version = "*", features = ["full"] }
uuid = { version = "*", features = ["v4", "serde"] }

[build-dependencies]
tauri-build = { version = "*", features = [] }
```
