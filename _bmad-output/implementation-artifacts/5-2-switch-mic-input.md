# Story 5.2: Switch Microphone Input

Status: done

## Story

As a user,
I want to switch between different microphone inputs,
So that I can use my preferred microphone for recording.

## Acceptance Criteria

1. **Given** I see the list of available microphones **When** I select a different microphone **Then** the app switches to use that microphone for recording **And** I see visual confirmation of the switch

2. **Given** I am currently recording **When** I try to switch microphones **Then** the switch is prevented or the recording is paused first

## Tasks / Subtasks

- [x] Backend: Add function to find device by name (AC: #1)
  - [x] Add `get_device_by_name(name: &str)` function in devices.rs
  - [x] Return the device matching the name, or fallback to default
- [x] Backend: Modify MicAudioRecorder to accept device name (AC: #1)
  - [x] Update `MicAudioRecorder::new()` signature to take `Option<String>` device_name
  - [x] Use selected device if provided, otherwise default
  - [x] Update `start_mic_capture()` to pass device name
- [x] Backend: Add Tauri command to start recording with device (AC: #1)
  - [x] Update `start_recording` command to accept optional device_name
  - [x] Pass device_name to mic capture
- [x] Frontend: Pass selected microphone when starting recording (AC: #1)
  - [x] Read `appSettings.selected_microphone` before recording
  - [x] Pass to `start_recording` Tauri command
- [x] Frontend: Disable mic selector during recording (AC: #2)
  - [x] Disable dropdown when recording starts
  - [x] Show tooltip "Cannot change microphone while recording"
  - [x] Re-enable when recording stops

## Dev Notes

### Backend Context

**Current Implementation (mic_audio.rs:60-64):**
```rust
let host = cpal::default_host();
let device = host.default_input_device()
    .ok_or(anyhow::anyhow!("No input device available"))?;
```

**Required Change:**
- Accept device name parameter
- Find device by name from `host.input_devices()`
- Fall back to default if not found

**Device Lookup Pattern:**
```rust
fn get_device_by_name(name: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    host.input_devices().ok()?.find(|d| d.name().ok().as_deref() == Some(name))
}
```

### Frontend Integration

**Recording Start (main.js):**
- Current: `await invoke('start_recording');`
- New: `await invoke('start_recording', { deviceName: appSettings.selected_microphone });`

**UI Disable Pattern:**
```javascript
// When recording starts
microphoneSelect.disabled = true;

// When recording stops
microphoneSelect.disabled = false;
```

### Files to Modify

- `src-tauri/src/devices.rs` - Add device lookup function
- `src-tauri/src/mic_audio.rs` - Accept device name parameter
- `src-tauri/src/audio.rs` - Update start_recording command
- `src/main.js` - Pass device name, disable selector during recording

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- cargo check passed with existing warnings only (unused doc comment, dead code in waveform.rs)

### Completion Notes List

1. ✅ Added `get_device_by_name(name: &str)` function in devices.rs:59-66
2. ✅ Updated `MicAudioRecorder::new()` to accept `Option<String>` device_name (mic_audio.rs:60-74)
3. ✅ Updated `start_mic_capture()` signature to pass device name (mic_audio.rs:168)
4. ✅ Updated `start_recording` Tauri command to accept `device_name` parameter (audio.rs:39)
5. ✅ Updated permissions.rs mic test to pass `None` for device name
6. ✅ Frontend passes `appSettings.selected_microphone` to start_recording (main.js:199-201)
7. ✅ Mic selector disabled during recording with tooltip (main.js:213-219)
8. ✅ Mic selector re-enabled after recording stops (main.js:254-260)

### File List

**Modified:**
- `src-tauri/src/devices.rs` - Added get_device_by_name function
- `src-tauri/src/mic_audio.rs` - Accept device_name parameter
- `src-tauri/src/audio.rs` - Updated start_recording command
- `src-tauri/src/permissions.rs` - Updated mic test call
- `src/main.js` - Pass device to recording, disable selector during recording
