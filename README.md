# No Bullshit Pipeline (NBP)

Total Audio Capture → Structured Data. **Privacy-first. No bullshit.**

## What Is This?

NBP is a high-performance, local-first tool that captures **everything** happening on your Mac's audio system — from your own voice to system sounds, Zoom calls, FaceTime meetings, and Slack huddles. It transforms raw audio into clean, structured data — transcripts, summaries, and pipeline outputs — without ever sending your sensitive information to the cloud (unless you explicitly choose to use an API).

**Core Principles:**

- 🔒 **Privacy First**: Everything runs on your Mac. Zero network calls by default.
- 🎙️ **Absolute Capture**: High-quality recording of both Microphone and System Audio (calls, videos, notifications).
- 📁 **Local Ownership**: All data reside in `~/nbp-data/` in universal formats (OGG, JSON, Markdown).
- 🔄 **Derivable Content**: Raw audio is immutable. All transcripts and summaries are derived artifacts.
- 🎚️ **Professional Signal**: Broadcast-grade normalization (EBU R128) for consistent levels across all sources.

**What It Does:**

- **Capture**: Record Mic + System Audio simultaneously (Meetings, Podcasts, Brainstorms).
- **Process**: Auto-mix and normalize tracks to professional standards.
- **Transcribe**: On-device Whisper (Metal GPU) or OpenAI Whisper API — your choice.
- **Summarize**: Generate structured summaries via OpenAI, Google Gemini, or Anthropic Claude.
- **Automate**: Chain steps into pipelines — transcribe → summarize → send to Slack → save to disk.
- **Organize**: Tag-based filtering, projects, and instant access to raw assets.

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
bun dev

# Build the release binary
bun build:release
```

The app will open automatically. Recordings are saved to `~/nbp-data/`.

## Features

### Audio

- **Total Audio Loopback**: System audio capture via Core Audio Process Taps (Zoom, FaceTime, browser, etc.) alongside your microphone via cpal.
- **Real-time Mixing**: Shared ring buffers with adaptive gain control and soft clipping.
- **EBU R128 Normalization**: Per-track loudness normalization (-23 LUFS, -1 dBTP true-peak limiter).
- **OGG Vorbis Encoding**: VBR quality ~128kbps, 48kHz stereo via vorbis_rs.
- **In-app Playback**: Listen to recordings directly in the app.
- **Waveform Visualization**: Real-time visual feedback during recording.
- **Mic Selection**: Choose from available input devices.
- **Auto-discard**: Skip recordings shorter than a configurable threshold (default 3s).
- **Mix-only Mode**: Save only the combined mix by default, keeping storage lean.

### Transcription

- **Local Whisper**: On-device inference via whisper-rs with Metal GPU acceleration. Downloadable models from Tiny (74 MB) to Large (2.95 GB).
- **OpenAI Whisper API**: Cloud-based transcription as an alternative.
- **AI Summarization**: Generate summaries via OpenAI GPT-4o, Google Gemini, or Anthropic Claude.
- **Prompt Templates**: Reusable prompt templates (built-in: meeting-notes, brainstorm, journal) with variable substitution.
- **Export**: Save transcripts as Markdown with YAML frontmatter.

### Pipelines

- **Multi-step Automation**: Define named pipelines with sequential steps.
- **Connectors**: LLM (OpenAI/Google/Anthropic), Save (to disk), Webhook (HTTP POST/PUT/PATCH), Slack.
- **Progress Tracking**: Per-step status (Waiting/Running/Done/Partial) with real-time progress events.
- **Pipeline Output**: Step results stored as Markdown with YAML frontmatter per recording.

### Slack Integration

- **Bot Token Auth**: Add/test Slack bot tokens, stored securely in macOS Keychain.
- **Channel Delivery**: Send pipeline outputs to Slack channels or DMs.

### UI

- **Neon Themes**: High-contrast interface with multiple themes (neon-purple, deep-blue, light).
- **Dynamic Header**: Context-aware UI that transforms based on your current task.
- **Smart Tagging**: Rank-based tag suggestions for lightning-fast organization.
- **Confirmation Modals**: Safety prompts for destructive actions.
- **Finder Integration**: One click to reveal raw files.

## Storage Structure

Recordings are stored in `~/nbp-data/` (configurable), config in `~/.nbp/`:

```
~/nbp-data/
├── {uuid}/
│   ├── metadata.json                  # Title, tags, timestamps, pipeline states
│   ├── audio_mix.ogg                  # Combined master mix (always present)
│   ├── raw_mic.ogg                    # Mic track (only if mix-only mode is off)
│   ├── raw_system.ogg                 # System track (only if mix-only mode is off)
│   ├── transcript.json                # Source-of-truth transcript
│   ├── summary.md                     # AI-generated summary
│   └── pipelines/
│       └── {pipeline_name}/
│           └── {step_name}.md         # Step output with YAML frontmatter

~/.nbp/
├── settings.json                      # App settings
├── pipelines.json                     # Pipeline definitions
├── prompt-templates.json              # Reusable prompt templates
├── projects.json                      # Tag-based projects
└── models/
    └── ggml-*.bin                     # Downloaded Whisper models
```

## Privacy & Security

- **Air-Gapped Ready**: Core recording and local transcription work without internet.
- **No Telemetry**: We don't track what you record, how long you record, or who you are.
- **Secure Storage**: API keys and Slack tokens stored in macOS Keychain. Settings file is `chmod 600`.
- **Encryption**: Bring your own encryption or let macOS handle it via FileVault.
- **Your Keys, Your Choice**: Use local Whisper for 100% on-device AI, or plug in your cloud keys.

## Unsigned run on macOS

Run `xattr -cr /Applications/nbp.app` to remove the Gatekeeper signature.

## License

Private project - all rights reserved.

**USE AS-IS. NO WARRANTY.**
