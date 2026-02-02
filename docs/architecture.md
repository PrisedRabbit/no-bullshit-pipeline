# NBP Architecture

> Privacy-first audio capture and processing for macOS

## Executive Summary

NBP (No Bullshit Pipeline) is a Tauri 2 desktop application that captures microphone and system audio simultaneously, processes it with professional-grade normalization, and provides local AI transcription via Whisper. The application follows a strict local-first, privacy-by-default architecture.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        NBP Application                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │                    Frontend (src/)                          │     │
│  │  ┌──────────┐  ┌──────────┐  ┌────────────────────────┐   │     │
│  │  │index.html│  │styles.css│  │ main.js + viewManager  │   │     │
│  │  └──────────┘  └──────────┘  └────────────────────────┘   │     │
│  │                                                            │     │
│  │  Views: Recordings List | Detail | Settings | Onboarding  │     │
│  │  State: CSS classes (detail-open, settings-open, etc.)    │     │
│  └────────────────────────────┬───────────────────────────────┘     │
│                               │                                      │
│                    Tauri IPC (invoke/emit)                          │
│                               │                                      │
│  ┌────────────────────────────┴───────────────────────────────┐     │
│  │                    Backend (src-tauri/)                     │     │
│  │                                                             │     │
│  │  ┌─────────────────────────────────────────────────────┐   │     │
│  │  │              Tauri Commands (23 total)               │   │     │
│  │  │  Recording: start/stop/pause/resume                  │   │     │
│  │  │  Storage: list/read/update/delete recordings         │   │     │
│  │  │  Transcription: models/download/transcribe           │   │     │
│  │  │  Config: load/save settings                          │   │     │
│  │  │  Permissions: check/request mic/system audio         │   │     │
│  │  └─────────────────────────────────────────────────────┘   │     │
│  │                                                             │     │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐    │     │
│  │  │ AudioState  │  │   Storage   │  │  Transcription  │    │     │
│  │  │  (Managed)  │  │   Manager   │  │    (Whisper)    │    │     │
│  │  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘    │     │
│  │         │                │                   │             │     │
│  │  ┌──────┴────────────────┴───────────────────┴──────┐     │     │
│  │  │            Audio Processing Pipeline              │     │     │
│  │  │                                                   │     │     │
│  │  │  ┌─────────┐   ┌─────────┐   ┌─────────────┐    │     │     │
│  │  │  │   Mic   │   │ System  │   │  Realtime   │    │     │     │
│  │  │  │ Capture │   │ Capture │   │   Mixer     │    │     │     │
│  │  │  │ (cpal)  │   │ (cidre) │   │             │    │     │     │
│  │  │  └────┬────┘   └────┬────┘   └──────┬──────┘    │     │     │
│  │  │       │             │               │           │     │     │
│  │  │  ┌────┴─────────────┴───────────────┴────┐     │     │     │
│  │  │  │     EBU R128 Normalizer (-23 LUFS)    │     │     │     │
│  │  │  │     + True Peak Limiter (-1 dBTP)     │     │     │     │
│  │  │  └───────────────────┬───────────────────┘     │     │     │
│  │  │                      │                          │     │     │
│  │  │  ┌───────────────────┴───────────────────┐     │     │     │
│  │  │  │      OGG Vorbis Encoder (VBR ~128k)   │     │     │     │
│  │  │  └───────────────────────────────────────┘     │     │     │
│  │  └───────────────────────────────────────────────┘     │     │
│  └─────────────────────────────────────────────────────────┘     │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │   ~/nbp-data/         │
                    │   (File System)       │
                    │                       │
                    │   {uuid}/             │
                    │   ├── raw_mic.ogg     │
                    │   ├── raw_system.ogg  │
                    │   ├── audio_mix.ogg   │
                    │   ├── metadata.json   │
                    │   └── transcript.md   │
                    └───────────────────────┘
