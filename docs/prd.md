# Product Requirements Document - NBP

**Author:** sk
**Date:** 2026-02-01

## Executive Summary

**NBP (No Bullshit Pipeline)** is a privacy-first audio capture application for macOS that records microphone and system audio simultaneously, processes it with professional-grade normalization, and provides AI-powered transcription and structured output.

**Core Value:** Total Audio Capture → Structured Data. Privacy-first. No bullshit.

**Current State:** v0.3.0 with working core recording, local Whisper transcription, tagging, and themes.

**This PRD covers:** v0.3 "The Pipeline & Polish" — Cloud AI integration, structured output templates, audio playback, and quality improvements.

## Success Criteria

### User Success

1. **Zero-friction capture** — Start recording in <2 seconds, no configuration needed
2. **Accurate transcription** — Cloud AI options (OpenAI, Gemini, Claude) produce usable text without manual correction
3. **Actionable output** — Meeting Notes template extracts attendees, decisions, action items automatically
4. **In-app review** — Play recordings + see waveforms without opening external apps
5. **Privacy confidence** — User knows exactly when data leaves device (API calls require explicit key setup)

### Business Success

(Personal project — "business" = personal productivity value)

1. **Daily driver status** — NBP replaces QuickTime/Voice Memos for all meeting capture
2. **Processing reliability** — 95%+ recordings successfully process to structured output
3. **Feature completion** — All 4 epics (14 stories) from v0.3 roadmap implemented

### Technical Success

1. **Audio integrity** — Zero drift/sync issues in mixed recordings (mic + system)
2. **Error resilience** — Graceful degradation when one source fails
3. **Build health** — Signed DMG passes Gatekeeper without quarantine warnings
4. **API stability** — Cloud transcription handles network failures gracefully

### Measurable Outcomes

| Metric | Target |
|--------|--------|
| Recording success rate | >99% (no crashes, no corruption) |
| Transcription accuracy | Matches source quality (Whisper-1/Gemini handles accents, crosstalk) |
| Template extraction | >80% of sections populated for typical meetings |
| App launch → recording | <3 seconds |

## Product Scope

### MVP - Minimum Viable Product (v0.1-0.2 — DONE)

- [x] Dual-track recording (mic + system audio)
- [x] EBU R128 normalization + real-time mixing
- [x] Local Whisper transcription
- [x] Tag-based organization
- [x] Neon UI with themes
- [x] Permissions onboarding

### Growth Features (v0.3 — IN SCOPE)

**Epic 1: Cloud AI Integration**
- Secure API key storage
- OpenAI Whisper-1 transcription
- GPT-4o summarization
- Google Gemini long-context processing
- Claude structured extraction

**Epic 2: Structured Output Templates**
- Template system core
- Meeting Notes template
- Brainstorm template
- Journal template

**Epic 3: Audio Playback & Visualization**
- In-app audio player
- Waveform preview with seek

**Epic 4: Quality & Polish**
- Audio mix error handling (drift prevention)
- Signed build entitlements
- Recording notification

### Vision (Future)

- Speaker diarization
- Real-time live transcription during recording
- Calendar integration (auto-tag by meeting)
- Export to Notion/Obsidian/etc.

## User Journeys

### Journey 1: Sarah's Weekly Team Standup (Meeting Notes)

**Persona:** Sarah, product manager who runs daily standups over Zoom. Tired of taking notes while trying to participate.

**Opening Scene:** Sarah joins her Monday standup. She used to frantically type while team members talked, missing context and misattributing action items.

**Rising Action:**
1. Opens NBP before joining Zoom
2. Clicks record — both mic and Zoom system audio captured
3. Participates fully in the meeting without note-taking
4. 30 minutes later, clicks stop
5. Sees "Processing..." as audio mixes in real-time

**Climax:** Sarah clicks "Process" and selects "Meeting Notes" template. GPT-4o extracts attendees (from voice identification context), key decisions, and action items with owners.

