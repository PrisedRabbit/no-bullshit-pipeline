# Epic 5: Processing Pipelines

## Overview

Transform NBP from a recording + transcription tool into a complete processing pipeline system. Replace flat tags with named pipelines that define ordered sequences of processing steps using built-in connectors (LLM, Save, Webhook) and external integrations via MCP.

**Status:** Not Started
**Priority:** High
**Target Version:** v0.4

## Goals

1. Replace tag-based organization with pipeline-based processing
2. Enable automated multi-step workflows (transcribe → process → deliver)
3. Support external integrations via MCP connector
4. Maintain file-based, debuggable execution model
5. Migrate existing template system to prompt templates

## Success Criteria

- User can create pipeline definitions with multiple steps
- Pipelines auto-execute when transcript becomes available
- All step outputs stored as markdown files with frontmatter
- MCP connector enables Slack, Notion, Telegram, etc. integrations
- Existing templates migrate to prompt templates seamlessly
- Pipeline status visible per recording (waiting/running/done/partial)
- Failed steps can be re-run individually

## User Stories

### Story 5.1: Pipeline Definition Model

**As a developer,**
I want a JSON-based pipeline definition format,
So that pipelines can be stored, loaded, and executed consistently.

**Acceptance Criteria:**

**Given** pipeline definitions exist
**When** application loads
**Then** pipelines are loaded from `~/nbp-data/pipelines.json`

**Given** a pipeline definition includes steps with connectors
**When** definition is validated
**Then** all required fields (name, connector, input, config) are present

**Given** a step references an invalid input
**When** pipeline is validated
**Then** validation error is returned with details

**Technical Requirements:**
- Pipeline JSON schema with name, description, steps array
- Step schema with name, connector, input, config, description
- Validation logic for input references (must be "transcript" or previous step name)
- Support for connector types: llm, save, webhook, mcp

### Story 5.2: Built-in Connector - LLM

**As a user,**
I want to process transcript content with AI prompts,
So that I can extract structured information.

**Acceptance Criteria:**

**Given** a step uses llm connector with valid prompt template
**When** step executes
**Then** transcript or previous step output is sent to configured AI provider
**And** response is saved as `{step-name}.md` with frontmatter

**Given** llm step specifies provider and model in config
**When** step executes
**Then** correct API endpoint and credentials are used

**Given** AI API call fails (network, auth, quota)
**When** step executes
**Then** error is captured in step frontmatter
**And** pipeline status becomes "partial"

**Technical Requirements:**
- Support providers: openai, google, anthropic
- Prompt template variable substitution
- API error handling with retry logic
- Token limit handling (truncation with warning)

### Story 5.3: Built-in Connector - Save

**As a user,**
I want to save step outputs to specific filesystem locations,
So that processed data integrates with external tools (Obsidian, file-based workflows).

**Acceptance Criteria:**

**Given** a step uses save connector with target path
**When** step executes
**Then** input file is copied to target path
**And** variables like {date}, {pipeline-name}, {recording-id} are substituted

**Given** target directory doesn't exist
**When** save step executes
**Then** directory is created recursively

**Given** target file already exists
**When** save step executes
**Then** file is overwritten
**And** warning is logged

**Technical Requirements:**
- Path variable substitution: {date}, {time}, {pipeline-name}, {recording-id}, {step-name}
- Directory creation with proper permissions
- Atomic file writes (write to temp, rename)

### Story 5.4: Built-in Connector - Webhook

**As a user,**
I want to POST step outputs to HTTP endpoints,
So that I can integrate with custom services, Zapier, n8n, etc.

**Acceptance Criteria:**

**Given** a step uses webhook connector with URL and method
**When** step executes
**Then** input file content is sent as HTTP request body
**And** response is saved in step output frontmatter

**Given** webhook returns non-2xx status code
**When** step executes
**Then** error is captured with status code and response body
**And** pipeline status becomes "partial"

**Technical Requirements:**
- Support HTTP methods: POST, PUT, PATCH
- Configurable headers (auth tokens, content-type)
- Timeout configuration (default 30s)
- Request body as JSON or plain text (configurable)
- Response status and body captured in step output

### Story 5.5: MCP Connector

**As a user,**
I want to integrate with third-party services via MCP,
So that I can send data to Slack, Notion, Telegram, Gmail, etc.

**Acceptance Criteria:**

**Given** a step uses mcp connector with server and tool config
**When** step executes
**Then** MCP server is invoked with tool name and arguments
**And** tool response is saved in step output

**Given** MCP server is unavailable
**When** step executes
**Then** error is captured indicating server connectivity issue
**And** pipeline status becomes "partial"

**Given** MCP tool returns error
**When** step executes
**Then** error message from tool is captured
**And** user can inspect error in step output file

**Technical Requirements:**
- MCP protocol implementation for tool invocation
- Server discovery/connection via MCP client library
- Input file content passed as tool argument
- Tool response captured (success/error)
- Server timeout handling

### Story 5.6: Pipeline Execution Engine

