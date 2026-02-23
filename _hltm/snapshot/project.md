# Project

Privacy-first macOS desktop app for audio capture (mic + system), AI transcription, and configurable processing pipelines.

## Stack

- Language: Rust (backend), Vanilla JS (frontend)
- Framework: Tauri 2.10 (Rust backend + webview frontend, no bundler — static files served directly)
- Key libraries:
  - Audio: cpal 0.15 (mic), cidre/Core Audio (system audio), vorbis_rs 0.5 (OGG encoding), rodio 0.21 (playback), ebur128 (normalization), rubato 0.15 (resampling)
  - AI: whisper-rs 0.12 (local transcription, Metal), llama-cpp-2 0.1.135 (local LLM, Metal), reqwest 0.13 (cloud APIs)
  - WebSocket: tokio-tungstenite 0.26 (OpenAI Realtime API)
  - Frontend: marked 17.0.3 (markdown rendering)
- Sidecar: fluidaudio-sidecar (Swift, bundled binary for additional audio processing)
- Package manager: bun (NOT npm/yarn/pnpm)

## Structure

```
nbp/
├── src/                    # Frontend (vanilla JS, HTML, CSS — served directly by Tauri)
│   ├── index.html
│   ├── main.js             # Core app logic, recording UI, settings, pipeline display
│   ├── pipeline-builder.js # Pipeline creation/editing UI
│   ├── integrations-settings.js # Settings for models, integrations
│   ├── viewManager.js      # View/navigation management
│   ├── styles.css
│   ├── assets/             # Icons, SVGs
│   └── vendor/             # Third-party JS
├── src-tauri/              # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── lib.rs          # Tauri app setup, command registration
│       ├── audio.rs        # Recording lifecycle (start/stop/pause/resume)
│       ├── audio_processing/ # Mixing, normalization, shared buffers
│       ├── mic_audio.rs    # Microphone capture (cpal)
│       ├── system_audio.rs # System audio capture (Core Audio Process Taps)
│       ├── transcription.rs # Whisper + cloud transcription
│       ├── realtime_transcription.rs # Live transcription during recording
│       ├── cloud_ai/       # Cloud providers (OpenAI, Google, Anthropic) + model fetching
│       ├── local_llm.rs    # Local model management (download, delete, freshness)
│       ├── config.rs       # App settings, API keys, provider config
│       ├── storage.rs      # Recording metadata, filesystem CRUD
│       ├── pipelines.rs    # Pipeline definition model
│       ├── pipeline_engine.rs # Pipeline step execution engine
│       ├── connectors/     # Pipeline connectors (LLM, MCP, Save, Webhook, Slack, Notion, Linear)
│       ├── integrations/   # Integration config management
│       └── prompt_templates.rs # Reusable prompt template registry
├── fluidaudio-sidecar/     # Swift sidecar for audio processing
├── builds/                 # Production build output
└── .beads/                 # Issue tracker database
```

## Run

```bash
# verify compilation (default)
bun run check

# development (opens a window)
bun run dev

# production build
bun run build:release

# install dependencies
bun install

# update all deps
bun run update-packages
```

## Toolchain

| Tool     | Command                                              |
| -------- | ---------------------------------------------------- |
| check    | `bun run check`                                      |
| dev      | `bun run dev`                                        |
| prod     | `bun run build:release`                              |
| packages | `bun install`                                        |
| update   | `bun run update-packages`                            |

<!-- TBD: no linter/formatter config found (no clippy.toml, rustfmt.toml, eslint, prettier) -->

## External Dependencies

- OpenAI API: transcription (Whisper), processing (GPT), real-time transcription (Realtime API WebSocket)
- Google AI API: transcription and processing
- Anthropic API: processing
- Ollama (localhost:11434): local model inference
- HuggingFace: GGUF model downloads
- Slack API: integration for sending results
- Notion API: integration for syncing data
- Linear API: integration for issue tracking
- macOS Keychain: secure credential storage
