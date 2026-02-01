# Story 2.2: Meeting Notes Template

Status: ready-for-dev

## Story

As a user,
I want a Meeting Notes template,
so that my meeting recordings become structured notes with attendees, agenda, decisions, and action items.

## Acceptance Criteria

1. **Given** I have a meeting transcription
   **When** I apply the Meeting Notes template
   **Then** the output includes: Date, Attendees, Agenda Items, Key Decisions, Action Items (with owners), Follow-ups

2. **Given** the AI cannot identify certain sections
   **When** processing completes
   **Then** those sections are marked as "Not identified" rather than omitted

## Tasks / Subtasks

- [ ] Task 1: Create Meeting Notes template file (AC: 1, 2)
  - [ ] Create `~/.nbp/templates/meeting-notes.json`
  - [ ] Define extraction prompt for all required fields
  - [ ] Handle "Not identified" fallback for missing sections

- [ ] Task 2: Optimize extraction prompt (AC: 1)
  - [ ] Prompt should identify speakers/attendees from context
  - [ ] Extract agenda items from discussion flow
  - [ ] Identify decision language ("we decided", "agreed to", etc.)
  - [ ] Find action items with ownership signals ("I'll", "you should", etc.)

- [ ] Task 3: Output formatting (AC: 1)
  - [ ] Generate clean Markdown output
  - [ ] Include timestamp of extraction
  - [ ] Link back to original transcript

## Dev Notes

### Meeting Notes Template Definition
```json
{
  "name": "Meeting Notes",
  "description": "Extract structured meeting notes with attendees, decisions, and action items",
  "output_format": "markdown",
  "prompt": "Analyze this meeting transcript and extract structured notes.\n\nExtract the following (use 'Not identified' if you cannot determine):\n\n1. **Date**: When the meeting occurred (from context or say 'Not identified')\n2. **Attendees**: List all people mentioned or speaking\n3. **Agenda Items**: Main topics discussed\n4. **Key Decisions**: Any decisions made during the meeting\n5. **Action Items**: Tasks assigned, include owner if mentioned and due date if specified\n6. **Follow-ups**: Items that need future attention\n\nFormat as clean Markdown.\n\nTranscript:\n{transcript}"
}
```

### Source Tree Components
- `~/.nbp/templates/meeting-notes.json` - Template file
- `src-tauri/src/templates.rs` - Template loading
- Depends on: Story 2.1 (Template System Core)

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.2]

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
