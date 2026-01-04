# nbp · TODO

Local voice → structured data. Privacy-first. No bullshit.

## v0.3 · The Pipeline & Polish (CURRENT)

### 1. AI Processing (The "Pipeline")

- [x] **Local Transcription**:
  - [x] Integration with `whisper-rs`.
  - [x] One-click download of Whisper models (Tiny/Base/Small/Medium/Large) to local disk.
- [ ] **Cloud API Integrations** (Secure key storage):
  - [ ] **OpenAI**: Whisper-1 (speech-to-text), GPT-4o (summarization).
  - [ ] **Google Gemini**: Flash 1.5 for long-context summary.
  - [ ] **Anthropic Claude**: Sonnet 3.5 for high-quality structured data extraction.
- [ ] **Structured Outpus**:
  - [ ] Define templates (Meeting Notes, Brainstorm, Journal).
  - [ ] Automatic mapping of speech to Markdown/JSON templates.

### 2. Audio Control

- [ ] **In-App Playback**: Simple, sleek audio player in the detail view.
- [ ] **Waveform Preview**: Static or dynamic waveform of the recorded audio.

### 3. v0.2 Release (ARCHIVE - DONE)

- [x] **Onboarding & Permissions**: Clean overlay explaining Mic & System Audio needs.
- [x] **Settings View**: Storage path configuration & theme toggling ("Neon Purple" / "Deep Obsidian").
- [x] **Basic Recording**: Stable mic + system audio mix.

## Maintenance

- [ ] Proper Error Handling for Audio Mix (avoiding drift).
- [ ] "Entitlements" verification for signed builds.

---

## 6. Explicit Non-Goals

- [ ] Cloud-only storage (Everything must reside locally first).
- [ ] Team collaboration features (This is a personal tool).
- [ ] Heavy Electron-like resource usage (Stay lean with Tauri).
- [ ] Add notification to warn everyone that nbp is recording. Option to disable this.