**As a system,**
I want to execute pipeline steps sequentially,
So that processing is predictable and debuggable.

**Acceptance Criteria:**

**Given** a recording has assigned pipelines
**When** transcript becomes available
**Then** all pipelines in "waiting" status start execution

**Given** a pipeline is executing
**When** a step completes successfully
**Then** next step starts immediately
**And** step output file is written with status "done"

**Given** a step fails during execution
**When** failure occurs
**Then** execution stops for that pipeline
**And** pipeline status becomes "partial"
**And** subsequent steps remain in "pending" status

**Given** all pipeline steps complete successfully
**When** execution finishes
**Then** pipeline status becomes "done"

**Technical Requirements:**
- Sequential step execution (no parallelization)
- Step status tracking: pending → running → done/failed
- Pipeline status computation from step statuses
- Auto-trigger on transcript availability
- Background execution (non-blocking UI)

### Story 5.7: Prompt Template Registry

**As a user,**
I want to manage reusable prompt templates,
So that I can use consistent prompts across pipelines and recordings.

**Acceptance Criteria:**

**Given** I create a new prompt template
**When** template is saved
**Then** template is stored in `~/nbp-data/prompt-templates.json`
**And** template appears in llm connector config dropdown

**Given** I edit an existing prompt template
**When** changes are saved
**Then** template is updated in registry
**And** new pipeline executions use updated prompt

**Given** I delete a prompt template
**When** deletion is confirmed
**Then** template is removed from registry
**And** pipelines referencing it show error

**Technical Requirements:**
- JSON storage for prompt templates
- CRUD operations: create, read, update, delete
- Template schema: name, description, prompt, created_at, updated_at
- Reference validation (check if template used by pipelines)

### Story 5.8: Built-in Template Migration

**As a developer,**
I want existing templates (Meeting Notes, Brainstorm, Journal) to migrate to prompt templates,
So that users don't lose functionality.

**Acceptance Criteria:**

**Given** application starts for first time with v0.4
**When** migration runs
**Then** three prompt templates are created: meeting-notes, brainstorm, journal
**And** migration flag is set to prevent re-run

**Given** user has recordings with old template metadata
**When** migration runs
**Then** recordings are unaffected (no data loss)

**Technical Requirements:**
- Migration function creates built-in prompt templates
- Templates match previous extraction quality
- One-time migration flag in settings
- Backward compatible metadata reading

### Story 5.9: Recording Pipeline Assignment

**As a user,**
I want to assign pipelines to recordings,
So that processing happens automatically.

**Acceptance Criteria:**

**Given** I view a recording detail
**When** I click "Add Pipeline"
**Then** I see a list of available pipelines
**And** I can select one or more to assign

**Given** I assign a pipeline to a recording with existing transcript
**When** assignment completes
**Then** pipeline starts executing immediately

**Given** I assign a pipeline to a recording without transcript
**When** assignment completes
**Then** pipeline status is "waiting"
**And** pipeline starts when transcript becomes available

**Given** I remove a pipeline from a recording
**When** removal is confirmed
**Then** pipeline directory is not deleted (outputs preserved)
**And** pipeline is no longer tracked in metadata

**Technical Requirements:**
- Metadata field: `pipelines: Vec<String>` (pipeline names)
- Pipeline assignment UI in detail view
- Pipeline status display per recording
- Auto-trigger logic for waiting pipelines

### Story 5.10: Pipeline Status Visualization

**As a user,**
I want to see pipeline execution status,
So that I know what's processing and what failed.

**Acceptance Criteria:**

**Given** I view a recording with assigned pipelines
**When** detail view loads
**Then** I see list of pipelines with status badges (waiting/running/done/partial)

**Given** I click on a pipeline
**When** pipeline detail opens
**Then** I see all steps with individual statuses
**And** I can open step output files

**Given** a step failed
**When** I view step details
**Then** I see error message and can click "Re-run"

**Technical Requirements:**
- Status badges with color coding
- Expandable pipeline view showing steps
- Link to open step output file in editor
- Re-run button for failed steps

### Story 5.11: Step Re-run

**As a user,**
I want to re-run failed pipeline steps,
So that I can recover from transient errors without re-processing entire pipeline.

**Acceptance Criteria:**

**Given** a step failed due to network error
**When** I click "Re-run" on that step
**Then** step executes again using same input
**And** step status updates to "running" → "done" or "failed"

**Given** a step completed successfully
**When** I click "Re-run"
**Then** step re-executes
**And** previous output file is overwritten

**Given** I re-run a step in the middle of a pipeline
**When** step completes successfully
**Then** subsequent steps do NOT auto-run (manual control)

**Technical Requirements:**
- Re-run button per step in UI
- Preserve input file reference
- Overwrite output file atomically
- No automatic cascade to next steps

### Story 5.12: Pipeline Re-run

**As a user,**
I want to re-run entire pipelines,
So that I can reprocess after editing transcript or fixing configuration.

**Acceptance Criteria:**

**Given** a pipeline is in "done" or "partial" status
**When** I click "Re-run Pipeline"
**Then** all steps execute from beginning
**And** existing output files are overwritten

