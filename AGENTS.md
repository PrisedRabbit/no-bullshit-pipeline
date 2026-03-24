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

<!-- BEGIN TK INTEGRATION -->
## Issue Tracking with tk

**IMPORTANT**: This project uses **tk** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other trackers.

**Note:** `tk` stores tickets as markdown files in `.tickets/` and does not run git commands for you.

### Why tk?

- Dependency-aware: track blockers and relationships between tickets
- Git-friendly: tickets are plain files in `.tickets/`
- Agent-friendly: fast CLI with ready/blocked views and JSON querying
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**

```bash
tk ready
```

**Create new tickets:**

```bash
tk create "Issue title" -d "Detailed context" -t bug|feature|task|epic|chore -p 0-4
tk create "Issue title" -d "What this issue is about" -p 1
```

**Claim and update:**

```bash
tk start nw-42
tk status nw-42 in_progress
```

**Complete work:**

```bash
tk close nw-42
```

**Dependency operations:**

```bash
tk dep <id> <dep-id>
tk blocked
tk dep tree --full <id>
```

**JSON output for automation:**

```bash
tk query
tk query '.[] | select(.status=="open")'
```

### Ticket Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `tk ready` shows unblocked tickets
2. **Claim your task**: `tk start <id>`
3. **Work on it**: implement, test, document
4. **Discover new work?** Create linked ticket:
   - `tk create "Found bug" -d "Details about what was found" -t bug -p 1`
   - If parent depends on new work, add dependency with `tk dep <parent-id> <new-id>`
5. **Complete**: `tk close <id>`
6. **Persist tracker state in git**:
   - stage `.tickets/` with related code changes
   - commit/push only when explicitly requested

### Important Rules

- ✅ Use tk for ALL task tracking
- ✅ Link discovered work to parent tickets via dependencies
- ✅ Check `tk ready` before asking "what should I work on?"
- ✅ Use `tk query` when machine-readable output is needed
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems
- ❌ Do NOT assume tk auto-commits anything

For more details, run `tk --help`.

<!-- END TK INTEGRATION -->


## Build & Test

_Add your build and test commands here_

```bash
# Example:
# bun install
# bun test
```

## Architecture Overview

_Add a brief overview of your project architecture_

## Conventions & Patterns

_Add your project-specific conventions here_

## Landing the Plane (Session Completion)

**When ending a work session**, complete all steps below.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Sync issue tracker state and git**
   ```bash
   git pull --rebase
   git add .tickets/
   git commit -m "sync tickets"
   git push
   git status
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All required changes committed and pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Do not leave tracker updates uncommitted in `.tickets/`
- Do not stop in the middle of sync/commit/push flow
- If push fails, resolve and retry
