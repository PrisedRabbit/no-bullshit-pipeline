# Story 5.3: Handle Different Sample Rates

Status: done

## Story

As a user,
I want the app to work correctly with different microphones that have different sample rates,
So that I can use bluetooth headsets (16kHz) and built-in mics (48kHz) interchangeably.

## Acceptance Criteria

1. **Given** I select a bluetooth microphone with 16kHz sample rate **When** I start recording **Then** the audio is captured correctly and resampled to the standard output rate **And** there is no audio distortion or sync issues

2. **Given** I switch from a 16kHz mic to a 48kHz mic **When** I start a new recording **Then** the app handles the sample rate change seamlessly

## Tasks / Subtasks

- [x] Backend: Verify resampling is already implemented (AC: #1)
  - [x] Review mic_audio.rs for resampling logic
  - [x] Confirm rubato resampler handles non-48kHz inputs
- [x] Backend: Verify per-recording sample rate detection (AC: #2)
  - [x] Confirm device config is read at recording start
  - [x] Confirm sample rate is used dynamically (not hardcoded)
- [x] Frontend: No changes needed - backend handles transparently

## Dev Notes

### Already Implemented

The sample rate handling was already implemented in `mic_audio.rs`:

**Sample Rate Detection (lines 67-68):**
```rust
let config = device.default_input_config()?;
let sample_rate = config.sample_rate().0;
```

**Resampling Setup (lines 179-200):**
```rust
let needs_resampling = sample_rate != MIXER_SAMPLE_RATE;
let mut resampler: Option<SincFixedIn<f32>> = if needs_resampling {
    println!("Mic: Resampling from {}Hz to {}Hz for real-time mixer", sample_rate, MIXER_SAMPLE_RATE);
    // ... rubato SincFixedIn resampler setup
}
```

**Key Points:**
- MIXER_SAMPLE_RATE is 48kHz (constant)
- Any input device with different sample rate triggers resampling
- Uses high-quality sinc interpolation (rubato crate)
- Resampler chunk processing in main loop (lines 294-314)
- Flush remaining samples at end (lines 349-372)

### Why This Story is Pre-Done

The v0.3 real-time mixer feature required sample rate normalization for consistent mixing. The resampling infrastructure was added then, which means:

- Bluetooth 16kHz mics → resampled to 48kHz ✓
- Built-in 48kHz mics → no resampling needed ✓
- USB mics at various rates → resampled as needed ✓

No additional work required - this story validates existing functionality.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- Code review of mic_audio.rs confirmed resampling implementation
- No code changes needed

### Completion Notes List

1. ✅ Verified resampling logic exists in mic_audio.rs:179-200
2. ✅ Verified per-device sample rate detection in mic_audio.rs:67-68
3. ✅ Verified resampler chunk processing in mic_audio.rs:294-314
4. ✅ Verified flush logic for remaining samples in mic_audio.rs:349-372
5. ✅ No code changes required - functionality already implemented

### File List

**Reviewed (no changes needed):**
- `src-tauri/src/mic_audio.rs` - Resampling already implemented
