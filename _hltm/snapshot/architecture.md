# Architecture

## Components

- **audio**: Recording lifecycle (start/stop/pause/resume), coordinates mic + system capture. Entry point: `src-tauri/src/audio.rs`.
- **mic_audio**: Microphone capture via cpal, feeds shared buffer. Entry point: `src-tauri/src/mic_audio.rs`.
- **system_audio**: System audio capture via Core Audio Process Taps (cidre). Entry point: `src-tauri/src/system_audio.rs`.
- **audio_processing**: Real-time mixing of mic + system, EBU R128 normalization, OGG Vorbis encoding, shared ring buffers, transcription buffer tap. Entry point: `src-tauri/src/audio_processing/mod.rs`.
- **transcription**: Post-recording transcription — local Whisper and cloud APIs. Entry point: `src-tauri/src/transcription.rs`.
- **realtime_transcription**: Live transcription during recording — local Whisper sliding window and OpenAI Realtime API WebSocket. Entry point: `src-tauri/src/realtime_transcription.rs`.
- **cloud_ai**: Cloud AI provider clients (OpenAI, Google, Anthropic) and model list fetching. Entry point: `src-tauri/src/cloud_ai/mod.rs`.
- **local_llm**: Local model management — download, delete, freshness check for GGUF and Ollama models. Entry point: `src-tauri/src/local_llm.rs`.
- **config**: App settings persistence — API keys, provider config, transcription settings. Stored as JSON. Entry point: `src-tauri/src/config.rs`.
- **storage**: Recording metadata and filesystem layout. Each recording = directory with audio files + metadata JSON. Entry point: `src-tauri/src/storage.rs`.
- **pipelines**: Pipeline definition model — steps, types, validation. Entry point: `src-tauri/src/pipelines.rs`.
- **pipeline_engine**: Sequential pipeline step execution with progress events. Entry point: `src-tauri/src/pipeline_engine.rs`.
- **connectors**: Built-in step connectors — LLM, MCP (Streamable HTTP), Save, Webhook, Slack, Notion, Linear. Entry point: `src-tauri/src/connectors/mod.rs`.
- **integrations**: Integration configuration CRUD — Slack, Notion, Linear, Webhook, Save paths. Entry point: `src-tauri/src/integrations/mod.rs`.
- **Frontend**: Vanilla JS SPA — main.js (core app), pipeline-builder.js (pipeline editor), integrations-settings.js (settings/models UI), viewManager.js (navigation). No framework, no bundler.

## Data Flow

### Recording Flow
1. User starts recording → `audio.rs` spawns mic_audio + system_audio capture threads
2. Both feed audio into shared ring buffers (`MIC_BUFFER`, `SYSTEM_BUFFER`)
3. `realtime_mixer` reads both buffers, mixes to stereo, normalizes (EBU R128)
4. Mixed audio → OGG Vorbis encoding → file on disk
5. Mixed audio → downsampled to 16kHz mono → `TRANSCRIPTION_BUFFER` (for real-time transcription)
6. User stops recording → finalization writes metadata JSON

### Transcription Flow
1. Post-recording: `transcription.rs` reads OGG file, decodes, feeds Whisper or cloud API
2. Real-time: `realtime_transcription.rs` reads from `TRANSCRIPTION_BUFFER` during recording, runs sliding window Whisper or streams to OpenAI Realtime API WebSocket

### Pipeline Flow
1. User assigns pipelines to recording (during or after recording)
2. On recording completion, `pipeline_engine.rs` executes assigned pipelines sequentially
3. Each step dispatches to a connector (LLM, MCP, Save, Webhook, etc.)
4. Step outputs chain: output of step N becomes input of step N+1
5. Progress events emitted to frontend via Tauri events

## Key Interfaces

### Tauri Commands (Rust → Frontend)
- All backend functionality exposed via `#[tauri::command]` functions registered in `lib.rs`
- Frontend calls via `window.__TAURI__.core.invoke("command_name", { args })`

### Tauri Events (Backend → Frontend)
- `transcription_progress`: transcription status updates
- `llm_download_progress`: model download progress `{model_id, downloaded, total, percent}`
- `realtime_transcript_delta`: live transcription text `{text, is_final}`
- `pipeline_progress`: pipeline step execution status
- `recording_complete`: recording finalization done

### Pipeline Connector Interface
- Each connector implements step execution: receives step config + input text, returns output text
- Connectors: `llm.rs`, `mcp.rs`, `save.rs`, `webhook.rs`, `slack.rs`, `notion.rs`, `linear.rs`

## State

- **Recording audio**: shared ring buffers in `audio_processing/shared_buffer.rs` (lazy_static globals: `MIC_BUFFER`, `SYSTEM_BUFFER`, `TRANSCRIPTION_BUFFER`)
- **Recording state**: `AudioState` managed by Tauri (is_recording, recorders, mixer handles)
- **App settings**: JSON file in app data dir, loaded into `Arc<Mutex<AppSettings>>` managed state
- **Recording metadata**: per-recording JSON files in `~/Library/Application Support/com.skopanev.nbp/recordings/<id>/metadata.json`
- **Audio files**: OGG files in same recording directory (mic.ogg, system.ogg, mix.ogg)
- **Pipeline definitions**: JSON files in app data dir
- **Prompt templates**: YAML files in app data dir
- **Credentials**: macOS Keychain via security-framework (production), `.dev-credentials.json` (dev mode)
