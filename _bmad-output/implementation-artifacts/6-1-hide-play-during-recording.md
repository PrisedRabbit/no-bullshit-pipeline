# Story 6.1: Hide Play Button During Recording

Status: done

## Story

As a user,
I want the PLAY button hidden while I'm recording,
So that the interface is cleaner and I don't accidentally try to play while recording.

## Acceptance Criteria

1. **Given** I am not recording **When** I view the main interface **Then** the PLAY button is visible for selected recordings

2. **Given** I start recording **When** the recording is active **Then** the PLAY button is hidden **And** only the STOP button is prominently visible

3. **Given** I stop recording **When** the recording completes **Then** the PLAY button becomes visible again

## Tasks / Subtasks

- [x] CSS: Hide audio player section during recording (AC: #2, #3)
  - [x] Add CSS rule to hide `#audio-player-section` when `body.is-recording-active`
  - [x] Player automatically shows when recording stops (class removed)

## Dev Notes

### Implementation Strategy

Simple CSS-only solution using existing state class:

```css
body.is-recording-active #audio-player-section {
  display: none;
}
```

This approach:
- No JavaScript changes needed
- Uses existing `is-recording-active` class (added in main.js:205, removed in main.js:238)
- Automatic show/hide based on recording state

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Completion Notes List

1. ✅ Added CSS rule to hide audio player during recording

### File List

**Modified:**
- `src/styles.css` - Added hiding rule for audio player during recording
