# Project Instructions for AI Agents

This file provides instructions and context for AI coding agents working on this project.

## General

- **Never** use `npm`, `npx`, `yarn`, `pnpm` - use `bun`, `bunx` for all package operations
- **NEVER** use internal task tools (TaskCreate, TaskUpdate, TaskList, TaskGet, TodoWrite) — they are forbidden. Use `tk` instead.

## Git Rules

- **NEVER** run `git commit` or `git push` unless the user explicitly says to commit or push
- Do not auto-commit, do not auto-push after fixes — wait for explicit instruction
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

- All UI/UX, styling, color, theme, and design tasks must use `ui-ux-pro-max` skill

## Documentation

- Always use `Context7` MCP when library/API documentation, code generation, setup or configuration steps are needed without having to explicitly ask.

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

<!-- BEGIN NTK INTEGRATION -->

### Tickets

Use only `ntk` CLI to manage tickets (tasks). Key commands:

- `ntk ls` — list tickets
  - `-s <status>` — filter by status (`open`, `in_progress`, `blocked`, `to_test`, `done`)
  - `-a <initials>` — filter by assignee
  - `-t <tags>` — filter by tags (comma-separated)
- `ntk show <id>` — view ticket details
- `ntk start <id>` — mark ticket as in_progress
- `ntk close <id>` — mark ticket as done
- `ntk next` — pick next ticket to work on (`-a`, `-P` to filter)
- `ntk users` — list assignees
- `ntk create <title>` — create a new ticket:
  - `-p <priority>` — `high`, `med`, `low`
  - `-a <initials>` — assignee (default: me)
  - `-s <status>` — default: `open`
  - `-T <type>` — `feature`, `task`, `bug`, `epic`, `constraint`, `scaffold`, `infra`, `chore`, `core` (default: `task`)
  - `-t <tags>` — comma-separated tags
  - `-d <text>` — description
  - `--deps <tid,tid>` — dependency ticket IDs (blocks `ntk next` until all done)
  - `--due <date>` — due date (`YYYY-MM-DD`)
- `ntk update <id>` — update ticket fields:
  - `-s <status>` — change status (`open`, `in_progress`, `blocked`, `to_test`, `done`)
  - `-p <priority>` — change priority (`high`, `med`, `low`)
  - `-a <initials>` — reassign
  - `-T <type>` — change type (`feature`, `task`, `bug`, `epic`, `constraint`, `scaffold`, `infra`, `chore`, `core`)
  - `-t +tag,-tag` — add/remove tags
  - `-P <project>` — change project
  - `-A <text>` — append text to page body
  - `--deps <tid,tid>` — set dependency ticket IDs
  - `--due <date>` — set due date (`YYYY-MM-DD`)
