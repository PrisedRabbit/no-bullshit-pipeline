# NBP Architecture

> Privacy-first audio capture and processing for macOS

## Executive Summary

NBP (No Bullshit Pipeline) is a Tauri 2 desktop application that captures microphone and system audio simultaneously, processes it with professional-grade normalization, and provides local AI transcription via Whisper. The application follows a strict local-first, privacy-by-default architecture.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        NBP Application                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │                    Frontend (src/)                          │     │
│  │  ┌──────────┐  ┌──────────┐  ┌────────────────────────┐   │     │
│  │  │index.html│  │styles.css│  │ main.js + viewManager  │   │     │
│  │  └──────────┘  └──────────┘  └────────────────────────┘   │     │
│  │                                                            │     │
│  │  Views: Recordings List | Detail | Settings | Onboarding  │     │
│  │  State: CSS classes (detail-open, settings-open, etc.)    │     │
│  └────────────────────────────┬───────────────────────────────┘     │
│                               │                                      │
│                    Tauri IPC (invoke/emit)                          │
│                               │                                      │
│  ┌────────────────────────────┴───────────────────────────────┐     │
│  │                    Backend (src-tauri/)                     │     │
│  │                                                             │     │
│  │  ┌─────────────────────────────────────────────────────┐   │     │
│  │  │              Tauri Commands (23 total)               │   │     │
│  │  │  Recording: start/stop/pause/resume                  │   │     │
│  │  │  Storage: list/read/update/delete recordings         │   │     │
│  │  │  Transcription: models/download/transcribe           │   │     │
│  │  │  Config: load/save settings                          │   │     │
│  │  │  Permissions: check/request mic/system audio         │   │     │
│  │  └─────────────────────────────────────────────────────┘   │     │
│  │                                                             │     │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐    │     │
│  │  │ AudioState  │  │   Storage   │  │  Transcription  │    │     │
│  │  │  (Managed)  │  │   Manager   │  │    (Whisper)    │    │     │
│  │  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘    │     │
│  │         │                │                   │             │     │
│  │  ┌──────┴────────────────┴───────────────────┴──────┐     │     │
│  │  │            Audio Processing Pipeline              │     │     │
│  │  │                                                   │     │     │
│  │  │  ┌─────────┐   ┌─────────┐   ┌─────────────┐    │     │     │
│  │  │  │   Mic   │   │ System  │   │  Realtime   │    │     │     │
│  │  │  │ Capture │   │ Capture │   │   Mixer     │    │     │     │
│  │  │  │ (cpal)  │   │ (cidre) │   │             │    │     │     │
│  │  │  └────┬────┘   └────┬────┘   └──────┬──────┘    │     │     │
│  │  │       │             │               │           │     │     │
│  │  │  ┌────┴─────────────┴───────────────┴────┐     │     │     │
│  │  │  │     EBU R128 Normalizer (-23 LUFS)    │     │     │     │
│  │  │  │     + True Peak Limiter (-1 dBTP)     │     │     │     │
│  │  │  └───────────────────┬───────────────────┘     │     │     │
│  │  │                      │                          │     │     │
│  │  │  ┌───────────────────┴───────────────────┐     │     │     │
│  │  │  │      OGG Vorbis Encoder (VBR ~128k)   │     │     │     │
│  │  │  └───────────────────────────────────────┘     │     │     │
│  │  └───────────────────────────────────────────────┘     │     │
│  └─────────────────────────────────────────────────────────┘     │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │   ~/nbp-data/         │
                    │   (File System)       │
                    │                       │
                    │   {uuid}/             │
                    │   ├── raw_mic.ogg     │
                    │   ├── raw_system.ogg  │
                    │   ├── audio_mix.ogg   │
                    │   ├── metadata.json   │
                    │   └── transcript.md   │
                    └───────────────────────┘
