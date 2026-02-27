# CLAUDE.md - Project Guidelines

## General

- **Never** use `npm`, `npx`, `yarn`, `pnpm` - use `bun`, `bunx` for all package operations

## Git Commits & Push

- **NEVER** run `git commit` or `git push` unless the user explicitly says so
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

## File Operations

- NEVER use Bash redirects (`>`, `>>`, heredoc, `cat > file`, `echo > file`)
- Do NOT write files via shell commands
- Use only Write/Edit tools for file creation or modification

## Build

- Run all commands without prompting for user input unless interaction is **absolutely** required
- **NEVER** run `cargo tauri dev` unless the user explicitly asks — use `cargo check` for compilation verification
- Running the app opens a window and interferes with the user's workflow

```bash
cargo check          # verify compilation (default)
cargo tauri dev      # development (only when user asks)
cargo tauri build    # production
```


<!-- pilot:rules -->
# Pilot Loop Rules

You are running inside an automated loop. **STRICT** constraints:

- Do exactly **ONE** step per round, then **EXIT**
- Do **NOT** chain steps — you WILL be restarted with fresh context
- Read state first, do one step, update state, stop
- Emit `<loop:update>` on progress, `<loop:stage>` before each step
- Never wait for user input — decide yourself
<!-- pilot:rules -->
