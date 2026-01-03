# No Bullshit Pipeline (NBP)

Local voice → structured data. Privacy-first. No bullshit.

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

- ✅ **Audio Recording**: Capture from microphone with pause/resume
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
│   ├── raw.wav              # immutable raw audio
│   ├── metadata.json        # source of truth
│   ├── transcript.md        # (future) derived
│   └── structured.json      # (future) derived
```

See [STORAGE.md](STORAGE.md) for full architecture details.

## Tech Stack

- **Backend**: Rust + Tauri
- **Frontend**: Vanilla JS + CSS
- **Audio**: cpal + hound
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
│   │   ├── audio.rs       # Audio capture
│   │   ├── storage.rs     # File operations
│   │   └── lib.rs         # Tauri setup
│   └── Cargo.toml
└── README.md
```

## Development Notes

- **No network**: Everything runs locally
- **No telemetry**: Zero tracking or analytics
- **Files over databases**: Human-readable formats only
- **Immutable raw audio**: `raw.wav` never changes after recording

## License

Private project - all rights reserved.