```

## Core Modules

### Audio Capture

| Module | File | Technology | Purpose |
|--------|------|------------|---------|
| Mic Audio | `mic_audio.rs` | cpal | Cross-platform microphone capture |
| System Audio | `system_audio.rs` | cidre (Core Audio) | macOS system audio loopback via Process Taps |
| Device Enumeration | `devices.rs` | cpal | Input device listing and selection |

**Key Design Decisions:**
- Uses ring buffers (`ringbuf`) for lock-free audio streaming
- Continuous timeline tracking ensures accurate duration
- Silence padding fills gaps to maintain sync
- Device selection by name (cpal device IDs are unstable)
- Automatic resampling for non-48kHz devices (Bluetooth at 16kHz)

### Audio Processing

| Module | File | Purpose |
|--------|------|---------|
| Normalizer | `normalizer.rs` | EBU R128 loudness normalization to -23 LUFS |
| Mixer | `mixer.rs` | Post-recording mix with resampling |
| Realtime Mixer | `realtime_mixer.rs` | Live mixing during recording |

**Processing Pipeline:**
1. **Capture** → Raw audio from devices
2. **Normalize** → Apply gain + true peak limiting
3. **Encode** → OGG Vorbis (quality 0.4 VBR)
4. **Mix** → Combine mic + system tracks

### Storage

**Pattern:** File-based, no database

```
~/nbp-data/
├── {uuid}/                    # Recording session
│   ├── raw_mic.ogg           # Immutable source
│   ├── raw_system.ogg        # Immutable source
│   ├── audio_mix.ogg         # Derived (regenerable)
│   ├── metadata.json         # Recording metadata
│   └── transcript.md         # Derived (regenerable)
└── projects.json             # Saved tag filters
```

**Invariants:**
1. One recording = one directory
2. Raw files are immutable after recording stops
3. Derived files can be deleted and regenerated
4. `metadata.json` is the source of truth

### Transcription

| Feature | Implementation |
|---------|----------------|
| Engine | whisper-rs with Metal acceleration |
| Models | Tiny (74MB) → Large (2.9GB) |
| Storage | `~/.nbp/models/` |
| Output | Markdown transcript |

**Pipeline:**
1. Convert OGG → 16kHz mono WAV
2. Load Whisper model (Metal GPU)
3. Run inference with greedy decoding
4. Save transcript as `transcript.md`

## State Management

### Backend State (Rust)

```rust
// Tauri managed state
AudioState {
    is_recording: Mutex<bool>,
    mic_recorder: Mutex<Option<MicAudioRecorder>>,
    system_recorder: Mutex<Option<SystemAudioRecorder>>,
    realtime_mixer: Mutex<Option<RealtimeMixer>>,
    current_session: Mutex<Option<RecordingMetadata>>,
    start_timestamp: Mutex<Option<SystemTime>>,
}

PermissionsStateCache(Arc<Mutex<PermissionsState>>)
```

### Frontend State (JavaScript)

```javascript
// Module-level state
let isRecording = false;
let allRecordings = [];
let selectedTags = [];
let selectedRecordingId = null;
let currentRecordingTags = [];
let permissions = { mic: false, system_audio: false };
let appSettings = null;
```

### View State (CSS)

```css
body.detail-open           /* Detail view visible */
body.settings-open         /* Settings view visible */
body.is-recording-active   /* Recording in progress */
body.deep-blue             /* Deep Blue theme applied */
body.deep-obsidian         /* Deep Obsidian theme applied */
```

## IPC Commands

### Recording
- `start_recording(tags: Vec<String>)` → `RecordingMetadata`
- `stop_recording()` → `()`
- `pause_recording()` → `()`
- `resume_recording()` → `()`

### Storage
- `list_recordings()` → `Vec<RecordingMetadata>`
- `read_metadata(recording_id)` → `RecordingMetadata`
- `update_tags(recording_id, tags)` → `()`
- `update_title(recording_id, title)` → `()`
- `delete_recording(recording_id)` → `()`
- `list_projects()` → `Vec<Project>`
- `save_projects(projects)` → `()`

### Transcription
- `get_whisper_models_info()` → `Vec<ModelInfo>`
- `download_whisper_model(size)` → `String`
- `delete_whisper_model(size)` → `()`
- `transcribe_recording(recording_id)` → `String`
- `get_transcript(recording_id)` → `Option<String>`

### Configuration
- `load_settings()` → `AppSettings`
- `save_settings(settings)` → `()`

### Permissions
- `check_permissions(onboarding_completed)` → `PermissionsState`
- `request_mic_permission()` → `bool`
- `request_system_audio_permission()` → `bool`
- `open_privacy_settings(pane)` → `()`

### Audio Devices
- `get_input_devices()` → `Vec<AudioDeviceInfo>`
- `get_audio_level()` → `f32`

## Configuration

### App Settings (`~/.nbp/settings.json`)

```json
{
  "storage_path": "~/nbp-data",
  "auto_discard_seconds": 3,
  "theme": "neon-purple",
  "onboarding_completed": true,
  "selected_microphone": null,
  "transcription": {
    "enabled": false,
    "provider": "LocalWhisper",
    "whisper_model": "Base",
    "api_key": null
  }
}
```

### Tauri Config (`src-tauri/tauri.conf.json`)

- Product: `nbp`
- Version: `0.4.0`
- Identifier: `com.skopanev.nbp`
- Min macOS: `13.0`
- Plugins: `dialog`, `opener`

## Security & Privacy

| Principle | Implementation |
|-----------|----------------|
| Local-first | All data in `~/nbp-data/`, no cloud by default |
| No telemetry | Zero tracking, zero analytics |
| Explicit consent | API keys only when user provides them |
| Permission checks | macOS permissions verified at startup |

## Processing Pipelines

### Pipeline Model

**Core Concept:** Named, ordered sequences of steps that process recording data. Replaces flat tags with actionable workflows.

**Pipeline States:**
- `waiting` - No transcript available yet
- `running` - Steps currently executing
- `done` - All steps completed successfully
- `partial` - Some steps failed, others succeeded

### File-Based Context

All pipeline execution uses filesystem as context. No in-memory state.

```
~/nbp-data/{recording-id}/
├── metadata.json
├── audio_mix.ogg
├── raw_mic.ogg
├── raw_system.ogg
├── transcript.md              # System-level, not pipeline-specific
└── pipelines/
    ├── hltm/
    │   ├── meeting_notes.md   # Step 1 output
    │   ├── action_items.md    # Step 2 output
    │   └── slack.md           # Step 3 delivery status
    └── self/
        ├── structured.md
        └── save.md
