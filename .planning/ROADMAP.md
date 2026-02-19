# Roadmap: NBP

## Milestones

- ✅ **v1 Pipelines v2** — Phases 1-8 (shipped 2026-02-19) — [archive](milestones/v1-ROADMAP.md)
- ✅ **v1.1 Resilience & Polish** — Phases 9-12 (shipped 2026-02-19) — [archive](milestones/v1.1-ROADMAP.md)
- 🚧 **v1.2 Connector Expansion** — Phases 13-18 (in progress)

## Phases

<details>
<summary>✅ v1 Pipelines v2 (Phases 1-8) — SHIPPED 2026-02-19</summary>

- [x] Phase 1: Notion Integration Infrastructure (3/3 plans) — completed 2026-02-18
- [x] Phase 2: Notion Connector (2/2 plans) — completed 2026-02-18
- [x] Phase 3: Prompt Augmentation (2/2 plans) — completed 2026-02-18
- [x] Phase 4: Integrations Settings UI (3/3 plans) — completed 2026-02-18
- [x] Phase 5: Pipeline Builder Redesign (3/3 plans) — completed 2026-02-19
- [x] Phase 6: Pre-Assignment UX and Execution (3/3 plans) — completed 2026-02-19
- [x] Phase 7: Pipeline Data Model and Tags Migration (2/2 plans) — completed 2026-02-19
- [x] Phase 8: UI Health Check (2/2 plans) — completed 2026-02-19

</details>

<details>
<summary>✅ v1.1 Resilience & Polish (Phases 9-12) — SHIPPED 2026-02-19</summary>

- [x] Phase 9: Bug Fixes (1/1 plan) — completed 2026-02-19
- [x] Phase 10: Structured Output Error Recovery (2/2 plans) — completed 2026-02-19
- [x] Phase 11: UX Polish (2/2 plans) — completed 2026-02-19
- [x] Phase 12: Schema Management (2/2 plans) — completed 2026-02-19

</details>

### 🚧 v1.2 Connector Expansion (In Progress)

**Milestone Goal:** Add Linear delivery connector, webhook named endpoints, and multi-pipeline-per-recording so pipelines cover all major delivery targets and users can apply multiple processing lenses to a single recording.

- [x] **Phase 13: Linear Backend** — API client, Keychain auth, schema fetch and storage
- [x] **Phase 14: Linear Delivery** — Structured output parsing, issue creation, JSON retry
- [x] **Phase 15: Linear Pipeline Integration** — Prompt augmentation with schema format specs, engine match arm
- [x] **Phase 16: Linear Frontend** — Integration wizard, pipeline builder delivery option, re-sync UI, member alias mapping (completed 2026-02-19)
- [ ] **Phase 17: Webhook Named Endpoints** — Named webhook endpoints in Integrations settings, delivery picker, backend CRUD
- [ ] **Phase 18: Multi-Pipeline Per Recording** — Chip bar multi-select, parallel pipeline execution on stop, multi-assign in detail view

## Phase Details

### Phase 13: Linear Backend
**Goal**: The system can authenticate with Linear, fetch project schema, and store credentials securely
**Depends on**: Phase 12 (existing connector patterns)
**Requirements**: LINEAR-01, LINEAR-03, LINEAR-09
**Success Criteria** (what must be TRUE):
  1. A valid Linear API key can be verified against the Linear GraphQL API
  2. Given a verified API key, the system fetches and persists the team/project schema (custom fields, labels, priorities, status options, team members)
  3. The Linear API key is stored in macOS Keychain and retrieved transparently (with dev-mode bypass)
  4. Schema data is stored in the integration profile on disk alongside the credential reference
**Plans**: 1 plan

Plans:
- [x] 13-01: Linear API client, credential storage, and schema fetch (types, GraphQL client, Keychain integration, team listing, schema persistence)

### Phase 14: Linear Delivery
**Goal**: The pipeline engine can create a Linear issue from structured LLM output, with retry on parse failure
**Depends on**: Phase 13
**Requirements**: LINEAR-07, LINEAR-10
**Success Criteria** (what must be TRUE):
  1. Given valid structured LLM output, a Linear issue is created with correct title, description, priority, label, assignee, and status fields
  2. When JSON parsing of LLM output fails, the engine retries with a structured-output correction prompt (same pattern as Notion)
  3. After successful retry, the issue is created with the corrected field values
  4. Delivery failures (network, API errors) surface as pipeline step errors visible to the user
**Plans**: 1 plan

Plans:
- [x] 14-01: Linear connector module, ConnectorType::Linear, and pipeline engine match arm with JSON retry

### Phase 15: Linear Pipeline Integration
**Goal**: Pipelines with an LLM step followed by a Linear step automatically receive Linear schema format instructions in the prompt
**Depends on**: Phase 14
**Requirements**: LINEAR-06
**Success Criteria** (what must be TRUE):
  1. When a pipeline has an LLM step immediately before a Linear step, the LLM prompt is automatically augmented with Linear field format specifications (priority values, label names, status options, team member IDs)
  2. The augmentation reflects the current stored schema, not hardcoded values
  3. Pipelines without a Linear step receive no Linear augmentation
