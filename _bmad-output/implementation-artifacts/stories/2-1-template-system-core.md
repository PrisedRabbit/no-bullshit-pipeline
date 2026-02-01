# Story 2.1: Template System Core

Status: ready-for-dev

## Story

As a user,
I want a template system that defines output structures,
so that my transcriptions can be automatically formatted.

## Acceptance Criteria

1. **Given** templates are defined in `~/.nbp/templates/`
   **When** I process a transcription with a template
   **Then** the output follows the template structure

2. **Given** I want to add a custom template
   **When** I create a new `.json` or `.md` template file
   **Then** it appears in the template selection dropdown

## Tasks / Subtasks

- [ ] Task 1: Create template system infrastructure (AC: 1, 2)
  - [ ] Create `src-tauri/src/templates.rs` module
  - [ ] Define `Template` struct with name, description, prompt, output_format
  - [ ] Implement template discovery from `~/.nbp/templates/`
  - [ ] Create default templates directory on first run

- [ ] Task 2: Built-in template definitions (AC: 1)
  - [ ] Create JSON schema for template definition
  - [ ] Bundle default templates (Meeting Notes, Brainstorm, Journal)
  - [ ] Copy defaults to user templates dir if not exists

- [ ] Task 3: Template loading and listing (AC: 2)
  - [ ] Add `list_templates` Tauri command
  - [ ] Add `get_template` Tauri command
  - [ ] Watch templates directory for changes (optional)

- [ ] Task 4: UI integration (AC: 2)
  - [ ] Add template selector dropdown to detail view
  - [ ] Show template description on hover/select
  - [ ] Remember last used template preference

## Dev Notes

### Template JSON Schema
```json
{
  "name": "Meeting Notes",
  "description": "Extract structured meeting notes",
  "output_format": "markdown",
  "prompt": "Extract from this transcript:\n- Attendees\n- Key decisions\n- Action items\n\nTranscript:\n{transcript}",
  "output_schema": {
    "type": "object",
    "properties": {
      "attendees": { "type": "array" },
      "decisions": { "type": "array" },
      "action_items": { "type": "array" }
    }
  }
}
```

### Source Tree Components
- `src-tauri/src/templates.rs` - New module
- `src-tauri/src/lib.rs` - Register template commands
- `src-tauri/src/config.rs` - Get templates directory path
- `src/index.html` - Template selector UI
- `src/main.js` - Template loading logic

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.1]

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