**Resolution:** In 2 minutes, Sarah has structured notes ready to paste into Notion. She catches an action item she would have missed while typing. Team gets meeting recap within 10 minutes of call ending.

**Journey Requirements:**
- Start/stop recording with minimal friction
- Real-time audio mixing (no post-processing wait)
- Cloud AI processing (GPT-4o)
- Meeting Notes template with structured extraction
- Copy-friendly output format

---

### Journey 2: Alex's Product Brainstorm (Brainstorm Template)

**Persona:** Alex, solo founder who thinks best out loud. Has ideas in the shower, on walks, at 2am.

**Opening Scene:** Alex is pacing their apartment at midnight, breakthrough idea forming but too scattered to write coherently.

**Rising Action:**
1. Grabs laptop, opens NBP
2. Records 45-minute stream-of-consciousness brainstorm
3. Jumps between topics, contradicts self, has tangents
4. Finally exhausted, stops recording

**Climax:** Next morning, Alex processes with "Brainstorm" template. Claude structures the chaos: core idea, supporting themes, contradictions noted, top 3 priorities surfaced.

**Resolution:** Alex has a clear starting point for a product spec. The "contradictions" section reveals an assumption they need to validate. The 45 minutes of rambling became 2 pages of structured thinking.

**Journey Requirements:**
- Long-form recording (45+ min)
- Works with mic-only (no system audio needed)
- Brainstorm template extracts themes, priorities
- Claude integration for nuanced extraction

---

### Journey 3: Recording Failure Recovery (Edge Case)

**Persona:** David, user in important client call when system audio fails mid-recording.

**Opening Scene:** David is 20 minutes into recording a crucial client call. System audio source disconnects (Zoom crashed and restarted).

**Rising Action:**
1. NBP detects system audio loss
2. **Notification appears:** "System audio lost. Mic recording continues."
3. David sees indicator but stays focused on call
4. Call ends, David stops recording

**Climax:** Recording list shows warning icon. David opens detail view, sees "Issues occurred during capture" indicator. Plays back — mic audio is intact, system audio partial.

**Resolution:** David processes with local Whisper (his voice only, but enough context). Gets 80% of the conversation. Makes note to follow up on client-side items he couldn't hear.

