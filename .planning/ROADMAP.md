# Roadmap: NBP Pipelines v2

## Overview

Eight phases deliver the Pipelines v2 redesign in strict backend-before-frontend order. The Rust infrastructure for Notion integration, prompt augmentation, and the pipeline engine is built first (Phases 1-3) because the frontend cannot correctly render connected integrations or augmented prompts until those backends exist. The frontend layers on top in dependency order: integration settings wizard (Phase 4), pipeline builder redesign (Phase 5), pre-assignment chip UX (Phase 6). Tags-to-pipeline label migration is deferred until the pipeline model is proven stable (Phase 7). The UI health check audits all v2 components last, because it cannot be written until everything it audits exists (Phase 8).

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Notion Integration Infrastructure** - Rust backend for integration profile storage, Keychain credential management, and Notion API client
- [ ] **Phase 2: Notion Connector** - Rust backend for structured JSON parsing, people alias resolution, and Notion page creation
- [ ] **Phase 3: Prompt Augmentation** - Pipeline engine look-ahead that auto-injects schema-derived format specs into LLM prompts
- [ ] **Phase 4: Integrations Settings UI** - Frontend wizard for adding Notion integration with schema sync and people mapping
- [ ] **Phase 5: Pipeline Builder Redesign** - Categorized preset picker, drag-and-drop reordering, and assembly preview replacing the developer-oriented builder
- [ ] **Phase 6: Pre-Assignment UX and Execution** - Pipeline chip bar, one-click recording, run status visibility, and multiple pipeline support
- [ ] **Phase 7: Pipeline Data Model and Tags Migration** - Unified pipeline-as-label model with lazy tags migration
- [ ] **Phase 8: UI Health Check** - Runtime DOM audit on startup with interactive walkthrough on first launch

## Phase Details

### Phase 1: Notion Integration Infrastructure
**Goal**: The app can securely store Notion credentials and read database schemas without exposing any API key in plaintext
**Depends on**: Nothing (first phase)
**Requirements**: NOTN-01, NOTN-02, NOTN-06, NOTN-07, INTG-05
**Success Criteria** (what must be TRUE):
  1. User can add a Notion integration by entering an API key, and the key is stored in macOS Keychain (never written to disk in plaintext)
  2. Integration profile JSON files are written to `~/.nbp/config/integrations/` as separate files, not embedded in `settings.json`
  3. User can trigger a manual schema re-sync and see the updated schema reflected in the stored profile
  4. Dev-mode Keychain bypass is in place so development workflow does not generate repeated macOS permission dialogs
**Plans**: TBD

Plans:
- [ ] 01-01: Add `notion-client` crate dependency; implement `integrations/notion.rs` with API client, `add_notion_integration`, `list_notion_databases`, `sync_notion_schema` commands
- [ ] 01-02: Implement `NotionIntegrationProfile` struct, per-integration JSON storage in `integrations/` directory, `config.rs` helpers, dev-mode Keychain bypass
- [ ] 01-03: Implement `update_notion_people_mappings`, `test_notion_integration`, `remove_notion_integration` commands

### Phase 2: Notion Connector
**Goal**: The pipeline engine can deliver AI-generated structured content to a Notion database with correct property formatting
**Depends on**: Phase 1
**Requirements**: NOTN-03, NOTN-04, NOTN-05, NOTN-08, EXEC-04
**Success Criteria** (what must be TRUE):
  1. A pipeline step with `ConnectorType::Notion` creates a page in the configured Notion database with correct property values
  2. Select property values from LLM output match case-insensitively (LLM outputs "high", Notion receives "High")
  3. People aliases in the integration profile resolve to Notion user IDs before the API call
  4. AI output that is not valid JSON fails with a clear error showing the specific parse failure rather than a generic Notion API error
**Plans**: TBD

Plans:
- [ ] 02-01: Implement `connectors/notion.rs` with `extract_json_array()`, `resolve_people_aliases()`, `resolve_select_value()`; add `jsonschema` crate for AI output validation
- [ ] 02-02: Extend `pipelines.rs` with `ConnectorType::Notion` and `integration_id` field; wire `notion-client` page creation with property type mapping table
- [ ] 02-03: Implement `EXEC-04` property normalization: case-insensitive select match with profile default fallback, people alias to user ID resolution

