# Project Instructions for AI Agents

This file provides instructions and context for AI coding agents working on this project.

## Git Rules

- **NEVER** run `git commit` or `git push` unless the user explicitly says to commit or push
- Do not auto-commit, do not auto-push after fixes — wait for explicit instruction

## UI/UX Skill Rule

When the user asks for design work (or mentions design/UI/UX improvements), always use the `ui-ux-pro-max` skill.

<!-- BEGIN BEADS INTEGRATION -->
## Issue Tracking with br (beads_rust)

**IMPORTANT**: This project uses **br (beads_rust)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

**Note:** `br` is non-invasive and never executes git commands. After issue-tracking changes, run `br sync --flush-only`, then commit `.beads/` manually.

### Why br?

- Dependency-aware: track blockers and relationships between issues
- Git-friendly: export JSONL to `.beads/issues.jsonl`
- Agent-optimized: JSON output, ready work detection, clean CLI for automation
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**

```bash
br ready --json
```

**Create new issues:**

```bash
br create "Issue title" --description "Detailed context" -t bug|feature|task -p 0-4 --json
br create "Issue title" --description "What this issue is about" -p 1 --json
```

**Claim and update:**

```bash
br update nbp-42 --status in_progress --json
br update nbp-42 --priority 1 --json
```

**Complete work:**

```bash
br close nbp-42 --reason "Completed" --json
```

**Sync tracker state to JSONL + git:**

```bash
br sync --flush-only
git add .beads/
git commit -m "sync beads"
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)
- `decision` - Product/architecture decision records

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `br ready` shows unblocked issues
2. **Claim your task**: `br update <id> --status in_progress`
3. **Work on it**: implement, test, document
4. **Discover new work?** Create linked issue:
   - `br create "Found bug" --description "Details about what was found" -p 1 --json`
   - Add dependency with `br dep add <new-id> <parent-id> -t discovered-from`
5. **Complete**: `br close <id> --reason "Done"`
6. **Flush tracker state**:
   - `br sync --flush-only`
   - `git add .beads/ && git commit -m "sync beads"`

### Important Rules

- ✅ Use br for ALL task tracking
- ✅ Always use `--json` for programmatic use
- ✅ Link discovered work to parent issues via dependencies
- ✅ Check `br ready` before asking "what should I work on?"
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems
- ❌ Do NOT assume br auto-commits anything

For more details, see README.md.

<!-- END BEADS INTEGRATION -->


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
   br sync --flush-only
   git add .beads/
   git commit -m "sync beads"
   git push
   git status
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All required changes committed and pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Do not leave tracker updates uncommitted in `.beads/`
- Do not stop in the middle of sync/commit/push flow
- If push fails, resolve and retry