**Given** I edit transcript.md manually
**When** I re-run pipeline
**Then** all steps use updated transcript
**And** outputs reflect changes

**Technical Requirements:**
- Pipeline-level re-run button
- Confirmation dialog (destructive action)
- Sequential re-execution of all steps
- Transcript re-read from filesystem

### Story 5.13: Transcript Format Migration

**As a developer,**
I want transcripts stored as markdown with frontmatter,
So that they follow same format as pipeline outputs.

**Acceptance Criteria:**

**Given** a new transcription completes
**When** transcript is saved
**Then** file format is markdown with YAML frontmatter including source, model, created_at, duration_sec

**Given** existing transcript.md files are plain text
**When** migration runs
**Then** files are converted to frontmatter format
**And** original content is preserved in body

**Technical Requirements:**
- Transcript frontmatter schema
- Migration function for existing files
- Backward compatible reading (handle plain text)

### Story 5.14: Pipeline Management UI

**As a user,**
I want to create and edit pipelines via UI,
So that I don't need to manually edit JSON files.

**Acceptance Criteria:**

**Given** I open Settings → Pipelines
**When** view loads
**Then** I see list of existing pipelines

**Given** I click "Create Pipeline"
**When** form opens
**Then** I can enter name, description, and add steps

**Given** I add a step
**When** I select connector type
**Then** form shows relevant config fields (e.g., prompt template for llm, path for save, URL for webhook, server/tool for mcp)

**Given** I save a new pipeline
**When** save completes
**Then** pipeline appears in list
**And** can be assigned to recordings

**Technical Requirements:**
- Settings view with pipeline CRUD
- Step configuration forms per connector type
- Input reference validation
- JSON serialization to pipelines.json

### Story 5.15: Sidebar Filter by Pipeline

**As a user,**
I want to filter recordings by assigned pipeline,
So that I can quickly find recordings processed by specific workflows.

**Acceptance Criteria:**

**Given** I am in main recordings view
**When** I open sidebar filters
**Then** I see "Pipelines" section with list of pipeline names

**Given** I select a pipeline filter
**When** filter is applied
**Then** only recordings with that pipeline assigned are shown

**Given** I select multiple pipelines
**When** filters are applied
**Then** recordings with ANY of selected pipelines are shown

**Technical Requirements:**
- Extend sidebar filter logic
- Pipeline presence check in metadata
- Multi-select pipeline filter UI

## Technical Architecture

### Data Model

**Pipelines Definition (`~/nbp-data/pipelines.json`):**

```json
{
  "pipeline-name": {
    "name": "Display Name",
    "description": "What this pipeline does",
    "steps": [
      {
        "name": "step-name",
        "connector": "llm|save|webhook|mcp",
        "input": "transcript|step-name",
        "config": { /* connector-specific */ }
      }
    ]
  }
}
```

**Recording Metadata (`metadata.json`):**

```json
{
  "id": "uuid",
  "title": "...",
  "created_at": "...",
  "duration_sec": 123,
  "pipelines": ["pipeline-name-1", "pipeline-name-2"]
}
```

**Step Output (`pipelines/{pipeline-name}/{step-name}.md`):**

```markdown
---
name: step-name
description: "..."
connector: llm
input: transcript
status: done|failed|running|pending
created_at: 2026-02-03T12:00:00Z
completed_at: 2026-02-03T12:00:05Z
error: null
---

Step output content here
```

### Connector Configurations

**LLM:**
```json
{
  "prompt_template": "template-name",
  "provider": "openai|google|anthropic",
  "model": "gpt-4o|gemini-1.5-flash|claude-sonnet-4"
}
```

**Save:**
```json
{
  "path": "~/Documents/{date}-{pipeline-name}.md"
}
```

**Webhook:**
```json
{
  "url": "https://hooks.zapier.com/...",
  "method": "POST",
  "headers": { "Authorization": "Bearer ..." }
}
```

**MCP:**
```json
{
  "server": "slack-mcp",
  "tool": "send-message",
  "args": { "channel": "#team" }
}
```

## Dependencies

- MCP client library (Rust)
- HTTP client for webhook connector (reqwest)
- Async runtime for background pipeline execution (tokio)
- YAML parser for frontmatter (serde_yaml)

## Testing Strategy

- Unit tests for connector implementations
- Integration tests for pipeline execution flow
- E2E tests for UI pipeline management
- Manual testing with real MCP servers (Slack, Notion)

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| MCP server unavailability breaks pipelines | Graceful error capture, step-level re-run |
| Complex pipelines hard to debug | File-based outputs, clear error messages |
| API rate limits cause frequent failures | Retry with exponential backoff, user notification |
| Pipeline config too complex for users | Start with simple UI, provide templates |

## Future Enhancements

- Visual pipeline constructor (drag-and-drop steps)
- Conditional steps (if/else based on step output)
- Parallel step execution (fan-out/fan-in)
- Pipeline templates marketplace (share with community)
- Real-time step execution progress (streaming)
