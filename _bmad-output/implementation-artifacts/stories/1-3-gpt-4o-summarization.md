# Story 1.3: GPT-4o Summarization

Status: ready-for-dev

## Story

As a user,
I want to summarize transcriptions using GPT-4o,
so that I get concise summaries of long recordings.

## Acceptance Criteria

1. **Given** I have a transcription and valid OpenAI API key
   **When** I select "Summarize with GPT-4o"
   **Then** the transcription is sent to GPT-4o and summary is saved as `summary.md`

2. **Given** the transcription is very long
   **When** processing
   **Then** the system handles token limits appropriately (chunking or truncation with warning)

## Tasks / Subtasks

- [ ] Task 1: Implement GPT-4o chat completion API (AC: 1)
  - [ ] Add chat completion function to `cloud_ai/openai.rs`
  - [ ] Create summarization prompt template
  - [ ] Handle API response and extract summary

- [ ] Task 2: Token management for long transcripts (AC: 2)
  - [ ] Calculate approximate token count (4 chars ~= 1 token)
  - [ ] Implement chunking strategy for long transcripts
  - [ ] Combine chunk summaries if needed
  - [ ] Warn user if transcript was truncated

- [ ] Task 3: UI integration (AC: 1)
  - [ ] Add "Summarize" button to detail view
  - [ ] Show summary in structured data section
  - [ ] Save summary as `summary.md` in recording folder

## Dev Notes

### Architecture Constraints
- GPT-4o context window: 128K tokens
- Typical transcript: ~150 words/minute = ~200 tokens/minute
- 1 hour recording = ~12K tokens (usually fits in context)
- For very long recordings, implement rolling summary

### API Details
```
POST https://api.openai.com/v1/chat/completions
Headers:
  Authorization: Bearer {api_key}
  Content-Type: application/json
Body:
  model: "gpt-4o"
  messages: [
    { role: "system", content: "You are a helpful assistant that creates concise summaries..." },
    { role: "user", content: "Summarize this transcript: {transcript}" }
  ]
```

### Summarization Prompt Template
```
Create a concise summary of this transcript. Include:
- Main topics discussed
- Key points and decisions
- Action items if any
- Important quotes or statements

Transcript:
{transcript}
```

### Source Tree Components
- `src-tauri/src/cloud_ai/openai.rs` - Add chat completion
- `src-tauri/src/storage.rs` - Save summary file
- `src/main.js` - Add summarize button handler
- `src/index.html` - Add summarize button to UI

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.3]
- [OpenAI Chat Completions API](https://platform.openai.com/docs/api-reference/chat)

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