```

### Step Format

Every step output is markdown with YAML frontmatter:

```markdown
---
name: meeting_notes
description: "Extract structured meeting notes"
connector: llm
input: transcript
status: done
created_at: 2026-02-03T12:00:00Z
completed_at: 2026-02-03T12:00:05Z
error: null
---

## Meeting Notes
- Decision: Launch in March
- Owner: SK takes frontend
```

### Connectors

**Built-in (3 types):**

| Connector | Purpose | I/O |
|-----------|---------|-----|
| `llm` | AI processing with prompt template | md → md |
| `save` | Copy file to specified path | md → md (status) |
| `webhook` | HTTP POST to URL | md → md (status) |

**External (MCP):**

All third-party integrations use MCP connector:

```yaml
connector: mcp
config:
  server: "slack-mcp"
  tool: "send-message"
  args: { channel: "#team" }
```

### Pipeline Definition

Stored in `~/nbp-data/pipelines.json`:

```json
{
  "hltm": {
    "name": "HLTM Team Meetings",
    "description": "Process team meetings to Slack + Notion",
    "steps": [
      {
        "name": "meeting_notes",
        "connector": "llm",
        "input": "transcript",
        "config": {
          "prompt_template": "meeting-notes-v1",
          "provider": "openai",
          "model": "gpt-4o"
        }
      },
      {
        "name": "action_items",
        "connector": "llm",
        "input": "meeting_notes",
        "config": {
          "prompt_template": "extract-actions",
          "provider": "claude"
        }
      },
      {
        "name": "slack",
        "connector": "mcp",
        "input": "meeting_notes",
        "config": {
          "server": "slack-mcp",
          "tool": "send-message",
          "args": { "channel": "#hltm" }
        }
      }
    ]
  }
}
```

### Prompt Templates

Reusable prompts stored in `~/nbp-data/prompt-templates.json`:

```json
{
  "meeting-notes-v1": {
    "name": "Meeting Notes Extractor",
    "description": "Extract structured meeting notes",
    "prompt": "Extract from transcript:\n- Attendees\n- Key decisions\n- Action items with owners\n\nFormat as markdown."
  }
}
```

### Execution Model

1. **Transcript Dependency:** Pipelines wait until `transcript.md` exists
2. **Sequential Steps:** Execute in definition order, no branching
3. **Input References:** Each step reads from previous step output or transcript
4. **Output Files:** Each step writes `{step-name}.md` in pipeline directory
5. **Error Handling:** Failed steps don't block inspection, mark pipeline as `partial`
6. **Re-run Support:** Individual steps or entire pipelines can be re-run

### Migration from Tags

Existing template system converts to pipelines:

**Before (v0.3):**
- Recording has tags: `["meeting", "team"]`
- User manually applies template after transcription

**After (v0.4 with Pipelines):**
- Recording assigned pipeline: `"hltm"`
- Pipeline auto-executes when transcript appears
- Steps produce files in `pipelines/hltm/`

Built-in templates (Meeting Notes, Brainstorm, Journal) migrate to prompt templates.

## Future Extensibility

**Platform Isolation:**
- `cidre` (Core Audio) is macOS-specific
- Other modules use cross-platform crates
- Potential for Linux (PulseAudio) or Windows (WASAPI) support

**API Integration Points:**
- `TranscriptionProvider` enum supports `LocalWhisper`, `OpenAI`, `Google`
- API key storage ready in settings
- MCP connector enables unlimited third-party integrations

**Pipeline Extensibility:**
- User-defined pipelines via JSON config
- Shareable pipeline templates
- Visual pipeline constructor (future)
