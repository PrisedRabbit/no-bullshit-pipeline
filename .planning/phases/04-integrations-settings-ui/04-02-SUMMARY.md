---
phase: 04-integrations-settings-ui
plan: 02
subsystem: ui
tags: [tauri, vanilla-js, notion, wizard, state-machine, integrations]

# Dependency graph
requires:
  - phase: 04-01-integrations-settings-foundation
    provides: integrations-settings.js module, Connected/Available layout, openNotionWizard() placeholder
  - phase: 01-notion-integration-infrastructure
    provides: add_notion_integration, list_notion_databases, sync_notion_schema, update_notion_people_mappings, remove_notion_integration Tauri commands

provides:
  - notion-wizard-modal HTML overlay with header, progress bar, body, and footer
  - Wizard CSS styles (input groups, database list, schema table, people mapping rows, info box)
  - openNotionWizard() entry point callable from Available section Notion card
  - renderWizardStep() 5-step state machine dispatcher
  - Step 0: API key entry with add_notion_integration validation and inline errors
  - Step 1: Mandatory share-instruction info box before database picker
  - Step 2: Database picker with list_notion_databases, selection, share-instruction error, retry button
  - Step 3: Schema table display (property name/type/options) with synced_at timestamp and re-sync button
  - Step 4: People mapping with pre-populated rows for people-type props, state-first management, Finish via update_notion_people_mappings
  - Cancel cleanup via remove_notion_integration at any step after API key entry

affects:
  - 04-03-save-path-backend

# Tech tracking
tech-stack:
  added: []
  patterns:
    - State-first wizard pattern: notionWizardState drives all rendering, never reads DOM for state
    - replaceNextBtn() helper removes stacked event listeners by cloning nodes
    - Step-specific render functions called by central renderWizardStep() dispatcher
    - Cancel cleanup gate: remove_notion_integration only called if integrationId exists

key-files:
  created: []
  modified:
    - src/integrations-settings.js
    - src/index.html
    - src/styles.css

key-decisions:
  - "Cancel wires once in openNotionWizard() via node clone — not re-attached on each step render"
  - "replaceNextBtn() clones Next button node on each step to prevent stacked async event listeners"
  - "Step 2 DB picker re-renders itself inline (not via renderWizardStep) to avoid re-fetching databases on error"
  - "People mapping rows use closure-captured idx for state mutations — removes need for data attributes"
  - "Finish skips update_notion_people_mappings call entirely when cleanMappings array is empty"

patterns-established:
  - "Wizard step render functions accept (body, nextBtn) — body for innerHTML, nextBtn for replaceNextBtn()"
  - "Error display pattern: set notionWizardState.error, call renderWizardStep() — error shown in step template"

requirements-completed: [NOTN-03, NOTN-04, NOTN-05]

# Metrics
duration: 2min
completed: 2026-02-18
---

# Phase 4 Plan 02: Notion Setup Wizard Summary

**Five-step Notion setup wizard (API key -> share instructions -> database picker -> schema display -> people mapping) using state-first rendering and existing Phase 1 Tauri commands**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-18T23:33:06Z
- **Completed:** 2026-02-18T23:35:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Added notion-wizard-modal overlay HTML with progress bar, dynamic body, and footer buttons to index.html
- Added complete wizard CSS covering all 5 step variants: input groups, database list items, schema table, mapping rows, info box, error states
- Implemented 458-line wizard state machine in integrations-settings.js: state object, openNotionWizard(), renderWizardStep() dispatcher, and 5 step renderer functions
- Cancel at any step after API key entry calls remove_notion_integration for cleanup (no orphan profiles)
- Completing wizard calls loadAllIntegrations() to refresh Connected section immediately

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Notion wizard modal HTML and wizard step CSS styles** - `9f6ad6f` (feat)
2. **Task 2: Implement Notion wizard state machine and step rendering** - `dff1bf1` (feat)

## Files Created/Modified
- `/workspace/src/integrations-settings.js` - Added 458 lines: notionWizardState, openNotionWizard, renderWizardStep, renderStep0-4, replaceNextBtn helper
- `/workspace/src/index.html` - Added notion-wizard-modal div (18 lines) after add-slack-modal
- `/workspace/src/styles.css` - Added 185 lines of wizard-* CSS classes

## Decisions Made
- Cancel button wired once in openNotionWizard() via node clone to avoid duplicate handlers across re-renders
- replaceNextBtn() clones the Next button node on each step render — eliminates stacked async event listener bugs
- Step 2 DB picker re-renders its own body inline to avoid re-fetching the databases list on sync error
- People mapping rows use closure-captured array index for state mutations — clean, no data attribute parsing
- Finish button skips update_notion_people_mappings entirely when all mappings are incomplete (empty alias or no user) rather than sending an empty array

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- `cargo` not available in execution environment (pre-existing blocker from Phase 1). All JS code verified with `node --check` for syntax correctness. Tauri command calls follow the identical invoke pattern established throughout integrations-settings.js and other modules.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Notion wizard complete end-to-end: API key entry -> share instructions -> database selection -> schema display -> people mapping
- openNotionWizard() defined and callable (the 04-01 placeholder guard `typeof openNotionWizard === 'function'` now passes)
- Connected section refreshes automatically after wizard completion via loadAllIntegrations()
- 04-03 can proceed with save path backend and delivery picker wiring

## Self-Check: PASSED

- FOUND: src/integrations-settings.js
- FOUND: src/index.html (notion-wizard-modal present)
- FOUND: src/styles.css (wizard-progress CSS present)
- FOUND: .planning/phases/04-integrations-settings-ui/04-02-SUMMARY.md
- FOUND: commit 9f6ad6f (Task 1 - HTML + CSS)
- FOUND: commit dff1bf1 (Task 2 - wizard state machine)

---
*Phase: 04-integrations-settings-ui*
*Completed: 2026-02-18*
