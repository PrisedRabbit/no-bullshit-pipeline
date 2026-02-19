# Roadmap: NBP Pipelines v2

## Overview

Eight phases deliver the Pipelines v2 redesign in strict backend-before-frontend order. The Rust infrastructure for Notion integration, prompt augmentation, and the pipeline engine is built first (Phases 1-3) because the frontend cannot correctly render connected integrations or augmented prompts until those backends exist. The frontend layers on top in dependency order: integration settings wizard (Phase 4), pipeline builder redesign (Phase 5), pre-assignment chip UX (Phase 6). Tags-to-pipeline label migration is deferred until the pipeline model is proven stable (Phase 7). The UI health check audits all v2 components last, because it cannot be written until everything it audits exists (Phase 8).

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Notion Integration Infrastructure** - Rust backend for integration profile storage, Keychain credential management, and Notion API client
- [x] **Phase 2: Notion Connector** - Rust backend for structured JSON parsing, people alias resolution, and Notion page creation (completed 2026-02-18)
- [x] **Phase 3: Prompt Augmentation** - Pipeline engine look-ahead that auto-injects schema-derived format specs into LLM prompts (completed 2026-02-18)
- [x] **Phase 4: Integrations Settings UI** - Frontend wizard for adding Notion integration with schema sync and people mapping (completed 2026-02-18)
- [x] **Phase 5: Pipeline Builder Redesign** - Categorized preset picker, drag-and-drop reordering, and assembly preview replacing the developer-oriented builder (completed 2026-02-19)
- [x] **Phase 6: Pre-Assignment UX and Execution** - Pipeline chip bar, one-click recording, run status visibility, and multiple pipeline support (completed 2026-02-19)
- [x] **Phase 7: Pipeline Data Model and Tags Migration** - Unified pipeline-as-label model with lazy tags migration (completed 2026-02-19)
- [ ] **Phase 8: UI Health Check** - Runtime DOM audit on startup with interactive walkthrough on first launch

## Phase Details

### Phase 1: Notion Integration Infrastructure
**Goal**: The app can securely store Notion credentials and read database schemas without exposing any API key in plaintext
**Depends on**: Nothing (first phase)
**Requirements**: NOTN-01, NOTN-02, NOTN-06, NOTN-07, INTG-05
**Success Criteria** (what must be TRUE):
  1. User can add a Notion integration by entering an API key, and the key is stored in macOS Keychain (never written to disk in plaintext)
  2. Integration profile JSON files are written to `~/.nbp/integrations/` as separate files, not embedded in `settings.json`
  3. User can trigger a manual schema re-sync and see the updated schema reflected in the stored profile
  4. Dev-mode Keychain bypass is in place so development workflow does not generate repeated macOS permission dialogs
**Plans**: 3 plans

Plans:
- [x] 01-01-PLAN.md — Module restructuring, notion-client dependency, dev-mode Keychain bypass, Notion profile types and I/O
- [x] 01-02-PLAN.md — Core Notion commands: add_notion_integration, list_notion_databases, sync_notion_schema
- [x] 01-03-PLAN.md — Secondary Notion commands: update_notion_people_mappings, test_notion_integration, remove_notion_integration

### Phase 2: Notion Connector
**Goal**: The pipeline engine can deliver AI-generated structured content to a Notion database with correct property formatting
**Depends on**: Phase 1
**Requirements**: NOTN-08, EXEC-04
**Success Criteria** (what must be TRUE):
  1. A pipeline step with `ConnectorType::Notion` creates a page in the configured Notion database with correct property values
  2. Select property values from LLM output match case-insensitively (LLM outputs "high", Notion receives "High")
  3. People aliases in the integration profile resolve to Notion user IDs before the API call
  4. AI output that is not valid JSON fails with a clear error showing the specific parse failure rather than a generic Notion API error
**Plans**: 2 plans

Plans:
- [x] 02-01-PLAN.md — Create `connectors/notion.rs` with execute(), extract_json_array(), resolve_people_aliases(), resolve_select_value(), build_notion_properties(); register in connectors/mod.rs
- [ ] 02-02-PLAN.md — Add ConnectorType::Notion to pipelines.rs with validation; wire match arm in pipeline_engine.rs; add unit tests

### Phase 3: Prompt Augmentation
**Goal**: When a pipeline's LLM step feeds a schema-aware connector, the AI is automatically prompted with the exact output format the connector expects
**Depends on**: Phase 1, Phase 2
**Requirements**: AUGM-01, AUGM-02, AUGM-03, AUGM-04, AUGM-05
**Success Criteria** (what must be TRUE):
  1. A pipeline with an LLM step followed by a Notion step produces structured JSON output without the user writing any format instructions
  2. If the integration profile is missing or inaccessible, the pipeline fails before the LLM call with a clear "sync schema in Settings" error (not a cryptic parse failure after an expensive API call)
  3. AI output is validated against the integration profile schema before the Notion API call; invalid output shows raw LLM response alongside the error
  4. The format spec injected into the prompt stays within a reasonable token budget even for Notion databases with many properties
**Plans**: 2 plans

Plans:
- [ ] 03-01-PLAN.md — N+1 look-ahead in pipeline engine, build_augmented_prompt(), build_notion_format_spec(), LLM connector augmented_prompt parameter
- [ ] 03-02-PLAN.md — validate_llm_output_for_notion() wired into Notion connector execute() flow; strengthen error messages with raw LLM output

