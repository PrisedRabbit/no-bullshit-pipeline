# CLAUDE.md - Project Guidelines

## Git Commits
- Never include "Co-Authored-By" lines in commit messages
- Never mention amount of lines changed, only functional changes
- Keep commit messages concise and descriptive

## Tech Stack
- Tauri 2 (Rust backend + Vanilla JS frontend)
- bun for package management (not npm)
- No bundler - static files served directly

## Audio
- OGG Vorbis encoding via vorbis_rs
- Real-time mixing via shared buffers
- In-app playback via rodio
- System audio capture via Core Audio Process Taps (cidre)
- Mic capture via cpal

## Build
```bash
cargo tauri dev      # development
cargo tauri build    # production
```
