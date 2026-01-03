---
trigger: always_on
---

# no-bullshit-pipeline

Local voice → structured data. Privacy-first. No bullshit.

## Scope

- Local audio capture and processing
- Inputs: mic, system audio, audio files
- Outputs: audio, transcript, structured data

## Principles

- Local-first
- Privacy by default (no network unless explicit)
- Raw audio is immutable
- All outputs derive from raw input

## Pipeline

- `capture → store → process → export`
- Processing is restartable and idempotent
- No step mutates previous data

## Storage

- Files, not databases
- Human-readable formats (audio, text, JSON)
- Data usable without this tool
- **Organization**: Co-located assets. Audio (`.ogg`), metadata (`.json`), and content (`.md`) reside in the same directory.
- Use universal, open formats.

## Tech

- Core: Rust
- UI shell: Tauri (thin)
- Native OS audio APIs only
- External APIs via explicit keys only

## Constraints

- UI never defines data flow
- No workflow/task management
- No telemetry or background services

## Rule Zero

If it breaks ownership, locality, or debuggability — don’t do it.
