# Story 6.2: Real-time Recording Waveform Visualization

Status: done

## Story

As a user,
I want to see a small waveform visualization near the record button while recording,
So that I have visual feedback that audio is being captured.

## Acceptance Criteria

1. **Given** I am recording **When** audio is being captured **Then** I see a compact waveform animation near the record button **And** the waveform responds to audio levels in real-time

2. **Given** the recording is paused **When** I look at the waveform **Then** the waveform shows a flat line or paused state

3. **Given** I am not recording **When** I look at the interface **Then** no waveform visualization is shown (or it shows idle state)

## Tasks / Subtasks

- [x] Backend: Audio level API (already exists)
  - [x] `get_audio_level` Tauri command returns real-time RMS level
- [x] Frontend: Waveform canvas and animation (already exists)
  - [x] Canvas element near record button
  - [x] 5-bar spectrum visualization with accent color
  - [x] 33fps update rate for smooth animation
  - [x] Instant attack, medium decay for natural feel
- [x] CSS: Show waveform only during recording (already exists)
  - [x] `.recording-waveform` hidden by default
  - [x] Shown when `body.is-recording-active`

## Dev Notes

### Already Implemented

**HTML (index.html:32-34):**
```html
<div id="recording-waveform" class="recording-waveform">
  <canvas id="recording-waveform-canvas" class="recording-waveform-canvas" width="40" height="20"></canvas>
</div>
```

**JavaScript (main.js:105-184):**
- `startWaveformAnimation()` - starts 33fps polling of audio level
- `stopWaveformAnimation()` - stops animation and clears canvas
- `drawSpectrum()` - renders 5-bar spectrum visualization
- Uses accent color from CSS variables

**CSS (styles.css:1464-1480):**
```css
.recording-waveform {
  display: none;
}
body.is-recording-active .recording-waveform {
  display: flex;
}
.recording-waveform-canvas {
  width: 40px;
  height: 20px;
}
```

**Backend (mic_audio.rs + lib.rs):**
- `CURRENT_AUDIO_LEVEL` atomic stores RMS level
- `get_audio_level` Tauri command exposes it to frontend

### Why This Story is Pre-Done

This feature was implemented as part of the v0.3 real-time mixer work to provide visual feedback during recording.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Completion Notes List

1. ✅ Verified backend audio level API exists (mic_audio.rs:34-36, lib.rs:get_audio_level)
2. ✅ Verified frontend waveform animation exists (main.js:105-184)
3. ✅ Verified CSS shows waveform only during recording (styles.css:1464-1473)
4. ✅ No code changes required - functionality already implemented

### File List

**Reviewed (no changes needed):**
- `src-tauri/src/mic_audio.rs` - Audio level capture
- `src-tauri/src/lib.rs` - get_audio_level command
- `src/main.js` - Waveform animation logic
- `src/index.html` - Canvas element
- `src/styles.css` - Visibility rules
