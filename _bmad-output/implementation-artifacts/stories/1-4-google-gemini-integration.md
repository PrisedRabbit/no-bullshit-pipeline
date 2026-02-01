# Story 1.4: Google Gemini Integration

Status: ready-for-dev

## Story

As a user,
I want to process long recordings using Google Gemini Flash 1.5,
so that I can summarize hour-long meetings without token limit issues.

## Acceptance Criteria

1. **Given** I have a valid Google API key configured
   **When** I select "Process with Gemini"
   **Then** the transcription is sent to Gemini Flash 1.5 for long-context processing

2. **Given** I have a recording longer than 1 hour
   **When** I process with Gemini
   **Then** the full context is handled without chunking

## Tasks / Subtasks

- [ ] Task 1: Create Google AI client module (AC: 1, 2)
  - [ ] Create `src-tauri/src/cloud_ai/google.rs`
  - [ ] Implement Gemini Flash 1.5 API integration
  - [ ] Handle authentication with API key

- [ ] Task 2: Long-context processing (AC: 2)
  - [ ] Gemini Flash 1.5 supports 1M tokens (huge context)
  - [ ] Send full transcript without chunking
  - [ ] Create appropriate prompt for long-form analysis

- [ ] Task 3: UI integration (AC: 1)
  - [ ] Add Gemini option to provider dropdown
  - [ ] Update settings UI for Google API key
  - [ ] Show processing progress

## Dev Notes

### Architecture Constraints
- Gemini Flash 1.5 context: 1,000,000 tokens
- Ideal for very long transcripts (multi-hour recordings)
- API endpoint: `https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent`

### API Details
```
POST https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={api_key}
Headers:
  Content-Type: application/json
Body:
{
  "contents": [{
    "parts": [{
      "text": "Summarize this transcript: {transcript}"
    }]
  }]
}
```

### Source Tree Components
- `src-tauri/src/cloud_ai/google.rs` - New file
- `src-tauri/src/cloud_ai/mod.rs` - Add google module
- `src-tauri/src/config.rs` - Google API key in ApiKeys
- `src/index.html` - Google option in provider select

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.4]
- [Google Gemini API Docs](https://ai.google.dev/gemini-api/docs)

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
