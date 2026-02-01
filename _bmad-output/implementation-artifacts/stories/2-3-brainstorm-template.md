# Story 2.3: Brainstorm Template

Status: ready-for-dev

## Story

As a user,
I want a Brainstorm template,
so that my ideation sessions become organized lists of ideas with categories and priorities.

## Acceptance Criteria

1. **Given** I have a brainstorm session transcription
   **When** I apply the Brainstorm template
   **Then** the output includes: Topic, Ideas (grouped by theme), Top 3 Priorities, Next Steps

## Tasks / Subtasks

- [ ] Task 1: Create Brainstorm template file (AC: 1)
  - [ ] Create `~/.nbp/templates/brainstorm.json`
  - [ ] Define extraction prompt for ideation content
  - [ ] Group ideas by emerging themes

- [ ] Task 2: Optimize extraction prompt (AC: 1)
  - [ ] Identify the core topic being brainstormed
  - [ ] Extract all ideas mentioned
  - [ ] Cluster similar ideas into themes
  - [ ] Identify priority signals ("this is important", "key thing", etc.)
  - [ ] Extract next steps or follow-up actions

- [ ] Task 3: Output formatting (AC: 1)
  - [ ] Generate organized Markdown with sections
  - [ ] Number ideas within each theme
  - [ ] Highlight top priorities

## Dev Notes

### Brainstorm Template Definition
```json
{
  "name": "Brainstorm",
  "description": "Organize ideation sessions into themed ideas with priorities",
  "output_format": "markdown",
  "prompt": "Analyze this brainstorm session and organize the ideas.\n\nExtract and organize:\n\n1. **Topic**: What is being brainstormed\n2. **Ideas by Theme**: Group all ideas into logical themes/categories\n3. **Top 3 Priorities**: The most important or emphasized ideas\n4. **Next Steps**: Any action items or follow-ups mentioned\n\nFormat as clean Markdown with clear sections.\n\nTranscript:\n{transcript}"
}
```

### Source Tree Components
- `~/.nbp/templates/brainstorm.json` - Template file
- Depends on: Story 2.1 (Template System Core)

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.3]

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
