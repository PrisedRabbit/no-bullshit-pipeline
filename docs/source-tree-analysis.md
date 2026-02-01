# NBP Source Tree Analysis

## Directory Structure

```
nbp/
├── src/                           # Frontend (Vanilla JS)
│   ├── index.html                 # Single-page app structure
│   ├── main.js                    # Application logic (~1049 lines)
│   ├── viewManager.js             # View state module
│   └── styles.css                 # Styling + themes (~1255 lines)
│
├── src-tauri/                     # Backend (Rust)
│   ├── Cargo.toml                 # Rust dependencies
│   ├── tauri.conf.json            # Tauri configuration
│   ├── build.rs                   # Build script (generated)
│   ├── Info.plist                 # macOS app metadata
│   ├── entitlements.plist         # macOS entitlements
│   ├── icons/                     # App icons (18 files)
│   │   ├── icon.icns              # macOS icon
│   │   ├── icon.ico               # Windows icon
│   │   └── *.png                  # Various sizes
│   └── src/
│       ├── main.rs                # Entry point (7 lines)
│       ├── lib.rs                 # Tauri app builder (60 lines)
│       ├── audio.rs               # Recording orchestration (272 lines)
│       ├── mic_audio.rs           # Microphone capture (344 lines)
│       ├── system_audio.rs        # System audio capture (382 lines)
│       ├── storage.rs             # File management (252 lines)
│       ├── config.rs              # Settings persistence (102 lines)
│       ├── permissions.rs         # macOS permissions (170 lines)
│       ├── transcription.rs       # Whisper integration (281 lines)
│       └── audio_processing/
│           ├── mod.rs             # Module exports (8 lines)
│           ├── normalizer.rs      # EBU R128 normalization (116 lines)
│           ├── mixer.rs           # Post-recording mixer (225 lines)
│           └── realtime_mixer.rs  # Live mixer (235 lines)
│
├── builds/                        # Release artifacts (.dmg files)
├── node_modules/                  # JS dependencies (Tauri CLI only)
│
├── .agent/                        # AI assistant configuration
│   └── rules/
│       └── general.md             # Project principles for AI
│
├── .vscode/                       # Editor configuration
├── .gitignore                     # Git ignore rules
├── package.json                   # JS package manifest
├── bun.lock                       # Bun lockfile
├── build.sh                       # Release build script
│
├── README.md                      # Project overview
├── STORAGE.md                     # Storage architecture docs
├── todo.md                        # Development roadmap
└── npb.code-workspace             # VS Code workspace
```

## Critical Directories

### `src/` - Frontend

| File | Lines | Purpose |
|------|-------|---------|
| `index.html` | 542 | Complete UI structure: app bar, sidebar, recordings list, detail view, settings, modals, onboarding |
| `main.js` | 1049 | All application logic: recording control, state management, Tauri IPC, UI rendering |
| `viewManager.js` | 22 | Simple view state switcher using CSS classes |
| `styles.css` | 1255 | Complete styling with two themes (Neon Purple, Deep Obsidian) |

**Frontend Architecture:**
- No build step, no framework
- Single-page app with CSS-based view switching
- Global window functions for onclick handlers
- Tauri invoke for all backend communication

### `src-tauri/src/` - Backend

| Module | File | Lines | Description |
|--------|------|-------|-------------|
| **Entry** | `main.rs` | 7 | Minimal entry point calling `nbp_lib::run()` |
| **App** | `lib.rs` | 60 | Tauri builder with plugins and command handlers |
| **Audio Core** | `audio.rs` | 272 | `AudioState` struct, start/stop/pause/resume recording |
| **Mic** | `mic_audio.rs` | 344 | `MicAudioRecorder` using cpal for cross-platform capture |
| **System** | `system_audio.rs` | 382 | `SystemAudioRecorder` using cidre for Core Audio Process Taps |
| **Processing** | `audio_processing/` | 584 | Normalization, mixing, encoding pipeline |
| **Storage** | `storage.rs` | 252 | File-based recording management, metadata CRUD |
| **Config** | `config.rs` | 102 | Settings persistence to `~/.nbp/settings.json` |
| **Permissions** | `permissions.rs` | 170 | macOS permission checks and requests |
| **Transcription** | `transcription.rs` | 281 | Whisper model management and inference |

### `src-tauri/src/audio_processing/` - Audio Pipeline

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 8 | Module exports |
| `normalizer.rs` | 116 | EBU R128 loudness normalization with true peak limiting |
| `mixer.rs` | 225 | Post-recording audio mixing with linear resampling |
| `realtime_mixer.rs` | 235 | Live mixing during recording from growing files |

## Entry Points

### Application Entry
- **Rust:** `src-tauri/src/main.rs:5` → `nbp_lib::run()`
- **Frontend:** `src/index.html` → `src/main.js:1029` → `init()`

### Key Functions

**Recording Flow:**
```
main.js:toggleRecording()
    → invoke("start_recording")
        → audio.rs:start_recording()
            → mic_audio.rs:start_mic_capture()
            → system_audio.rs:start_system_capture()
            → RealtimeMixer::new()
```

**Transcription Flow:**
```
main.js:process-btn click
    → invoke("transcribe_recording")
        → transcription.rs:transcribe_recording()
            → convert_ogg_to_wav()
            → run_whisper_transcription()
```

## File Statistics

| Category | Files | Lines of Code |
|----------|-------|---------------|
| Rust Backend | 13 | ~2,450 |
| JavaScript | 2 | ~1,070 |
| HTML | 1 | 542 |
| CSS | 1 | 1,255 |
| **Total** | **17** | **~5,320** |

## Dependencies

### Rust (Cargo.toml)

| Category | Crates |
|----------|--------|
| Framework | tauri 2, tauri-plugin-dialog, tauri-plugin-opener |
| Audio I/O | cpal, cidre (Core Audio), hound |
| Audio Processing | ebur128, rubato, ringbuf |
| Encoding | vorbis_rs, lewton |
| AI/ML | whisper-rs (with Metal) |
| Async | tokio, futures-util |
| Network | reqwest |
| Data | serde, serde_json, chrono, uuid |
| Error Handling | anyhow, log |

### JavaScript (package.json)

| Package | Purpose |
|---------|---------|
| @tauri-apps/cli | Tauri CLI for development and builds |

## Configuration Files

| File | Purpose |
|------|---------|
| `src-tauri/tauri.conf.json` | Tauri app configuration, window settings, bundling |
| `src-tauri/Cargo.toml` | Rust dependencies and crate metadata |
| `src-tauri/Info.plist` | macOS application metadata |
| `src-tauri/entitlements.plist` | macOS entitlements for audio capture |
| `package.json` | Node/Bun package configuration |
| `.gitignore` | Git ignore patterns |
