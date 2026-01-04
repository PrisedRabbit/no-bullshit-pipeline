# No Bullshit Pipeline (NBP)

Total Audio Capture → Structured Data. **Privacy-first. No bullshit.**

## What Is This?

NBP is a high-performance, local-first tool that captures **everything** happening on your Mac's audio system — from your own voice to system sounds, Zoom calls, FaceTime meetings, and Slack huddles. It then transforms this raw audio into clean, structured data without ever sending your sensitive information to the cloud (unless you explicitly choose to use an API).

**Core Principles:**

- 🔒 **Privacy First**: Everything runs on your Mac. Zero network calls by default.
- 🎙️ **Absolute Capture**: High-quality recording of both Microphone and System Audio (calls, videos, notifications).
- 📁 **Local Ownership**: All data reside in `~/nbp-data/` in universal formats (OGG, JSON, Markdown).
- 🔄 **Derivable Content**: Raw audio is immutable. All transcripts and summaries are derived artifacts.
- 🎚️ **Professional Signal**: Broadcast-grade normalization (EBU R128) for consistent levels across all sources.

**What It Does:**

- **Capture**: Record Mic + System Audio simultaneously (Meetings, Podcasts, Brainstorms).
- **Process**: Auto-mix and normalize tracks to professional standards.
- **Synthesize**: (Coming soon) Turn audio into transcripts and structured summaries via local Whisper or your preferred AI APIs (OpenAI, Gemini, Claude).
- **Organize**: Tag-based filtering and instant access to raw assets.

## Quick Start

### Prerequisites

- **macOS** (13.0+)
- **Rust** (latest stable)
- **Bun** (fast package manager)

### Build and Run

```bash
# Install dependencies
bun install

# Run development mode
bun tauri dev

# Build the release binary
bun build:release
```

The app will open automatically. Recordings are saved to `~/nbp-data/`.

## Features (v0.1.0)

- ✅ **Total Audio Loopback**: Record system audio (Zoom/FaceTime) alongside your mic.
- ✅ **Sexy Neon UI**: A high-contrast, premium interface designed for focus.
- ✅ **Dynamic Header**: Intelligent UI that transforms based on your current task.
- ✅ **Smart Tagging**: Rank-based tag suggestions for lightning-fast organization.
- ✅ **Safety First**: Custom neon confirmation modals for destructive actions.
- ✅ **Deep Metadata**: Every recording carries its own DNA (title, tags, timing).
- ✅ **Finder Integration**: One click to see your raw files.

## Storage Structure

All your data is stored in `~/nbp-data/` with a clean, transparent hierarchy:

```
~/nbp-data/
├── {uuid}/
│   ├── raw_mic.ogg          # Normalized microphone track
│   ├── raw_system.ogg       # Normalized system/call track
│   ├── audio_mix.ogg        # The combined "master" mix
│   ├── metadata.json        # Title, tags, timestamps
│   ├── transcript.md        # (v0.2) The text content
│   └── structured.json      # (v0.2) The AI-extracted intelligence
```

## Privacy & Security

- **Air-Gapped Ready**: You can use the core recording features without an internet connection.
- **No Telemetry**: We don't track what you record, how long you record, or who you are.
- **Encryption**: Bring your own encryption or let macOS handle it via FileVault.
- **Your Keys, Your Choice**: Use local Whisper for 100% on-device AI, or plug in your cloud keys for advanced modularity.

## Unsigned run on mac

Run `xattr -cr /Applications/nbp.app` to remove the Gatekeeper signature.

## License

Private project - all rights reserved.

**USE AS-IS. NO WARRANTY.**