### Phase 3: Prompt Augmentation
**Goal**: When a pipeline's LLM step feeds a schema-aware connector, the AI is automatically prompted with the exact output format the connector expects
**Depends on**: Phase 1, Phase 2
**Requirements**: AUGM-01, AUGM-02, AUGM-03, AUGM-04, AUGM-05
**Success Criteria** (what must be TRUE):
  1. A pipeline with an LLM step followed by a Notion step produces structured JSON output without the user writing any format instructions
  2. If the integration profile is missing or inaccessible, the pipeline fails before the LLM call with a clear "sync schema in Settings" error (not a cryptic parse failure after an expensive API call)
  3. AI output is validated against the integration profile schema before the Notion API call; invalid output shows raw LLM response alongside the error
  4. The format spec injected into the prompt stays within a reasonable token budget even for Notion databases with many properties
**Plans**: TBD

Plans:
- [ ] 03-01: Implement `build_augmented_prompt()` in `pipeline_engine.rs` with N+1 look-ahead; return `Result<String, String>` (hard fail, no silent fallthrough)
- [ ] 03-02: Implement `build_notion_format_spec()` from integration profile with field relevance filtering; wire validation of AI JSON output against profile schema before delivery

### Phase 4: Integrations Settings UI
**Goal**: Users can connect, configure, and verify integrations (Notion and named save paths) through the app UI without editing any config files
**Depends on**: Phase 1
**Requirements**: INTG-01, INTG-02, INTG-03, INTG-04
**Success Criteria** (what must be TRUE):
  1. The Integrations settings page shows Connected integrations with Test and Remove buttons, and an Available section for adding new ones
  2. User can complete the Notion setup wizard (API key entry, database picker, schema display, people mapping) entirely within the app
  3. The wizard includes an explicit, illustrated step instructing the user to share the integration with their database via the Notion UI; the 404 error handler shows this same instruction
  4. Named save path integrations appear in the Connected section alongside Notion and can be added, renamed, and removed
  5. The delivery step picker in the pipeline builder shows only integrations that appear in the Connected section
**Plans**: TBD

Plans:
- [ ] 04-01: Implement `integrations-settings.js` module; Connected/Available section layout; Test/Remove buttons wired to `test_notion_integration`/`remove_notion_integration` commands
- [ ] 04-02: Implement multi-step Notion setup wizard (API key → DB picker → schema display with last-synced timestamp → people mapping); include mandatory database-sharing instruction step
- [ ] 04-03: Implement named save path integrations UI; wire delivery picker filter to show only connected integrations

### Phase 5: Pipeline Builder Redesign
**Goal**: Users can build pipelines by picking from labeled presets in two categories, with automatic step chaining, drag-and-drop reordering, and a visual assembly preview
**Depends on**: Phase 4
**Requirements**: BLDR-01, BLDR-02, BLDR-03, BLDR-04, BLDR-05, BLDR-06, BLDR-07, BLDR-08
**Success Criteria** (what must be TRUE):
  1. The step picker shows two sections — Processing (AI steps) and Delivery (send somewhere) — with built-in presets that add with no required fields
  2. Adding a Meeting Notes, Action Items, Summary, Structure, or Custom Prompt step creates a correctly configured step without the user filling in connector type, model, or input source
  3. The Custom Prompt step shows one text area; an optional checkbox saves the prompt as a reusable template
  4. Steps can be reordered by dragging; the saved pipeline order matches the visual order after drag
  5. A visual chain preview below the step list shows the full pipeline flow including automatic transcript-to-step-1 and step-N-to-next-step chaining
  6. Provider and model settings are hidden by default and available in a collapsed Advanced section per step
**Plans**: TBD

Plans:
- [ ] 05-01: Extract `pipeline-builder.js` from `main.js`; establish state-first re-render pattern (`pipelineState.steps` as authoritative source, `renderSteps()` full re-render); integrate SortableJS for drag-and-drop
- [ ] 05-02: Implement categorized step picker (Processing / Delivery); built-in processing presets with smart defaults; delivery picker filtered to connected integrations only
- [ ] 05-03: Implement Custom Prompt step with textarea and reusable template checkbox; input chaining toggle; assembly preview; Advanced section with per-step provider/model override

