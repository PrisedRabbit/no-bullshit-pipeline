# Story 4.1: Audio Mix Error Handling

Status: ready-for-dev

## Story

As a user,
I want robust error handling during recording,
so that audio drift and mix failures are detected and reported gracefully.

## Acceptance Criteria

1. **Given** mic and system audio are being recorded
   **When** sample rate drift is detected
   **Then** the system compensates or warns me without crashing

2. **Given** one audio source fails during recording
   **When** the failure occurs
   **Then** I receive a notification and the other source continues recording

3. **Given** a recording completes with errors
   **When** I view the recording
   **Then** I see an indicator that issues occurred during capture

## Tasks / Subtasks

- [ ] Task 1: Drift detection and compensation (AC: 1)
  - [ ] Monitor sample count difference between mic and system audio
  - [ ] Detect drift exceeding threshold (e.g., >100ms)
  - [ ] Implement resampling compensation if drift detected
  - [ ] Log drift events for debugging

- [ ] Task 2: Source failure handling (AC: 2)
  - [ ] Wrap audio capture in error handlers
  - [ ] Continue recording if one source fails
  - [ ] Emit event to frontend on source failure
  - [ ] Record which sources were active/failed

- [ ] Task 3: Recording health indicators (AC: 3)
  - [ ] Add `recording_health` field to metadata
  - [ ] Store issues list (drift events, source failures)
  - [ ] Show warning icon in recordings list for problematic recordings
  - [ ] Show detailed health info in detail view

## Dev Notes

### Architecture Constraints
- Current audio mixing in `src-tauri/src/audio.rs`
- Real-time mixing happens during recording
- Drift can occur due to different sample rate clocks

### Health Metadata Schema
```json
{
  "recording_health": {
    "status": "warning",  // "ok", "warning", "error"
    "issues": [
      {"type": "drift", "timestamp_ms": 45000, "drift_ms": 150},
      {"type": "source_lost", "source": "system_audio", "timestamp_ms": 120000}
    ]
  }
}
```

### Source Tree Components
- `src-tauri/src/audio.rs` - Add error handling (primary)
- `src-tauri/src/storage.rs` - Health metadata storage
- `src/main.js` - Health indicator display
- `src/index.html` - Warning icon UI
- `src/styles.css` - Warning styling

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 4.1]
- [Source: docs/architecture.md]

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
