# NBP Development Guide

## Prerequisites

| Requirement | Version | Notes |
|-------------|---------|-------|
| **macOS** | 13.0+ (Ventura) | Required for Core Audio Process Taps |
| **Rust** | Latest stable (2024 edition) | Install via rustup |
| **Bun** | Latest | Fast JS runtime/package manager |
| **Xcode CLI** | Latest | For Metal framework and build tools |

## Quick Start

```bash
# Clone the repository
git clone <repo-url>
cd nbp

# Install JavaScript dependencies
bun install

# Run in development mode
bun tauri dev
```

The app will open automatically. Development mode includes hot-reload for frontend changes.

## Build Commands

| Command | Description |
|---------|-------------|
| `bun tauri dev` | Development mode with hot-reload |
| `bun tauri build` | Build release binary |
| `./build.sh` | Build and package as DMG |

## Project Structure

```
nbp/
├── src/                 # Frontend code (edit these)
├── src-tauri/src/       # Backend code (edit these)
├── src-tauri/icons/     # App icons
├── builds/              # Release artifacts
└── docs/                # Documentation
```

## Development Workflow

### Frontend Development

The frontend is vanilla HTML/JS/CSS with no build step:

1. Edit `src/index.html` for structure
2. Edit `src/main.js` for logic
3. Edit `src/styles.css` for styling
4. Changes hot-reload in `bun tauri dev`

**Key patterns:**
- Use `invoke("command_name", { args })` for backend calls
- Use CSS classes on `body` for view states
- Global functions attached to `window` for onclick handlers

### Backend Development

The backend is Rust with Tauri:

1. Edit files in `src-tauri/src/`
2. Backend recompiles automatically in dev mode (slower than frontend)
3. Add new commands to `lib.rs` invoke_handler

**Adding a new command:**

```rust
// In the relevant module (e.g., storage.rs)
#[tauri::command]
pub fn my_new_command(arg: String) -> Result<String, String> {
    // Implementation
    Ok("result".to_string())
}

// In lib.rs, add to invoke_handler
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    my_module::my_new_command,
])
```

**Calling from frontend:**

```javascript
const result = await invoke("my_new_command", { arg: "value" });
```

## Key Files to Know

### Must-Read for New Features

| File | When to modify |
|------|----------------|
| `src-tauri/src/lib.rs` | Adding new commands |
| `src/main.js` | Any frontend logic |
| `src/index.html` | New UI elements |
| `src-tauri/src/storage.rs` | Recording metadata changes |

### Audio Pipeline

| File | Purpose |
|------|---------|
| `audio.rs` | Recording state machine |
| `mic_audio.rs` | Microphone capture |
| `system_audio.rs` | System audio capture |
| `audio_processing/*.rs` | Normalization and mixing |

## Environment Setup

### macOS Permissions

The app requires:
1. **Microphone access** - For voice recording
2. **Screen Recording** - For system audio capture (Core Audio Taps)

On first run, grant permissions when prompted or via System Preferences > Privacy & Security.

### Running Unsigned Builds

For development builds without code signing:

```bash
xattr -cr /Applications/nbp.app
```

## Data Locations

| Data | Location |
|------|----------|
| Recordings | `~/nbp-data/` |
| Settings | `~/.nbp/settings.json` |
| Whisper Models | `~/.nbp/models/` |
| Build Artifacts | `./builds/` |

## Testing

Currently no automated tests. Manual testing checklist:

1. **Recording:** Start/stop, verify files created
2. **Playback:** Open folder, play in external player
3. **Transcription:** Download model, transcribe, verify output
4. **Settings:** Change storage path, theme, auto-discard
5. **Permissions:** Test onboarding flow, permission requests

## Common Tasks

### Changing Audio Format

Edit encoding parameters in `mic_audio.rs` and `system_audio.rs`:

```rust
VorbisBitrateManagementStrategy::QualityVbr { target_quality: 0.4 }
```

Quality 0.4 ≈ 128kbps VBR. Range: 0.0 (lowest) to 1.0 (highest).

### Adding a New Theme

1. Add CSS variables in `styles.css`:
```css
body.my-theme {
  --bg-primary: #...;
  --accent: #...;
  /* etc */
}
```

2. Add button in `index.html` settings section
3. Update theme handling in `main.js`

### Adding a New Whisper Model

Edit `transcription.rs`:

```rust
pub enum WhisperModelSize {
    Tiny,
    Base,
    Small,
    Medium,
    Large,
    // Add: TurboLarge,
}
```

## Debugging

### Rust Logs

Backend logs print to terminal when running `bun tauri dev`:

```rust
println!("Debug: {}", value);
eprintln!("Error: {}", error);
```

### Frontend Console

Open DevTools in the app window (Cmd+Option+I) for JavaScript debugging.

### Audio Issues

Check system audio permissions and verify Process Tap is working:
- Look for "nbp-audio-tap" in Audio MIDI Setup
- Check Console.app for Core Audio errors

## Release Process

1. Update version in `package.json`, `Cargo.toml`, and `tauri.conf.json`
2. Run `./build.sh`
3. Find DMG in `builds/nbp_v{version}.dmg`

## Architecture Principles

From `.agent/rules/general.md`:

1. **Local-first** - No network unless explicit
2. **Files over databases** - Grep, rsync just work
3. **Raw audio is immutable** - Never modify source files
4. **Derived content regenerable** - Mix, transcript can be deleted
5. **No telemetry** - Zero tracking
6. **UI never defines data flow** - Backend is authoritative