### Phase 4: Integrations Settings UI
**Goal**: Users can connect, configure, and verify integrations (Notion and named save paths) through the app UI without editing any config files
**Depends on**: Phase 1
**Requirements**: INTG-01, INTG-02, INTG-03, INTG-04, NOTN-03, NOTN-04, NOTN-05
**Success Criteria** (what must be TRUE):
  1. The Integrations settings page shows Connected integrations with Test and Remove buttons, and an Available section for adding new ones
  2. User can complete the Notion setup wizard (API key entry, database picker, schema display, people mapping) entirely within the app
  3. The wizard includes an explicit, illustrated step instructing the user to share the integration with their database via the Notion UI; the 404 error handler shows this same instruction
  4. Named save path integrations appear in the Connected section alongside Notion and can be added, renamed, and removed
  5. The delivery step picker in the pipeline builder shows only integrations that appear in the Connected section
**Plans**: 3 plans

Plans:
- [ ] 04-01-PLAN.md — Expose list_notion_profiles command; create integrations-settings.js module with Connected/Available layout; Test/Remove buttons for Notion and Slack
- [ ] 04-02-PLAN.md — Multi-step Notion setup wizard (API key → share instruction → DB picker → schema display → people mapping) as modal overlay
- [ ] 04-03-PLAN.md — Save path integration Rust backend and UI; wire pipeline builder Notion connector to show connected integrations

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
**Plans**: 3 plans

Plans:
- [ ] 05-01-PLAN.md — Extract pipeline-builder.js from main.js with SortableJS drag-and-drop replacing native HTML5 DnD
- [ ] 05-02-PLAN.md — Categorized step picker (Processing/Delivery) with built-in presets and new backend templates
- [ ] 05-03-PLAN.md — Custom Prompt form with save-as-template, enhanced assembly preview, Advanced section, prompt_inline backend support

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
**Plans**: 3 plans

Plans:
- [ ] 06-01-PLAN.md — Pipeline chip bar in app bar with renderPipelineChips(), startRecordingWithPipeline(), 5-chip cap with overflow popover, mid-recording assignment
- [ ] 06-02-PLAN.md — Default pipeline and last-used pipeline in AppSettings; Audio tab dropdown; detail view pipeline assignment dropdown
- [ ] 06-03-PLAN.md — Auto-transcribe + auto-execute on recording stop; pipeline run status badges in detail view; failed step inline error display

### Phase 7: Pipeline Data Model and Tags Migration
**Goal**: The unified pipeline-as-label mental model is enforced in storage; existing recordings with tags are transparently migrated without data loss
**Depends on**: Phase 6
**Requirements**: PIPE-01, PIPE-02, PIPE-03, PIPE-04, PIPE-05
**Success Criteria** (what must be TRUE):
  1. A pipeline with zero steps functions as a label/tag in all listing and filtering views
  2. Opening a recording that has `tags` in its metadata shows those tags as pipeline labels without any manual migration step
  3. Recording metadata stores pipeline references (by ID), not raw string tags, after migration
  4. Multiple pipelines can be assigned to a single recording; each pipeline's output is written to its own directory under the recording
**Plans**: 2 plans

Plans:
- [ ] 07-01-PLAN.md — Move PipelineState types to pipelines.rs, add typed `pipelines` field to RecordingMetadata, implement lazy tag-to-pipeline-label migration, remove zero-step validation guard
- [ ] 07-02-PLAN.md — Zero-step early return in pipeline engine (label-only pipeline skips execution), remove frontend zero-step guard in pipeline builder

### Phase 8: UI Health Check
**Goal**: The app automatically verifies that all interactive UI elements are present and functional on every startup, with an optional guided walkthrough for first-time users
**Depends on**: Phase 7
**Requirements**: HLTH-01, HLTH-02, HLTH-03, HLTH-04
**Success Criteria** (what must be TRUE):
  1. On every app startup, a silent DOM audit runs after all async startup calls complete and a status badge appears in the status bar
  2. The health check verifies that all v2 interactive elements (chips, builder, integrations page, detail view) exist and respond to synthetic events
  3. Clicking the health badge when failures exist shows a report with specific element names and suggested fixes
  4. User can trigger the interactive walkthrough on demand from Settings; it also appears automatically on first launch
**Plans**: 2 plans

Plans:
- [x] 08-01-PLAN.md — DOM health audit engine, status badge in app bar, diagnostic report modal
- [ ] 08-02-PLAN.md — Interactive walkthrough with first-launch auto-trigger, Settings on-demand button, walkthrough_completed persistence

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Notion Integration Infrastructure | 3/3 | Complete    | 2026-02-18 |
| 2. Notion Connector | 2/2 | Complete    | 2026-02-18 |
| 3. Prompt Augmentation | 2/2 | Complete   | 2026-02-18 |
| 4. Integrations Settings UI | 3/3 | Complete    | 2026-02-18 |
| 5. Pipeline Builder Redesign | 2/3 | Complete    | 2026-02-19 |
| 6. Pre-Assignment UX and Execution | 3/3 | Complete    | 2026-02-19 |
| 7. Pipeline Data Model and Tags Migration | 2/2 | Complete    | 2026-02-19 |
| 8. UI Health Check | 1/2 | In progress | - |
