# Snapshot Guide

Rules for `_hltm/snapshot/` files. Project memory that survives context resets.

## Rules

- **Additive** — never delete. Append, update, extend.
- **Facts** — what IS, not what SHOULD BE.
- **WHY > WHAT** — "We use X because Y, rejected Z because W". Not just "We use X".
- **No boilerplate** — skip obvious. Document what a new agent wouldn't guess.
- **Concrete** — file paths, function names, commands. Not "modular architecture".
- **Current** — update when things change.
- Flat structure — all files in `_hltm/snapshot/`, no subdirs.

## Files

### `project.md`

One-liner, tech stack, repo structure (tree, 2 levels), build/test/lint/dev commands, toolchain table, external deps.

No history, no team info, no marketing.

```
# Project
<one-liner>

## Stack
- Language: ...
- Framework: ...
- Key libraries: ...

## Structure
project/
├── src/        # ...
└── config/     # ...

## Run
# build
<command>
# test
<command>

## Toolchain
| Tool   | Command     |
|--------|------------|
| check  | `<command>` |
| dev    | `<command>` |

## External Dependencies
- <service>: <purpose>
```

### `architecture.md`

Components (one line each), data flow, key interfaces, state management.

No individual function details, no code blocks >5 lines.

```
# Architecture

## Components
- **<name>**: <responsibility>. Entry: `<file>`.

## Data Flow
<main path through system>

## Key Interfaces
### <name>
- Input/Output/Contract

## State
- <what>: <where>, <format>
```

### `conventions.md`

Naming patterns, file organization, error handling, codebase-specific patterns, anti-patterns with WHY.

No language-standard conventions, no generic best practices, no linter-enforced rules.

```
# Conventions

## Naming
- Files: `<pattern>`
- Functions: `<pattern>`

## File Organization
- New <type> → `<path>`

## Error Handling
<approach>

## Patterns
### <name>
<when, example reference>

## Anti-Patterns
### <what NOT to do>
Why: <reason>
Instead: <alternative>
```

### `infra.md`

Environments, CI/CD, config locations, monitoring.

No secrets, no one-time setup, no vendor docs.

```
# Infrastructure

## Environments
| Env | Deploy method |
|-----|--------------|
| dev | ... |

## CI/CD
Pipeline: `<file>`, trigger: <how>

## Config
- `<file>`: <what>
- Secrets: <names, NOT values>

## Monitoring
- Logs: <where>
- Health: <how>
```

### `decisions.md`

ADRs. Additive — never edit old.

Each: date, context, decision, alternatives rejected (with why), consequences.

New at TOP. Only decisions a future agent needs. Not "tabs vs spaces".

```
## YYYY-MM-DD: <title>
**Context**: <problem>
**Decision**: <chosen>
**Alternatives**: <rejected, why>
**Consequences**: <implications>
```

### `lessons.md`

Post-mortems. Additive.

Each: what happened, root cause, fix, prevention.

New at TOP. Be specific — include file paths, task IDs, error messages.

```
## YYYY-MM-DD: <title>
**What**: <failure>
**Cause**: <why>
**Fix**: <done>
**Prevention**: <avoid next time>
```