**Journey Requirements:**
- Robust error detection during recording
- Continue recording on partial failure
- Clear visual indicators of issues
- Graceful degradation (process what's available)

---

### Journey 4: First-Time Setup (Onboarding)

**Persona:** New user who just installed NBP.

**Opening Scene:** Downloads NBP from website, opens the DMG, drags to Applications.

**Rising Action:**
1. Launches app — permissions overlay appears
2. Grants microphone permission (system prompt)
3. Grants Screen Recording permission (for system audio)
4. Onboarding shows status: both permissions granted
5. Clicks "Continue" to main app

**Climax:**
- Goes to Settings → Transcription
- Enables transcription, downloads Base model (141MB)
- Test recording: says "Testing, 1, 2, 3"
- Processes — sees transcript appear

**Resolution:** User is confident the app works. Optional: enters OpenAI API key for cloud transcription. Ready for first real recording.

**Journey Requirements:**
- Clear permission onboarding flow
- Model download with progress indication
- Quick test path to validate setup
- API key configuration in settings

---

### Journey Requirements Summary

| Journey | Key Capabilities Required |
|---------|---------------------------|
| Meeting Notes | Cloud AI (GPT-4o), Meeting Notes template, real-time mixing |
| Brainstorm | Long recording, Brainstorm template, Claude integration |
| Failure Recovery | Error detection, notifications, graceful degradation |
| Onboarding | Permission flow, model download, settings UI |

## Innovation & Novel Patterns

### Detected Innovation Areas

1. **System Audio Loopback** — Using macOS Core Audio Process Taps to capture system audio (Zoom, FaceTime, browser) alongside mic. Most consumer recording apps lack this capability.

2. **Privacy-First AI** — Local Whisper (Metal-accelerated) as default. No cloud dependency. API keys are opt-in, making air-gapped operation possible.

3. **Structured Output Pipeline** — AI doesn't just transcribe; it extracts structure (Meeting Notes, Brainstorm, Journal templates). Transforms audio → actionable artifacts.

4. **Hybrid Processing Model** — User controls the privacy/accuracy tradeoff:
   - Local Whisper: Fast, private, good enough
   - Cloud Whisper-1: Accurate, handles accents
   - Gemini Flash 1.5: Long-context (1hr+ recordings)
   - Claude: Nuanced extraction

### Competitive Landscape

| Competitor | System Audio | Local AI | Structured Output | Privacy-First |
|------------|--------------|----------|-------------------|---------------|
| Otter.ai | No | No | Yes | No (cloud-only) |
| Fireflies | No | No | Yes | No (cloud-only) |
| Voice Memos | No | No | No | Yes |
| QuickTime | No | No | No | Yes |
| **NBP** | **Yes** | **Yes** | **Yes** | **Yes** |

### Validation Approach

- **System Audio**: Already validated in v0.1-0.2 (Core Audio Process Taps working)
- **Local Whisper**: Already validated in v0.3 (transcription working)
- **Cloud AI**: Validate API integrations with OpenAI/Google/Anthropic
- **Templates**: Validate extraction quality with real meeting recordings

### Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Process Taps API changes | Monitor macOS releases, maintain compatibility layer |
| Cloud AI rate limits | Graceful degradation to local Whisper |
| Template extraction quality | Iterative prompt engineering, user feedback loop |

## Desktop App Specific Requirements

### Project-Type Overview

NBP is a **native macOS desktop application** built with Tauri 2. It leverages platform-specific APIs (Core Audio Process Taps) that are not available on other platforms, making cross-platform support a non-goal for v0.3.

**Platform Focus:** macOS 13.0+ (Ventura and later)

### Platform Support

| Attribute | Value |
|-----------|-------|
| Minimum OS | macOS 13.0 (Ventura) |
| Architecture | Universal (Apple Silicon + Intel) |
| Framework | Tauri 2.9.5 |
| Backend | Rust 2024 Edition |
| Frontend | Vanilla HTML/JS/CSS |

**Why macOS-only:**
- Core Audio Process Taps (system audio loopback) require macOS 13+
- cidre crate is macOS-specific (Core Audio bindings)
- No equivalent API on Windows/Linux for low-latency system audio capture

### System Integration

**Required Permissions:**

| Permission | Purpose | Grant Method |
|------------|---------|--------------|
| Microphone | Voice capture | System prompt on first use |
| Screen Recording | System audio via Process Taps | System Preferences (manual) |

**System APIs Used:**

| API | Crate | Purpose |
|-----|-------|---------|
| Core Audio Process Taps | cidre | System audio loopback capture |
| Audio Units | cpal | Microphone input capture |
| Metal | whisper-rs | GPU-accelerated Whisper inference |
| Keychain | (future) | Secure API key storage |

### Update Strategy

**Current (v0.3):**
- Manual download from distribution site
- DMG-based installation
- User replaces existing app manually

**Future Roadmap:**
- Sparkle framework for auto-updates
- Delta updates for bandwidth efficiency
- Update notifications in-app

### Offline Capabilities

NBP is designed for **air-gapped operation**:

| Feature | Offline | Online |
|---------|---------|--------|
| Recording | Full functionality | Full functionality |
| Local Whisper transcription | Full functionality | Full functionality |
| Cloud AI transcription | Requires API keys + network | Requires API keys |
| Model download | Requires network | One-time download |

**Data Storage:**
- All recordings stored locally in `~/nbp-data/`
- Settings stored locally in `~/.nbp/settings.json`
- Whisper models stored locally in `~/.nbp/models/`
- No cloud sync, no telemetry, no analytics

### Implementation Considerations

**Tauri 2 Specifics:**
- Uses `tauri-plugin-dialog` for file dialogs
- Uses `tauri-plugin-opener` for opening folders
- IPC via `invoke()` pattern
- Event emitting for async updates (download progress, transcription segments)

**Build & Distribution:**
- `./build.sh` creates signed DMG
- Entitlements required: microphone, screen recording
- App notarization recommended for distribution

## Project Scoping & Phased Development

### MVP Strategy & Philosophy

**MVP Approach:** Problem-Solving MVP
- Core problem: Capture total audio (mic + system) and produce actionable output
- Validated: Users can record, transcribe locally, and organize with tags
- v0.3 extends with cloud AI and structured output

**Resource Requirements:**
- Solo developer (sk)
- No external dependencies for core functionality
- Cloud APIs require user-provided keys only

### MVP Feature Set (Phase 1 — COMPLETED)

**Core User Journeys Supported:**
- Basic recording (Journey 1 & 2 partial)
- First-time setup (Journey 4)

**Must-Have Capabilities (DONE):**
- [x] Dual-track recording (mic + system audio)
- [x] Real-time audio mixing
- [x] EBU R128 normalization
- [x] Local Whisper transcription
- [x] Tag-based organization
- [x] Settings persistence
- [x] Permission onboarding

### Growth Features (Phase 2 — v0.3 IN PROGRESS)

**Core User Journeys Completed:**
- Meeting Notes (Journey 1) — requires Cloud AI + templates
- Brainstorm (Journey 2) — requires templates
- Failure Recovery (Journey 3) — requires error handling

**Planned Capabilities:**

| Epic | Features | Priority |
|------|----------|----------|
| 1: Cloud AI | API key storage, OpenAI, Gemini, Claude | High |
| 2: Templates | Meeting Notes, Brainstorm, Journal | High |
| 3: Playback | Audio player, waveform visualization | Medium |
| 4: Polish | Error handling, signed builds, notifications | Medium |

### Expansion Features (Phase 3 — Future)

**Vision Capabilities:**
- Speaker diarization (identify who said what)
- Real-time live transcription during recording
- Calendar integration (auto-tag meetings)
- Export to Notion/Obsidian/other tools
- Cross-platform (if demand exists)

### Risk Mitigation Strategy

**Technical Risks:**

| Risk | Mitigation |
|------|------------|
| Core Audio API changes | Monitor Apple releases, maintain compatibility |
| Cloud API rate limits | Graceful fallback to local Whisper |
| Model download failures | Retry logic, clear error messages |

**Market Risks:**

| Risk | Mitigation |
|------|------------|
| Competitors add system audio | Privacy-first + local AI is differentiator |
| Apple removes Process Taps | Monitor deprecation notices |

**Resource Risks:**

| Risk | Mitigation |
|------|------------|
| Solo developer bandwidth | Prioritize epics by user value |
| Feature creep | Stick to v0.3 scope, defer to Vision |

## Functional Requirements

### Audio Capture

- FR1: User can start recording mic audio with a single click
- FR2: User can start recording system audio (Zoom, FaceTime, browser) simultaneously with mic
- FR3: System can capture system audio via macOS Process Taps
- FR4: User can stop recording with a single click
- FR5: User can pause and resume an active recording
- FR6: System continues mic recording if system audio source fails mid-recording

### Audio Processing

- FR7: System can normalize recorded audio to EBU R128 standard (-23 LUFS)
- FR8: System can apply true peak limiting (-1 dBTP) to prevent clipping
- FR9: System can mix mic and system audio tracks in real-time during recording
- FR10: System can encode audio to OGG Vorbis format (VBR ~128kbps)
- FR11: System can detect and compensate for sample rate drift between audio sources

### Transcription

- FR12: User can transcribe recordings using local Whisper models
- FR13: User can select Whisper model size (Tiny, Base, Small, Medium, Large)
- FR14: User can download Whisper models with progress indication
- FR15: User can delete downloaded Whisper models
- FR16: User can transcribe recordings using OpenAI Whisper-1 API
- FR17: User can summarize transcriptions using GPT-4o
- FR18: User can process long recordings (1hr+) using Google Gemini Flash 1.5
- FR19: User can extract structured data using Anthropic Claude

### Structured Output

- FR20: User can apply output templates to transcriptions
- FR21: User can select "Meeting Notes" template for meeting recordings
- FR22: System extracts attendees, key decisions, action items from Meeting Notes template
- FR23: User can select "Brainstorm" template for ideation sessions
- FR24: System extracts themes, priorities, contradictions from Brainstorm template
- FR25: User can select "Journal" template for personal recordings
- FR26: System extracts mood, key thoughts, reflections from Journal template
- FR27: User can create custom templates in `~/.nbp/templates/`

### Audio Playback

- FR28: User can play recordings directly in the app
- FR29: User can pause playback
- FR30: User can seek to any position using a seek bar
- FR31: User can see waveform visualization of recordings
- FR32: User can click on waveform to seek to that position
- FR33: System shows playhead position on waveform during playback
- FR34: System stops playback when switching to different recording

### User Settings

- FR35: User can configure storage path for recordings
- FR36: User can set auto-discard threshold for short recordings
- FR37: User can select UI theme (Neon Purple, Deep Obsidian)
- FR38: User can enable/disable transcription feature
- FR39: User can select transcription provider (Local Whisper, OpenAI, Google, Anthropic)
- FR40: User can store API keys for cloud services
- FR41: System masks API keys in UI after saving

### Recording Management

- FR42: User can view list of all recordings
- FR43: User can filter recordings by tags
- FR44: User can add tags to recordings
- FR45: User can remove tags from recordings
- FR46: User can edit recording title
- FR47: User can delete recordings
- FR48: User can open recording folder in Finder
- FR49: User can view recording metadata (date, duration, tags)
- FR50: System suggests tags based on usage frequency

### Notifications & Feedback

- FR51: User can enable/disable recording start notification
- FR52: System displays notification when recording is active (if enabled)
- FR53: System shows visual indicator when recording is in progress
- FR54: System shows warning indicator if issues occurred during capture
- FR55: System displays clear error messages for API failures
- FR56: System shows progress indicator during model download
- FR57: System shows processing indicator during transcription

## Non-Functional Requirements

### Performance

- NFR1: App launches to ready state within 3 seconds
- NFR2: Recording starts within 500ms of user clicking "Record"
- NFR3: Recording stops and file writes within 1 second of user clicking "Stop"
- NFR4: Real-time audio mixing adds <10ms latency
- NFR5: Local Whisper transcription processes at >=1x real-time on Apple Silicon
- NFR6: UI remains responsive during transcription (no blocking)
- NFR7: Waveform visualization renders within 2 seconds for recordings up to 1 hour

### Security & Privacy

- NFR8: API keys stored in macOS Keychain (not plaintext)
- NFR9: API keys never logged or transmitted except to their designated service
- NFR10: No telemetry, analytics, or usage data collected
- NFR11: All recordings stored locally with user-controlled paths
- NFR12: No cloud sync or backup of recordings
- NFR13: Settings file readable only by current user (600 permissions)

### Reliability

- NFR14: Recording continues if system audio source fails (mic-only fallback)
- NFR15: Recording continues if network connection lost
- NFR16: Zero data loss on app crash during recording (auto-save to temp)
- NFR17: Corrupt or partial audio files handled gracefully (no app crash)
- NFR18: Cloud API failures display clear error messages and preserve local state
- NFR19: Model download can be resumed if interrupted

### Integration

- NFR20: OpenAI API calls use standard REST with proper authentication
- NFR21: Google AI SDK follows official client library patterns
- NFR22: Anthropic API calls use standard REST with proper authentication
- NFR23: Cloud API timeouts configurable with sensible defaults (30s)
- NFR24: Network errors don't block UI (async with cancellation)
- NFR25: API response parsing fails gracefully with user-friendly messages

### Usability

- NFR26: All primary actions accessible via keyboard shortcuts
- NFR27: Visual feedback for all state changes (recording, processing, complete)
- NFR28: Error messages actionable (include what went wrong and what to do)
- NFR29: Theme colors maintain sufficient contrast for readability
