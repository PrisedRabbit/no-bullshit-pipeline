# Story 1.2: OpenAI Whisper-1 Transcription

Status: ready-for-dev

## Story

As a user,
I want to transcribe recordings using OpenAI's Whisper-1 API,
so that I get fast, accurate transcriptions without local GPU requirements.

## Acceptance Criteria

1. **Given** I have a valid OpenAI API key configured
   **When** I select "Process" on a recording and choose OpenAI Whisper
   **Then** the audio is sent to Whisper-1 API and transcription is saved

2. **Given** the API call fails (network error, invalid key, quota exceeded)
   **When** processing completes
   **Then** I see a clear error message indicating the failure reason

## Tasks / Subtasks

- [ ] Task 1: Create OpenAI API client module (AC: 1, 2)
  - [ ] Add `reqwest` dependency for HTTP requests
  - [ ] Create `src-tauri/src/cloud_ai/mod.rs` module
  - [ ] Create `src-tauri/src/cloud_ai/openai.rs`
  - [ ] Implement Whisper-1 transcription API call
  - [ ] Handle multipart form upload for audio files

- [ ] Task 2: Integrate with transcription workflow (AC: 1)
  - [ ] Update `transcription.rs` to support cloud providers
  - [ ] Add provider selection logic (LocalWhisper vs OpenAI)
  - [ ] Save transcription result to recording folder

- [ ] Task 3: Error handling and UI feedback (AC: 2)
  - [ ] Handle network errors gracefully
  - [ ] Handle API errors (401 invalid key, 429 rate limit, etc.)
  - [ ] Display user-friendly error messages in UI
  - [ ] Add loading state during API call

## Dev Notes

### Architecture Constraints
- OpenAI Whisper-1 API endpoint: `https://api.openai.com/v1/audio/transcriptions`
- Audio format: Send OGG file directly (already in correct format)
- Max file size: 25MB (Whisper-1 limit)
- Response format: JSON with `text` field

### API Details
```
POST https://api.openai.com/v1/audio/transcriptions
Headers:
  Authorization: Bearer {api_key}
  Content-Type: multipart/form-data
Body:
  file: (audio file)
  model: "whisper-1"
  response_format: "json"
```

### Source Tree Components
- `src-tauri/src/transcription.rs` - Existing transcription logic
- `src-tauri/src/config.rs` - API key retrieval
- `src-tauri/src/lib.rs` - Command registration
- `src/main.js` - Frontend transcription trigger

### Dependencies to Add (Cargo.toml)
- `reqwest = { version = "0.11", features = ["json", "multipart"] }`

### Testing Standards
- Test with valid API key and short audio file
- Test error handling with invalid API key
- Verify transcription saves to correct location

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.2]
- [Source: docs/architecture.md]
- [OpenAI Whisper API Docs](https://platform.openai.com/docs/api-reference/audio/createTranscription)

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
