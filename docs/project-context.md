# NBP Project Context

> Rules, constraints, and invariants for AI agents working on this codebase.

## Tech Stack

- **Framework:** Tauri 2 (Rust backend + Vanilla JS frontend)
- **Package Manager:** bun (not npm)
- **Bundler:** None - static files served directly
- **Audio Encoding:** OGG Vorbis via vorbis_rs
- **Audio Playback:** rodio
- **System Audio:** Core Audio Process Taps (cidre) - macOS only
- **Mic Capture:** cpal

## Constraints (MUST NOT)

- [ ] Never use npm - use bun for all package operations
- [ ] Never add bundler dependencies (webpack, vite, etc.)
- [ ] Never store API keys in plaintext files
- [ ] Never use cpal device IDs directly - they are unstable; use device name as identifier
- [ ] Never assume microphone sample rate - may be 16kHz (bluetooth) or 48kHz (built-in)
- [ ] Never modify raw audio files after recording stops (immutable source)

## Invariants (MUST ALWAYS)

- [ ] Real-time mixer expects 48kHz - resample non-48kHz sources before mixing
- [ ] Body CSS classes control UI state: `is-recording-active`, `detail-open`, `settings-open`
- [ ] Theme changes apply via body class: `deep-obsidian`, `deep-blue`
- [ ] All themes must meet WCAG AA contrast ratio (4.5:1 minimum for text)
- [ ] One recording = one directory in ~/nbp-data/{uuid}/
- [ ] metadata.json is the source of truth for recording state
- [ ] OGG encoder uses native sample rate for best quality

## Patterns

### Audio State

```
Recording active → body.is-recording-active
  → Hide audio player (#audio-player-section)
  → Show recording waveform (.recording-waveform)
  → Disable mic selector (#settings-microphone)
  → Disable refresh button (#refresh-devices-btn)
```

### Theme Application

```javascript
// Remove all theme classes, then add new one
document.body.classList.remove('deep-obsidian', 'deep-blue');
document.body.classList.add(themeName);
```

### Tauri IPC

```javascript
// Always use window.__TAURI__.invoke()
const result = await window.__TAURI__.invoke('command_name', { arg1: value1 });
```

## Build Commands

```bash
cargo tauri dev      # Development
cargo tauri build    # Production
```

## Directory Structure

```
src/                  # Frontend (vanilla HTML/JS/CSS)
src-tauri/src/        # Rust backend
  audio.rs            # Recording commands
  mic_audio.rs        # Microphone capture
  system_audio.rs     # System audio capture (cidre)
  devices.rs          # Device enumeration
  playback.rs         # In-app audio playback
  config.rs           # Settings persistence
~/nbp-data/           # Recording storage
~/.nbp/               # App settings and models
```
