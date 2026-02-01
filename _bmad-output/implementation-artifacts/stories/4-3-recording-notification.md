# Story 4.3: Recording Notification

Status: ready-for-dev

## Story

As a user,
I want to optionally notify others when NBP is recording,
so that meeting participants are aware of the recording.

## Acceptance Criteria

1. **Given** I am about to start a recording
   **When** notification setting is enabled
   **Then** a system notification is displayed indicating recording is active

2. **Given** I prefer silent recording
   **When** I disable the notification setting
   **Then** no notification is shown when recording starts

3. **Given** recording is in progress
   **When** the notification is visible
   **Then** it clearly identifies NBP as the recording application

## Tasks / Subtasks

- [ ] Task 1: Add notification setting (AC: 1, 2)
  - [ ] Add `show_recording_notification` to AppSettings
  - [ ] Default to `true` for transparency
  - [ ] Add toggle in Settings UI
  - [ ] Persist setting in settings.json

- [ ] Task 2: Implement system notification (AC: 1, 3)
  - [ ] Use `tauri-plugin-notification` or native macOS API
  - [ ] Show notification on recording start
  - [ ] Notification text: "NBP is recording audio"
  - [ ] Include NBP icon in notification

- [ ] Task 3: Notification lifecycle (AC: 1)
  - [ ] Show notification when recording starts
  - [ ] Optionally dismiss when recording stops
  - [ ] Handle notification permissions

## Dev Notes

### Architecture Constraints
- Use Tauri notification plugin or native macOS notifications
- Respect user preference for silent recording
- Notification should be non-intrusive but visible

### Notification Content
```
Title: NBP Recording
Body: Audio recording is in progress
Icon: NBP app icon
```

### Settings Addition
```rust
pub struct AppSettings {
    // ... existing fields
    pub show_recording_notification: bool,  // default: true
}
```

### Dependencies
- `tauri-plugin-notification` - May need to add to Cargo.toml

### Source Tree Components
- `src-tauri/src/config.rs` - Add setting
- `src-tauri/src/audio.rs` - Trigger notification on record start
- `src/index.html` - Settings toggle UI
- `src/main.js` - Settings binding

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 4.3]
- [Tauri Notification Plugin](https://v2.tauri.app/plugin/notification/)

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
