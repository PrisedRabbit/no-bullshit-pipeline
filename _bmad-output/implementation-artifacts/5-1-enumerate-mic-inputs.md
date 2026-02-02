# Story 5.1: Enumerate Available Microphone Inputs

Status: done

## Story

As a user,
I want to see all available microphone inputs in the app,
So that I can choose which microphone to use for recording.

## Acceptance Criteria

1. **Given** the app is running **When** I open the settings or recording view **Then** I see a list of available microphone inputs (built-in, bluetooth, external) **And** the current default input is highlighted

2. **Given** I connect a new microphone (bluetooth or USB) **When** the system detects the new device **Then** the microphone list updates to include the new device

## Tasks / Subtasks

- [x] Backend: Verify existing `get_input_devices` Tauri command returns complete device info (AC: #1)
  - [x] Confirm AudioDeviceInfo includes id, name, sample_rate, channels, is_default
  - [x] Test command returns all connected input devices
- [x] Frontend: Add microphone selector dropdown in Settings → Transcription section (AC: #1)
  - [x] Call `get_input_devices()` command on settings view load
  - [x] Render dropdown with device names
  - [x] Mark default device visually (indicator or label)
- [x] Frontend: Implement device hot-plug detection (AC: #2)
  - [x] Add event listener or polling mechanism for device changes
  - [x] Refresh device list when new device connected
  - [x] Update dropdown options dynamically
- [x] Settings: Persist selected device name in app settings (AC: #1)
  - [x] Add `selected_microphone` field to AppSettings struct
  - [x] Save/load selected device name
- [x] UI: Show device type indicators (Built-in, Bluetooth, External) (AC: #1)
  - [x] Parse device name to infer type
  - [x] Display type in parentheses (e.g., "AirPods Pro (Bluetooth)")

## Dev Notes

### Critical Backend Context

**✅ DEVICE ENUMERATION ALREADY IMPLEMENTED**
- **File:** `src-tauri/src/devices.rs` (57 lines, complete implementation)
- **Tauri Command:** `get_input_devices()` - Ready to call from frontend
- **Data Structure:** `AudioDeviceInfo` with id, name, sample_rate, channels, is_default
- **Device ID Strategy:** Uses device **name** as ID (cpal IDs are unstable per project-context.md)

**Backend Requirements (from project-context.md):**
- ❌ **NEVER use cpal device IDs directly** - they are unstable
- ✅ **ALWAYS use device name as identifier**
- ⚠️ **Sample rates vary:** Bluetooth (16kHz), Built-in (48kHz) - display but don't constrain

### Frontend Integration Points

**Location:** Settings View (`src/index.html` lines 290-475, `src/main.js`)

**Current Settings Structure:**
```html
<div class="settings-section">
  <h3>Transcription</h3>
  <div class="settings-item">
    <!-- Add mic selector here -->
  </div>
</div>
```

**Tauri IPC Pattern (from project-context.md):**
```javascript
const devices = await window.__TAURI__.invoke('get_input_devices');
```

**State Management:**
- Settings loaded via `load_settings()` command (returns AppSettings struct)
- Settings saved via `save_settings(settings)` command
- Add `selected_microphone: string | null` to AppSettings

**CSS Classes for UI State:**
- Use existing `.settings-select` class for dropdown styling
- Matches theme system: `--text-primary`, `--text-secondary`, `--bg-input`, `--border`

### Device Hot-Plug Detection

**Strategy Options:**
1. **Polling:** Call `get_input_devices()` every 2-3 seconds when settings open
2. **Manual Refresh:** Add "Refresh" button next to dropdown
3. **Event-based (future):** cpal doesn't natively support device change events

**Recommended:** Polling when settings view active (pause when closed)

### Device Type Indicators

**Heuristics for device type:**
- "Built-in" or "MacBook" → Built-in
- "AirPods", "Bluetooth" → Bluetooth
- Otherwise → External

**Display Format:**
```
MacBook Pro Microphone (Built-in)  [Default]
AirPods Pro (Bluetooth)
Shure MV7 (External)
```

### Architecture Compliance

**From architecture.md:**
- Mic capture via cpal (`mic_audio.rs`)
- Settings persistence in `~/.nbp/settings.json`
- Frontend: Vanilla HTML/JS/CSS (no bundler)
- Tauri 2.9.5 IPC via `invoke()` pattern

**From project-context.md:**
- Package manager: bun (not npm)
- Theme CSS variables apply via body classes
- Settings structure managed in Rust (`config.rs`)

### Testing Requirements

**Manual Testing Checklist:**
1. Open Settings → see dropdown with devices
2. Default device shows indicator
3. Connect bluetooth headset → dropdown updates
4. Select different device → saved in settings
5. Restart app → selected device persists
6. Device names show type indicators (Built-in/Bluetooth/External)

**Files to Modify:**
- `src/index.html` (add dropdown in Settings section)
- `src/main.js` (add device loading, polling, save logic)
- `src-tauri/src/config.rs` (add `selected_microphone` field to AppSettings)
- `src-tauri/src/lib.rs` (ensure `get_input_devices` command registered)

**Files to Review:**
- `src-tauri/src/devices.rs` (understand device data structure)
- `src/styles.css` (use `.settings-select` class for dropdown)

### Project Structure Notes

**Alignment:**
- Settings UI follows established pattern (toggles, dropdowns, input fields)
- Rust backend uses existing devices.rs module
- No new dependencies required

### References

- [Source: src-tauri/src/devices.rs] - Backend device enumeration implementation
- [Source: docs/project-context.md#Constraints] - Device ID stability rule
- [Source: docs/architecture.md#Core Modules] - Mic capture architecture
- [Source: docs/prd.md#Functional Requirements] - FR58-FR60 requirements
- [Source: docs/epics/epics-v04.md#Epic 5] - Story requirements and AC
- [Source: docs/ux-design.md#Microphone Selector] - UI/UX specifications

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- cargo check passed with minor warnings (unused doc comment, dead code)
- cargo tauri dev compiled and ran successfully

### Completion Notes List

1. ✅ Backend `get_input_devices` command verified - already registered in lib.rs:80, devices.rs complete with AudioDeviceInfo struct
2. ✅ Added `selected_microphone: Option<String>` field to AppSettings in config.rs
3. ✅ Added microphone selector dropdown in Settings → Audio Input section (index.html)
4. ✅ Implemented device loading via `loadMicrophoneDevices()` function (main.js)
5. ✅ Implemented polling-based hot-plug detection every 2 seconds when settings view open
6. ✅ Added device type inference (Built-in/Bluetooth/USB/External) via `getDeviceType()` function
7. ✅ Default device marked with [Default] label in dropdown
8. ✅ Refresh button added for manual device list reload
9. ✅ Selection persisted via `appSettings.selected_microphone`
10. ✅ Code review pass - improved device type heuristics for common USB mics and Bluetooth devices

### File List

**Modified:**
- `src-tauri/src/config.rs` - Added `selected_microphone` field to AppSettings
- `src/index.html` - Added microphone selector dropdown in Settings
- `src/main.js` - Added device loading, polling, type inference logic

**Reviewed (no changes needed):**
- `src-tauri/src/devices.rs` - Already complete
- `src-tauri/src/lib.rs` - `get_input_devices` already registered
