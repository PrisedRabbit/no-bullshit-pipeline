# NBP Project Overview

## What is NBP?

**NBP (No Bullshit Pipeline)** is a privacy-first audio capture application for macOS that records microphone and system audio simultaneously, processes it with professional-grade normalization, and provides local AI transcription.

**Version:** 0.3.0
**Platform:** macOS 13.0+ (Ventura and later)
**License:** Private - All rights reserved

## Core Value Proposition

> Total Audio Capture → Structured Data. Privacy-first. No bullshit.

NBP captures everything happening on your Mac's audio system — your voice, Zoom calls, FaceTime meetings, system sounds — and transforms it into clean, organized data without ever sending information to the cloud (unless you explicitly choose to).

## Key Features

| Feature | Status | Description |
|---------|--------|-------------|
| **Total Audio Loopback** | ✅ v0.1 | Record system audio (Zoom/FaceTime) alongside mic |
| **Dual Track Recording** | ✅ v0.1 | Separate mic and system tracks |
| **EBU R128 Normalization** | ✅ v0.1 | Professional broadcast-grade loudness |
| **Real-time Mixing** | ✅ v0.3 | Live mix during recording |
| **Local Transcription** | ✅ v0.3 | Whisper models (Tiny → Large) |
| **Neon UI** | ✅ v0.2 | Premium dark interface with themes |
| **Smart Tagging** | ✅ v0.1 | Rank-based tag suggestions |
| **Cloud APIs** | 🔜 v0.3 | OpenAI, Google, Claude integration |
| **Structured Output** | 🔜 v0.3 | Meeting notes, brainstorm templates |

## Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Framework** | Tauri 2.9.5 | Desktop app shell |
| **Backend** | Rust 2024 | Core logic, audio processing |
| **Frontend** | Vanilla JS/HTML/CSS | UI layer |
| **Audio I/O** | cpal + cidre | Cross-platform + Core Audio |
| **Audio Processing** | ebur128, vorbis_rs | Normalization, encoding |
| **AI/ML** | whisper-rs (Metal) | Local transcription |

## Architecture Highlights

### Privacy by Design
- All data stays in `~/nbp-data/`
- Zero telemetry, zero analytics
- Network calls only with explicit API keys
- Air-gapped operation supported

### Professional Audio
- EBU R128 loudness standard (-23 LUFS)
- True peak limiting (-1 dBTP)
- 48kHz stereo output
- OGG Vorbis VBR encoding

### Clean Data Model
- One recording = one directory
- Raw files are immutable
- Derived content is regenerable
- Universal formats (OGG, JSON, MD)

## Quick Stats

| Metric | Value |
|--------|-------|
| Total Source Files | 17 |
| Lines of Code | ~5,320 |
| Rust Modules | 13 |
| Tauri Commands | 23 |
| Dependencies (Rust) | 19 |

## Project Health

| Aspect | Status |
|--------|--------|
| Core Recording | Stable |
| Transcription | Working (local Whisper) |
| Cloud APIs | Planned |
| Tests | Manual only |
| Documentation | Complete |

## Roadmap (v0.3)

**In Progress:**
- [ ] Cloud API integrations (OpenAI, Google, Claude)
- [ ] Structured output templates
- [ ] In-app audio playback
- [ ] Waveform preview

**Maintenance:**
- [ ] Error handling improvements
- [ ] Signed build entitlements

**Non-Goals:**
- Cloud-only storage
- Team collaboration
- Heavy resource usage
- Background services

## Getting Started

1. **Prerequisites:** macOS 13+, Rust, Bun
2. **Install:** `bun install`
3. **Run:** `bun tauri dev`
4. **Grant Permissions:** Microphone + Screen Recording

See [Development Guide](./development-guide.md) for details.

## Architecture

See [Architecture](./architecture.md) for detailed system design.

## Source Code

See [Source Tree Analysis](./source-tree-analysis.md) for directory structure.