### Phase 6: Pre-Assignment UX and Execution
**Goal**: Users can select a pipeline before recording starts with a single click, see pipeline run status per recording, and assign multiple pipelines to one recording
**Depends on**: Phase 5
**Requirements**: ASGN-01, ASGN-02, ASGN-03, ASGN-04, ASGN-05, ASGN-06, ASGN-07, EXEC-01, EXEC-02, EXEC-03
**Success Criteria** (what must be TRUE):
  1. Pipeline chips appear in the app bar; clicking a chip starts recording immediately with that pipeline pre-assigned — no navigation occurs
  2. The chip bar shows at most 5 pipelines and a visible overflow control for additional pipelines from first render
  3. Chips remain interactive during an active recording for mid-recording pipeline assignment
  4. The last-used pipeline chip is visually highlighted on the next app launch
  5. Recording detail view shows pipeline run status (Waiting / Running / Done / Failed) and inline error for the specific step that failed
  6. After recording stops, transcription and pipeline execution begin automatically with no user action required
**Plans**: TBD

Plans:
- [ ] 06-01: Implement pipeline chip rendering in `main.js`; `startRecordingWithPipeline()` flow; 5-chip cap with overflow popover; chips remain active during recording
- [ ] 06-02: Implement default pipeline setting in Settings > General; last-used pipeline persistence; ASGN-04 post-recording detail view assignment
- [ ] 06-03: Wire auto-transcribe → auto-pipeline execution on recording stop (EXEC-01); surface pipeline run status and per-step failure details in recording detail view (EXEC-02, EXEC-03)

### Phase 7: Pipeline Data Model and Tags Migration
**Goal**: The unified pipeline-as-label mental model is enforced in storage; existing recordings with tags are transparently migrated without data loss
**Depends on**: Phase 6
**Requirements**: PIPE-01, PIPE-02, PIPE-03, PIPE-04, PIPE-05
**Success Criteria** (what must be TRUE):
  1. A pipeline with zero steps functions as a label/tag in all listing and filtering views
  2. Opening a recording that has `tags` in its metadata shows those tags as pipeline labels without any manual migration step
  3. Recording metadata stores pipeline references (by ID), not raw string tags, after migration
  4. Multiple pipelines can be assigned to a single recording; each pipeline's output is written to its own directory under the recording
**Plans**: TBD

Plans:
- [ ] 07-01: Extend `RecordingMetadata` with `pipelines` field; implement `storage::migrate_tags_to_pipeline_labels()` lazy per-recording migration (on access, not batch); retain `tags` field for backward compatibility
- [ ] 07-02: Implement zero-step pipeline support in engine (label-only pipeline that skips execution); wire PIPE-05 multiple pipeline output directory isolation

### Phase 8: UI Health Check
**Goal**: The app automatically verifies that all interactive UI elements are present and functional on every startup, with an optional guided walkthrough for first-time users
**Depends on**: Phase 7
**Requirements**: HLTH-01, HLTH-02, HLTH-03, HLTH-04
**Success Criteria** (what must be TRUE):
  1. On every app startup, a silent DOM audit runs after all async startup calls complete and a status badge appears in the status bar
  2. The health check verifies that all v2 interactive elements (chips, builder, integrations page, detail view) exist and respond to synthetic events
  3. Clicking the health badge when failures exist shows a report with specific element names and suggested fixes
  4. User can trigger the interactive walkthrough on demand from Settings; it also appears automatically on first launch
**Plans**: TBD

Plans:
- [ ] 08-01: Implement `ui-health-check.js` with `runAudit()` returning `{ passed, failed, issues[] }`; defer via `requestIdleCallback` after all startup `invoke()` calls; status bar badge wired to diagnostic report
- [ ] 08-02: Implement Level 2 interactive walkthrough; trigger on first launch via settings flag; wire on-demand trigger in Settings

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Notion Integration Infrastructure | 0/3 | Not started | - |
| 2. Notion Connector | 0/3 | Not started | - |
| 3. Prompt Augmentation | 0/2 | Not started | - |
| 4. Integrations Settings UI | 0/3 | Not started | - |
| 5. Pipeline Builder Redesign | 0/3 | Not started | - |
| 6. Pre-Assignment UX and Execution | 0/3 | Not started | - |
| 7. Pipeline Data Model and Tags Migration | 0/2 | Not started | - |
| 8. UI Health Check | 0/2 | Not started | - |
