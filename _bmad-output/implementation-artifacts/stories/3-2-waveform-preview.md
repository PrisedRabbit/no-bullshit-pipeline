# Story 3.2: Waveform Preview

Status: ready-for-dev

## Story

As a user,
I want to see a waveform visualization of my recordings,
so that I can visually navigate and identify sections.

## Acceptance Criteria

1. **Given** I am viewing a recording's detail view
   **When** the view loads
   **Then** a waveform of the audio is displayed

2. **Given** the waveform is displayed
   **When** audio is playing
   **Then** a playhead indicator shows the current position

3. **Given** the waveform is displayed
   **When** I click on a position in the waveform
   **Then** playback seeks to that position

## Tasks / Subtasks

- [ ] Task 1: Generate waveform data backend (AC: 1)
  - [ ] Create `src-tauri/src/waveform.rs` module
  - [ ] Decode audio and sample amplitude data
  - [ ] Downsample to reasonable resolution (e.g., 1000 points)
  - [ ] Cache waveform data for fast reload

- [ ] Task 2: Waveform visualization frontend (AC: 1, 2)
  - [ ] Create canvas-based waveform renderer
  - [ ] Draw waveform bars from amplitude data
  - [ ] Style with theme colors
  - [ ] Show playhead indicator during playback

- [ ] Task 3: Interactive seeking (AC: 3)
  - [ ] Handle click events on waveform canvas
  - [ ] Calculate position from click coordinates
  - [ ] Trigger seek via playback commands
  - [ ] Update playhead position

## Dev Notes

### Architecture Constraints
- Generate waveform server-side (Rust) for performance
- Send amplitude data as array to frontend
- Frontend renders using HTML5 Canvas
- Resolution: ~1000 data points regardless of audio length

### Waveform Data Format
```json
{
  "duration_ms": 180000,
  "samples": [0.1, 0.3, 0.5, 0.8, 0.4, ...],  // normalized 0-1
  "sample_rate": 1000  // points total
}
```

### Dependencies to Add (Cargo.toml)
- May reuse `symphonia` or `rodio` decoder already in use

### Tauri Commands
- `get_waveform_data(recording_id: String)` - Returns waveform samples

### Source Tree Components
- `src-tauri/src/waveform.rs` - New module
- `src-tauri/src/lib.rs` - Register waveform commands
- `src/main.js` - Waveform rendering and interaction
- `src/styles.css` - Waveform styling
- Depends on: Story 3.1 (Audio Playback)

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.2]

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
