# CLAUDE.md - Project Guidelines

## General

- **Never** use `npm`, `npx`, `yarn`, `pnpm` - use `bun`, `bunx` for all package operations

## Git Commits

- Never include "Co-Authored-By" lines in commit messages
- Never mention amount of lines changed, only functional changes
- Keep commit messages concise and descriptive

## Tech Stack

- Tauri (Rust backend + Vanilla JS frontend)
- bun for package management (not npm)
- No bundler - static files served directly

## Audio

- OGG Vorbis encoding via vorbis_rs
- Real-time mixing via shared buffers
- In-app playback via rodio
- System audio capture via Core Audio Process Taps (cidre)
- Mic capture via cpal

## UI/UX Design

- All UI/UX, styling, color, theme, and design tasks must use `ui-ux-pro-max-skill` skill

## Documentation

- Always use `Context7` MCP when I need library/API documentation, code generation, setup or configuration steps without me having to explicitly ask.

## Build

- Run all commands without prompting for user input unless interaction is **absolutely** required

```bash
cargo tauri dev      # development
cargo tauri build    # production
```