**Plans**: 1 plan

Plans:
- [x] 15-01: Prompt augmentation for Linear schema and pipeline engine match arm

### Phase 16: Linear Frontend
**Goal**: Users can set up a Linear integration, configure a delivery step in the pipeline builder, map team member aliases, and re-sync the schema
**Depends on**: Phase 15
**Requirements**: LINEAR-02, LINEAR-04, LINEAR-05, LINEAR-08
**Success Criteria** (what must be TRUE):
  1. User can open integration settings, enter a Linear API key, select a team and project, and save — the integration appears in the list
  2. The pipeline builder shows a Linear delivery step option only when a Linear integration exists
  3. User can map Linear team member display names to name aliases used in transcripts (participant resolution)
  4. User can trigger a schema re-sync from integration settings or from the pipeline builder, and the UI shows a staleness warning when the schema is outdated
**Plans**: 2 plans

Plans:
- [x] 16-01: Linear wizard UI, connected cards, backend member alias support (LINEAR-02, LINEAR-04)
- [x] 16-02: Linear delivery step in pipeline builder and schema re-sync UI (LINEAR-05, LINEAR-08)

### Phase 17: Webhook Named Endpoints
**Goal**: Users can pre-configure named webhook endpoints in Integrations settings and select them by name in the pipeline builder delivery picker
**Depends on**: Phase 16
**Success Criteria** (what must be TRUE):
  1. User can add a named webhook endpoint (name, URL, method, optional headers) in Settings > Integrations — it appears in the Connected list
  2. The pipeline builder delivery picker shows configured webhook endpoints by name (not a raw URL field)
  3. When a pipeline step with a named webhook runs, the request is sent to the configured URL with the stored settings
  4. User can edit or remove a webhook endpoint from the Connected list
**Plans**: 2 plans

Plans:
- [ ] 17-01-PLAN.md — Backend: WebhookIntegration profile type, CRUD Tauri commands, update connector to use integration_id
- [ ] 17-02-PLAN.md — Frontend: Integrations settings section (add/edit/remove/test), pipeline builder delivery picker

### Phase 18: Multi-Pipeline Per Recording
**Goal**: A single recording can have multiple pipelines assigned and all run automatically after stop
**Depends on**: Phase 17
**Success Criteria** (what must be TRUE):
  1. Chip bar supports multi-select — clicking a chip toggles it on/off, multiple chips can be active simultaneously
  2. After recording stops, all assigned pipelines run (sequentially or with independent progress tracking per pipeline)
  3. Detail view shows all assigned pipelines with individual run status; user can add or remove pipelines post-recording
  4. If a transcript already exists (older recording), assigning a new pipeline triggers execution immediately for that pipeline
**Plans**: 2 plans

Plans:
- [ ] 18-01-PLAN.md — Frontend chip bar multi-select, autoTranscribeAndExecute multi-pipeline loop, progress tracking per pipeline
- [ ] 18-02-PLAN.md — Detail view multi-pipeline assignment and status display, post-recording pipeline add/remove

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Notion Integration Infrastructure | v1 | 3/3 | Complete | 2026-02-18 |
| 2. Notion Connector | v1 | 2/2 | Complete | 2026-02-18 |
| 3. Prompt Augmentation | v1 | 2/2 | Complete | 2026-02-18 |
| 4. Integrations Settings UI | v1 | 3/3 | Complete | 2026-02-18 |
| 5. Pipeline Builder Redesign | v1 | 3/3 | Complete | 2026-02-19 |
| 6. Pre-Assignment UX and Execution | v1 | 3/3 | Complete | 2026-02-19 |
| 7. Pipeline Data Model and Tags Migration | v1 | 2/2 | Complete | 2026-02-19 |
| 8. UI Health Check | v1 | 2/2 | Complete | 2026-02-19 |
| 9. Bug Fixes | v1.1 | 1/1 | Complete | 2026-02-19 |
| 10. Structured Output Error Recovery | v1.1 | 2/2 | Complete | 2026-02-19 |
| 11. UX Polish | v1.1 | 2/2 | Complete | 2026-02-19 |
| 12. Schema Management | v1.1 | 2/2 | Complete | 2026-02-19 |
| 13. Linear Backend | v1.2 | 1/1 | Complete | 2026-02-19 |
| 14. Linear Delivery | v1.2 | 1/1 | Complete | 2026-02-19 |
| 15. Linear Pipeline Integration | v1.2 | 1/1 | Complete | 2026-02-19 |
| 16. Linear Frontend | v1.2 | 2/2 | Complete | 2026-02-19 |
| 17. Webhook Named Endpoints | v1.2 | 0/2 | Not started | - |
| 18. Multi-Pipeline Per Recording | v1.2 | 0/2 | Not started | - |
