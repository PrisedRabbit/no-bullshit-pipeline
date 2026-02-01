# Story 2.4: Journal Template

Status: ready-for-dev

## Story

As a user,
I want a Journal template,
so that my voice journal entries become formatted diary entries.

## Acceptance Criteria

1. **Given** I have a journal recording
   **When** I apply the Journal template
   **Then** the output includes: Date, Mood (inferred), Key Thoughts, Reflections, Gratitude items

## Tasks / Subtasks

- [ ] Task 1: Create Journal template file (AC: 1)
  - [ ] Create `~/.nbp/templates/journal.json`
  - [ ] Define extraction prompt for personal reflection content
  - [ ] Infer mood from language and tone

- [ ] Task 2: Optimize extraction prompt (AC: 1)
  - [ ] Detect emotional tone and mood indicators
  - [ ] Extract main thoughts and ideas
  - [ ] Identify reflective statements
  - [ ] Find gratitude expressions ("thankful", "grateful", "appreciate")
  - [ ] Preserve personal voice in output

- [ ] Task 3: Output formatting (AC: 1)
  - [ ] Generate warm, personal Markdown format
  - [ ] Use first-person voice in summaries
  - [ ] Include mood emoji indicator

## Dev Notes

### Journal Template Definition
```json
{
  "name": "Journal",
  "description": "Transform voice journals into formatted diary entries",
  "output_format": "markdown",
  "prompt": "Transform this voice journal entry into a formatted diary entry.\n\nExtract and format:\n\n1. **Date**: Today's date or mentioned date\n2. **Mood**: Infer the overall emotional tone (e.g., Reflective, Energetic, Grateful, Anxious, Peaceful)\n3. **Key Thoughts**: Main ideas or events discussed\n4. **Reflections**: Any insights or realizations\n5. **Gratitude**: Things the speaker expressed thanks for (if any)\n\nWrite in first person, preserving the personal voice. Format as a warm, readable diary entry.\n\nTranscript:\n{transcript}"
}
```

### Mood Detection Signals
- Positive: "happy", "excited", "grateful", "love"
- Reflective: "thinking about", "wondering", "realized"
- Anxious: "worried", "stressed", "concerned"
- Peaceful: "calm", "relaxed", "content"

### Source Tree Components
- `~/.nbp/templates/journal.json` - Template file
- Depends on: Story 2.1 (Template System Core)

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.4]

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
