# nbp · TODO

Local voice → structured data. Privacy-first. No bullshit.

## v0.2 · User Experience & Configuration (NEXT)

### 1. Onboarding & Permissions (DONE)

- [x] **First Launch Window**: Clean, high-contrast modal or overlay explaining why we need Mic & System Audio permissions.
- [x] **Permission Gate**: Large buttons to trigger "Request Microphone" and "Request Screen Recording" (for system audio).
- [x] **Interface Warnings**: Subtle indicator (e.g., orange dot or banner) if Mic/S-Audio access is revoked or missing.
- [x] **Permission Help**: Guide links to System Settings -> Privacy & Security.

### 2. Settings View (DONE)

- [x] **General Settings**:
  - [x] **Storage Path**: Custom directory for `nbp-data` (default: `~/nbp-data`).
  - [x] **Clean-up**: Auto-discard recordings shorter than X seconds.
- [x] **Theme Tweak**: Toggle between "Neon Purple" and "Deep Obsidian" (minimalist).

### 3. AI Processing (The "Pipeline")

- [ ] **Local Transcription**:
  - [ ] Integration with `faster-whisper` or `whisper-rs`.
  - [ ] One-click download of Whisper models (Tiny/Base/Small) to local disk.
- [ ] **Cloud API Integrations** (Secure key storage):
  - [ ] **OpenAI**: Whisper-1 (speech-to-text), GPT-4o (summarization).
  - [ ] **Google Gemini**: Flash 1.5 for long-context summary.
  - [ ] **Anthropic Claude**: Sonnet 3.5 for high-quality structured data extraction.
- [ ] **Structured Outpus**:
  - [ ] Define templates (Meeting Notes, Brainstorm, Journal).
  - [ ] Automatic mapping of speech to Markdown/JSON templates.

### 4. Audio Control

- [ ] **In-App Playback**: Simple, sleek audio player in the detail view.
- [ ] **Waveform Preview**: Static or dynamic waveform of the recorded audio.

## Maintenance

- [ ] Proper Error Handling for Audio Mix (avoiding drift).
- [ ] "Entitlements" verification for signed builds.

---

## 6. Explicit Non-Goals

- [ ] Cloud-only storage (Everything must reside locally first).
- [ ] Team collaboration features (This is a personal tool).
- [ ] Heavy Electron-like resource usage (Stay lean with Tauri).
