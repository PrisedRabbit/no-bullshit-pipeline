# v0.1 · PLAN

## 0. Core

- Minimalist UI
- Showing List of previous recordings
- Showing current recording

## 1. Projects (lightweight)

**Goal:** grouping, not management.

- `project` = set of tags
- Tags can be added/removed at any time:
  - before recording
  - during recording
  - after recording
- Tags are stored on the recording as metadata
- No hierarchies, no statuses, no rules

> Projects are filters, not workflows.

---

## 2. Capture Controls (MVP)

**Goal:** zero thinking while recording.

- One primary control:
  - `Record` → `Pause` → `Resume` → `Stop`
- Clear states:
  - idle / recording / paused
- Minimal UI:
  - timer
  - current project/tags
- No waveforms, no hotkeys (v0.1)

---

## 3. Raw Audio Capture (core)

**Goal:** the file always exists.

- Permissions:
  - request microphone access
  - request system audio access
  - handle denied state
- On record start:
  - create recording ID
  - open audio file immediately
- During capture:
  - write in chunks
  - update metadata
- On stop:
  - close file safely
- Errors must not destroy already written data

---

## 4. Processing

**Goal:** capture ≠ processing.

- After `Stop`:
  - recording becomes `ready`
- Processing:
  - triggered manually (button)
  - optional auto-run flag later
- Each step:
  - separate artifact
  - explicit status (`pending / done / failed`)

---

## 5. Artifacts (output)

**Goal:** take data and leave.

- Minimal set:
  - `audio.ogg`
  - `transcript.md`
  - `summary.md`
  - `meta.json`
- Stable structure
- Script-friendly, no app required to read

---

## 6. Explicit Non-Goals (v0.1)

- text editing
- workflows / tasks
- sync / cloud
- iOS app
- advanced settings

---

## Success Criteria

User can:

1. record
2. pause / resume
3. stop
4. run processing
5. grab files and move on