```

## Core Modules

### Audio Capture

| Module | File | Technology | Purpose |
|--------|------|------------|---------|
| Mic Audio | `mic_audio.rs` | cpal | Cross-platform microphone capture |
| System Audio | `system_audio.rs` | cidre (Core Audio) | macOS system audio loopback via Process Taps |
| Device Enumeration | `devices.rs` | cpal | Input device listing and selection |

**Key Design Decisions:**
- Uses ring buffers (`ringbuf`) for lock-free audio streaming
- Continuous timeline tracking ensures accurate duration
- Silence padding fills gaps to maintain sync
- Device selection by name (cpal device IDs are unstable)
- Automatic resampling for non-48kHz devices (Bluetooth at 16kHz)

### Audio Processing

| Module | File | Purpose |
|--------|------|---------|
| Normalizer | `normalizer.rs` | EBU R128 loudness normalization to -23 LUFS |
| Mixer | `mixer.rs` | Post-recording mix with resampling |
| Realtime Mixer | `realtime_mixer.rs` | Live mixing during recording |

**Processing Pipeline:**
1. **Capture** → Raw audio from devices
2. **Normalize** → Apply gain + true peak limiting
3. **Encode** → OGG Vorbis (quality 0.4 VBR)
4. **Mix** → Combine mic + system tracks

### Storage

**Pattern:** File-based, no database

```
~/nbp-data/
├── {uuid}/                    # Recording session
│   ├── raw_mic.ogg           # Immutable source
│   ├── raw_system.ogg        # Immutable source
│   ├── audio_mix.ogg         # Derived (regenerable)
│   ├── metadata.json         # Recording metadata
│   └── transcript.md         # Derived (regenerable)
└── projects.json             # Saved tag filters
```

**Invariants:**
1. One recording = one directory
2. Raw files are immutable after recording stops
3. Derived files can be deleted and regenerated
4. `metadata.json` is the source of truth

### Transcription

| Feature | Implementation |
|---------|----------------|
| Engine | whisper-rs with Metal acceleration |
| Models | Tiny (74MB) → Large (2.9GB) |
| Storage | `~/.nbp/models/` |
| Output | Markdown transcript |

**Pipeline:**
1. Convert OGG → 16kHz mono WAV
2. Load Whisper model (Metal GPU)
3. Run inference with greedy decoding
4. Save transcript as `transcript.md`

## State Management

### Backend State (Rust)

```rust
// Tauri managed state
AudioState {
    is_recording: Mutex<bool>,
    mic_recorder: Mutex<Option<MicAudioRecorder>>,
    system_recorder: Mutex<Option<SystemAudioRecorder>>,
    realtime_mixer: Mutex<Option<RealtimeMixer>>,
    current_session: Mutex<Option<RecordingMetadata>>,
    start_timestamp: Mutex<Option<SystemTime>>,
}

PermissionsStateCache(Arc<Mutex<PermissionsState>>)
```

### Frontend State (JavaScript)

```javascript
// Module-level state
let isRecording = false;
let allRecordings = [];
let selectedTags = [];
let selectedRecordingId = null;
let currentRecordingTags = [];
let permissions = { mic: false, system_audio: false };
let appSettings = null;
```

### View State (CSS)

```css
body.detail-open           /* Detail view visible */
body.settings-open         /* Settings view visible */
body.is-recording-active   /* Recording in progress */
body.deep-blue             /* Deep Blue theme applied */
body.deep-obsidian         /* Deep Obsidian theme applied */
```

## IPC Commands

### Recording
- `start_recording(tags: Vec<String>)` → `RecordingMetadata`
- `stop_recording()` → `()`
- `pause_recording()` → `()`
- `resume_recording()` → `()`

### Storage
- `list_recordings()` → `Vec<RecordingMetadata>`
- `read_metadata(recording_id)` → `RecordingMetadata`
- `update_tags(recording_id, tags)` → `()`
- `update_title(recording_id, title)` → `()`
- `delete_recording(recording_id)` → `()`
- `list_projects()` → `Vec<Project>`
- `save_projects(projects)` → `()`

### Transcription
- `get_whisper_models_info()` → `Vec<ModelInfo>`
- `download_whisper_model(size)` → `String`
- `delete_whisper_model(size)` → `()`
- `transcribe_recording(recording_id)` → `String`
- `get_transcript(recording_id)` → `Option<String>`

### Configuration
- `load_settings()` → `AppSettings`
- `save_settings(settings)` → `()`

### Permissions
- `check_permissions(onboarding_completed)` → `PermissionsState`
- `request_mic_permission()` → `bool`
- `request_system_audio_permission()` → `bool`
- `open_privacy_settings(pane)` → `()`

### Audio Devices
- `get_input_devices()` → `Vec<AudioDeviceInfo>`
- `get_audio_level()` → `f32`

## Configuration

### App Settings (`~/.nbp/settings.json`)

```json
{
  "storage_path": "~/nbp-data",
  "auto_discard_seconds": 3,
  "theme": "neon-purple",
  "onboarding_completed": true,
  "selected_microphone": null,
  "transcription": {
    "enabled": false,
    "provider": "LocalWhisper",
    "whisper_model": "Base",
    "api_key": null
  }
}
```

### Tauri Config (`src-tauri/tauri.conf.json`)

- Product: `nbp`
- Version: `0.4.0`
- Identifier: `com.skopanev.nbp`
- Min macOS: `13.0`
- Plugins: `dialog`, `opener`

## Security & Privacy

| Principle | Implementation |
|-----------|----------------|
| Local-first | All data in `~/nbp-data/`, no cloud by default |
| No telemetry | Zero tracking, zero analytics |
| Explicit consent | API keys only when user provides them |
| Permission checks | macOS permissions verified at startup |

## Future Extensibility

**Platform Isolation:**
- `cidre` (Core Audio) is macOS-specific
- Other modules use cross-platform crates
- Potential for Linux (PulseAudio) or Windows (WASAPI) support

**API Integration Points:**
- `TranscriptionProvider` enum supports `LocalWhisper`, `OpenAI`, `Google`
- API key storage ready in settings
