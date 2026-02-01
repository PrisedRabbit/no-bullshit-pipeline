# Story 1.5: Anthropic Claude Integration

Status: ready-for-dev

## Story

As a user,
I want to extract structured data using Claude Sonnet,
so that I get high-quality extraction for meeting action items, decisions, and key points.

## Acceptance Criteria

1. **Given** I have a valid Anthropic API key configured
   **When** I select "Extract with Claude"
   **Then** the transcription is processed and structured data is saved as JSON

2. **Given** I select a template type (Meeting Notes, Brainstorm, Journal)
   **When** Claude processes the transcription
   **Then** the output matches the template structure

## Tasks / Subtasks

- [ ] Task 1: Create Anthropic API client module (AC: 1)
  - [ ] Create `src-tauri/src/cloud_ai/anthropic.rs`
  - [ ] Implement Claude Messages API integration
  - [ ] Handle authentication with API key

- [ ] Task 2: Structured extraction prompts (AC: 2)
  - [ ] Create extraction prompt for Meeting Notes structure
  - [ ] Create extraction prompt for Brainstorm structure
  - [ ] Create extraction prompt for Journal structure
  - [ ] Parse Claude response into JSON

- [ ] Task 3: UI integration (AC: 1, 2)
  - [ ] Add Claude/Anthropic option to provider dropdown
  - [ ] Add template selection UI
  - [ ] Save structured output as JSON in recording folder

## Dev Notes

### Architecture Constraints
- Claude 3.5 Sonnet context: 200K tokens
- Best for nuanced extraction and following complex instructions
- API endpoint: `https://api.anthropic.com/v1/messages`

### API Details
```
POST https://api.anthropic.com/v1/messages
Headers:
  x-api-key: {api_key}
  anthropic-version: 2023-06-01
  Content-Type: application/json
Body:
{
  "model": "claude-sonnet-4-20250514",
  "max_tokens": 4096,
  "messages": [{
    "role": "user",
    "content": "Extract structured data from this transcript..."
  }]
}
```

### Extraction Prompt Template (Meeting Notes)
```
Extract structured data from this meeting transcript. Return valid JSON with:
{
  "date": "YYYY-MM-DD",
  "attendees": ["list of people mentioned"],
  "agenda_items": ["topics discussed"],
  "key_decisions": ["decisions made"],
  "action_items": [{"owner": "person", "task": "description", "due": "date if mentioned"}],
  "follow_ups": ["items to follow up on"]
}

Transcript:
{transcript}
```

### Source Tree Components
- `src-tauri/src/cloud_ai/anthropic.rs` - New file
- `src-tauri/src/cloud_ai/mod.rs` - Add anthropic module
- `src-tauri/src/config.rs` - Anthropic API key in ApiKeys
- `src/index.html` - Anthropic option and template selector

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.5]
- [Anthropic API Docs](https://docs.anthropic.com/en/api/messages)

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
