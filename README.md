# No Bullshit Pipeline (NBP)

Local voice → structured data. **Privacy-first. No bullshit.**

## What Is This?

NBP captures your voice (mic + system audio) and stores it locally on your machine. No cloud. No telemetry. No servers. Your data stays **yours**.

**Core Principles:**

- 🔒 **Privacy First**: Everything runs on your Mac. Zero network calls (unless you explicitly add API keys for optional transcription/processing)
- 📁 **Local Storage**: All recordings saved to `~/nbp-data/` in open formats (OGG, JSON, Markdown)
- 🎯 **No Bullshit**: No databases, no telemetry, no tracking, no dark patterns
- 🔄 **Immutable Sources**: Raw audio files never change. All processing is derived and reproducible
- 🎚️ **Professional Audio**: EBU R128 normalization, stereo mixing, synchronized recording

**What You Get:**

- Mic + system audio capture (both normalized to -23 LUFS)
- Automatic audio mixing into a single file
- Tag-based organization
- File-based storage you can use without this tool
- Future: Transcription, structured data extraction (when you add your own API keys)

## Quick Start

### Prerequisites

- **Rust** (latest stable)
- **Bun** (package manager)
- **macOS** (for audio capture)

### Development

```bash
# Install dependencies
bun install

# Run dev server
bun run tauri dev
```

The app will open automatically. Recordings are saved to `~/nbp-data/`.

### Build

```bash
bun run tauri build
```

## Features (v0.1)

- ✅ **Dual Audio Recording**: Capture microphone + system audio simultaneously
- ✅ **Professional Normalization**: EBU R128 broadcast standard (-23 LUFS)
- ✅ **Auto-Mixing**: Mic + system combined into single stereo mix
- ✅ **Synchronized Audio**: Files guaranteed same length for easy processing
- ✅ **UUID-based Storage**: Each recording in `~/nbp-data/{uuid}/`
- ✅ **Tag Management**: Add/remove tags with Enter key and × button
- ✅ **Tag Filtering**: Gmail-style tag filters with AND logic
- ✅ **Detail View**: Full-width view with editable title and tags
- ✅ **Native Dialogs**: macOS system dialogs for delete confirmation
- ✅ **Open in Finder**: Direct access to recording folders

## Storage

Recordings are stored in `~/nbp-data/` with this structure:

```
~/nbp-data/
├── {uuid}/
│   ├── raw_mic.ogg          # normalized microphone (stereo)
│   ├── raw_system.ogg       # normalized system audio (stereo)
│   ├── audio_mix.ogg        # auto-mixed combination
│   ├── metadata.json        # title, tags, timestamps
│   ├── transcript.md        # (future) derived from audio
│   └── structured.json      # (future) extracted data
```

All files are human-readable and usable without this tool.

See [STORAGE.md](STORAGE.md) for full architecture details.

## Tech Stack

- **Backend**: Rust + Tauri
- **Frontend**: Vanilla JS + CSS
- **Audio**:
  - `cpal` - Cross-platform audio I/O
  - `vorbis_rs` - OGG Vorbis encoding
  - `lewton` - OGG Vorbis decoding
  - `ebur128` - EBU R128 loudness normalization
  - `cidre` - macOS Core Audio Taps (system audio)
- **Storage**: File-based (no database)

## Project Structure

```
nbp/
├── src/                    # Frontend
│   ├── index.html
│   ├── main.js
│   └── styles.css
├── src-tauri/              # Backend
│   ├── src/
│   │   ├── audio.rs              # Recording coordinator
│   │   ├── mic_audio.rs          # Microphone capture + OGG encoding
│   │   ├── system_audio.rs       # System audio capture (Core Audio Taps)
│   │   ├── audio_processing.rs   # Normalization + mixing
│   │   ├── storage.rs            # File operations
│   │   └── lib.rs                # Tauri setup
│   └── Cargo.toml
└── README.md
```

## Privacy & Data Control

- **No network calls**: Everything runs locally
- **No telemetry**: Zero tracking or analytics
- **No cloud**: Your recordings never leave your machine
- **Optional APIs**: If you want transcription, YOU add YOUR keys. We don't provide any services.
- **Open formats**: OGG audio, JSON metadata, Markdown text - use them anywhere
- **Transparent storage**: Files organized in your home directory, accessible without the app

## License

Private project - all rights reserved.
