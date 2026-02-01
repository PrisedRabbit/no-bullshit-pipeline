# Story 1.1: Secure API Key Storage

Status: ready-for-dev

## Story

As a user,
I want to securely store my API keys for cloud services,
so that my credentials are protected and persist across sessions.

## Acceptance Criteria

1. **Given** I am in the Settings view
   **When** I enter an API key for a service (OpenAI/Google/Anthropic)
   **Then** the key is encrypted and stored in `~/.nbp/settings.json`
   **And** the key is masked in the UI after saving

2. **Given** I have saved API keys
   **When** I restart the application
   **Then** my keys are available for use without re-entering

## Tasks / Subtasks

- [ ] Task 1: Extend config.rs for multiple API keys (AC: 1, 2)
  - [ ] Add `ApiKeys` struct with fields for openai, google, anthropic
  - [ ] Add Anthropic to `TranscriptionProvider` enum
  - [ ] Update `TranscriptionConfig` to use `ApiKeys` struct
  - [ ] Set file permissions to 0600 after saving settings

- [ ] Task 2: Update frontend UI for multiple API keys (AC: 1)
  - [ ] Add separate input fields for each provider's API key
  - [ ] Show/hide relevant API key input based on selected provider
  - [ ] Mask API keys after saving (show only last 4 chars)

- [ ] Task 3: Update settings load/save logic (AC: 1, 2)
  - [ ] Update `load_settings` to handle new ApiKeys structure
  - [ ] Update `save_settings` to persist all API keys
  - [ ] Add migration for existing single api_key field

## Dev Notes

### Architecture Constraints
- Settings stored at `~/.nbp/settings.json`
- Tauri 2 IPC pattern: `invoke("command_name", { params })`
- Frontend: Vanilla JS (no React/Vue)
- Backend: Rust with serde for serialization

### Source Tree Components
- `src-tauri/src/config.rs` - Settings management (primary)
- `src/index.html` - Settings UI (lines 378-396)
- `src/main.js` - Frontend settings logic (lines 727-794)

### Testing Standards
- Manual testing: Save keys, restart app, verify persistence
- Verify file permissions are 0600 (user-only read/write)

### Project Structure Notes
- Config dir: `~/.nbp/`
- Models dir: `~/.nbp/models/`
- Data dir: `~/nbp-data/` (configurable)

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.1]
- [Source: docs/architecture.md]
- [Source: src-tauri/src/config.rs]

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
