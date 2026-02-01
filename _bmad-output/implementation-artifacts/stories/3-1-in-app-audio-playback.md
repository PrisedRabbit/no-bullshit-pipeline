# Story 3.1: In-App Audio Playback

Status: ready-for-dev

## Story

As a user,
I want to play recordings directly in the app,
so that I don't need to open external applications.

## Acceptance Criteria

1. **Given** I am viewing a recording's detail view
   **When** I click the play button
   **Then** the mixed audio plays through system speakers

2. **Given** audio is playing
   **When** I click pause
   **Then** playback pauses and can be resumed

3. **Given** audio is playing
   **When** I use the seek bar
   **Then** playback jumps to the selected position

4. **Given** I switch to a different recording
   **When** audio is playing
   **Then** playback stops automatically

## Tasks / Subtasks

- [ ] Task 1: Create audio playback backend (AC: 1, 2, 3)
  - [ ] Create `src-tauri/src/playback.rs` module
  - [ ] Use `rodio` crate for audio playback
  - [ ] Implement play, pause, resume, stop commands
  - [ ] Implement seek functionality

- [ ] Task 2: Playback state management (AC: 2, 4)
  - [ ] Track current playback state (playing, paused, stopped)
  - [ ] Track current position in audio
  - [ ] Auto-stop when switching recordings
  - [ ] Emit events for position updates

- [ ] Task 3: UI audio controls (AC: 1, 2, 3)
  - [ ] Add play/pause button to detail view
  - [ ] Add seek bar with current position
  - [ ] Show total duration and current time
  - [ ] Update UI state based on playback events

## Dev Notes

### Architecture Constraints
- Audio format: OGG Vorbis (already saved in this format)
- Use `rodio` crate for cross-platform audio playback
- Emit position updates via Tauri events for UI sync

### Dependencies to Add (Cargo.toml)
- `rodio = "0.17"` - Audio playback

### Tauri Commands
- `play_audio(path: String)` - Start playback
- `pause_audio()` - Pause playback
- `resume_audio()` - Resume playback
- `stop_audio()` - Stop playback
- `seek_audio(position_ms: u64)` - Seek to position
- `get_playback_state()` - Get current state

### Tauri Events
- `playback-position` - Emitted every 100ms with current position
- `playback-ended` - Emitted when audio finishes

### Source Tree Components
- `src-tauri/src/playback.rs` - New module
- `src-tauri/src/lib.rs` - Register playback commands
- `src/index.html` - Audio control UI
- `src/main.js` - Playback control handlers

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.1]

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
