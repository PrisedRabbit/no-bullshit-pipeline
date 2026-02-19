# Roadmap: NBP

## Milestones

- ✅ **v1 Pipelines v2** — Phases 1-8 (shipped 2026-02-19) — [archive](milestones/v1-ROADMAP.md)
- 🚧 **v1.1 Resilience & Polish** — Phases 9-12 (in progress)

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

### 🚧 v1.1 Resilience & Polish (In Progress)

**Milestone Goal:** Harden the v1 pipeline system — fix known bugs, add error recovery for structured outputs, improve UX polish for pipeline chips and prompt augmentation visibility, and add schema management safeguards.

- [ ] **Phase 9: Bug Fixes** — Fix integrations tab render and Slack state consistency
- [ ] **Phase 10: Structured Output Error Recovery** — Retry, user-visible failure, and partial success for structured delivery steps
- [ ] **Phase 11: UX Polish** — Pipeline chip overflow menu, augmented prompt visibility, per-step timing
- [ ] **Phase 12: Schema Management** — Token budget validation, schema staleness warnings, in-builder re-sync

## Phase Details

### Phase 9: Bug Fixes
**Goal**: Known UI bugs are resolved so the app works correctly on first load without workarounds
**Depends on**: Phase 8 (v1 complete)
**Requirements**: BUG-01, BUG-02
**Success Criteria** (what must be TRUE):
  1. Integrations tab displays all connectors correctly the first time it is opened, without the user clicking or interacting to trigger a re-render
  2. Slack connection status shown in the app bar matches the status shown in the Integrations settings tab without a page reload
  3. Opening the app fresh shows consistent Slack state across all views
**Plans**: TBD

Plans:
- [ ] 09-01: Fix MutationObserver selector in integrations-settings.js and consolidate Slack connection state

### Phase 10: Structured Output Error Recovery
**Goal**: Pipeline steps that require structured AI output (JSON) handle failures gracefully without silent data loss or pipeline abandonment
**Depends on**: Phase 9
**Requirements**: ERR-01, ERR-02, ERR-03
**Success Criteria** (what must be TRUE):
  1. When a Notion delivery step receives invalid JSON from the AI, the system automatically retries with a stricter prompt before reporting failure
  2. When structured output parsing fails after retry, the pipeline run output shows the raw AI response alongside a clear, actionable error message
  3. When one delivery step fails, subsequent independent steps in the pipeline still execute and deliver their output
  4. Pipeline run output distinguishes between steps that succeeded, steps that failed, and steps that were skipped
**Plans**: TBD

Plans:
- [ ] 10-01: Implement JSON retry logic with stricter prompt for structured delivery steps
- [ ] 10-02: Add failure display (raw output + error message) and partial-success pipeline execution

### Phase 11: UX Polish
**Goal**: Pipeline interaction and run output provide clear, transparent information so users understand what the system did and can manage many pipelines efficiently
**Depends on**: Phase 9
**Requirements**: UX-01, UX-02, UX-03
**Success Criteria** (what must be TRUE):
  1. When more than N pipelines exist, the app bar shows only the top N pipeline chips with an overflow menu button that reveals the rest
  2. Pipeline run output includes an expandable "Augmented prompt" section that shows exactly what context was injected into the AI prompt
  3. Pipeline run output shows the wall-clock duration for each step so users can see which steps are slow
  4. The overflow menu and augmented prompt section work correctly at edge cases (0 pipelines, 1 pipeline, exactly N pipelines)
**Plans**: TBD

Plans:
- [ ] 11-01: Implement pipeline chip overflow menu in app bar
- [ ] 11-02: Add augmented prompt expandable section and per-step timing to pipeline run output

### Phase 12: Schema Management
**Goal**: Users are protected from token budget overflows and stale schema data, and can refresh schema without leaving the pipeline builder
**Depends on**: Phase 10, Phase 11
**Requirements**: SCHM-01, SCHM-02, SCHM-03
**Success Criteria** (what must be TRUE):
  1. Before executing an AI step with prompt augmentation, the system validates that the estimated token budget fits within the model's context limit and surfaces a clear error if not
  2. Integration profile in settings shows the timestamp of the last schema sync
  3. When a schema is more than 7 days old, the integration profile displays a visible staleness warning
  4. Pipeline builder includes a "Re-sync schema" action that triggers a fresh schema fetch without navigating to the Integrations settings tab
**Plans**: TBD

Plans:
- [ ] 12-01: Add token budget validation before AI step execution and schema staleness UI in integration profile
- [ ] 12-02: Add re-sync schema action to pipeline builder

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
| 9. Bug Fixes | v1.1 | 0/1 | Not started | - |
| 10. Structured Output Error Recovery | v1.1 | 0/2 | Not started | - |
| 11. UX Polish | v1.1 | 0/2 | Not started | - |
| 12. Schema Management | v1.1 | 0/2 | Not started | - |
