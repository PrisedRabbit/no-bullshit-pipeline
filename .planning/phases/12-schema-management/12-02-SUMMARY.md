---
phase: 12-schema-management
plan: "02"
subsystem: ui
tags: [notion, pipeline-builder, schema, tauri, invoke]

# Dependency graph
requires:
  - phase: 02-notion-connector
    provides: sync_notion_schema Tauri command and notionProfiles global
  - phase: 05-pipeline-builder-redesign
    provides: pipeline builder step editor with showStepEditor() and Notion connector branch
provides:
  - Re-sync Schema button in Notion step editor inside pipeline builder
affects: [pipeline-builder, notion-integration, schema-management]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Wire event handlers after DOM insertion (stepEl.replaceWith(editorEl) then addEventListener)"
    - "Scope connector-specific UI wiring with if (step.connector === 'notion') guard"
    - "Update global array in-place after successful Tauri invoke to keep downstream UI consistent"

key-files:
  created: []
  modified:
    - src/pipeline-builder.js

key-decisions:
  - "Re-sync uses existing sync_notion_schema Tauri command — no new backend code needed"
  - "notionProfiles global updated in-place so subsequent pipeline builder interactions see fresh data"
  - "Button only appears when profiles.length > 0 — naturally scoped by existing conditional branch"
  - "Success/error feedback shown inline via .resync-status span next to the button"

patterns-established:
  - "Post-DOM-insertion handler wiring: attach event listeners after replaceWith() so editorEl is in DOM"

requirements-completed: [SCHM-03]

# Metrics
duration: 1min
completed: 2026-02-19
---

# Phase 12 Plan 02: Re-sync Schema in Pipeline Builder Summary

**Re-sync Schema button in Notion step editor calls sync_notion_schema, updates notionProfiles global in-place, shows inline loading/success/error feedback without navigating away**

## Performance

- **Duration:** ~1 min
- **Started:** 2026-02-19T13:09:39Z
- **Completed:** 2026-02-19T13:10:24Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Added Re-sync Schema button row (button + status span) inside Notion step editor configFields, shown only when profiles exist
- Wired async click handler after editor element is inserted into DOM — reads currently selected integration_id, finds profile, calls sync_notion_schema
- Updates notionProfiles[idx] in-place so all downstream UI stays current without page reload
- Full loading/success/error feedback: "Syncing..." while in progress, inline text status, error shown in red with error message

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Re-sync Schema button to Notion step editor** - `a9df3ea` (feat)

**Plan metadata:** _(docs commit — see below)_

## Files Created/Modified
- `src/pipeline-builder.js` - Added Re-sync Schema button in Notion connector configFields and post-DOM click handler with full async sync logic

## Decisions Made
- Re-sync uses existing `sync_notion_schema` Tauri command — no new backend code needed
- `notionProfiles` global updated in-place so other UI stays current without reload
- Button scoped to `profiles.length > 0` branch naturally — no additional guard needed
- Success/error feedback shown inline next to button via `.resync-status` span
- Button handler wired after `stepEl.replaceWith(editorEl)` so editorEl is in DOM when querySelector runs

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 12 complete — both plans executed (12-01: token budget validation + schema staleness UI; 12-02: re-sync in builder)
- SCHM-01, SCHM-02, SCHM-03 all completed
- v1.1 Schema Management milestone complete

## Self-Check: PASSED

- src/pipeline-builder.js: FOUND
- .planning/phases/12-schema-management/12-02-SUMMARY.md: FOUND
- Commit a9df3ea: FOUND

---
*Phase: 12-schema-management*
*Completed: 2026-02-19*
