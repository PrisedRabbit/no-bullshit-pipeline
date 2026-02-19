---
phase: 16-linear-frontend
plan: 02
subsystem: pipeline-ui
tags: [linear, pipeline-builder, frontend, resync, tauri]
dependency_graph:
  requires:
    - phase: 16-01
      provides: Linear wizard UI, linearProfiles global, sync_linear_schema Tauri command
  provides:
    - Linear delivery option in pipeline builder step picker
    - Linear step config editor with integration selector and re-sync button
    - Re-sync button on Linear connected card in integration settings
  affects: [src/pipeline-builder.js, src/integrations-settings.js]
tech-stack:
  added: []
  patterns: [Notion connector pattern replicated for Linear in pipeline builder]
key-files:
  created: []
  modified:
    - src/pipeline-builder.js
    - src/integrations-settings.js
key-decisions:
  - "Linear in deliveryConnectors array alongside save/notion/slack/webhook/mcp — ensures Linear steps render with delivery styling in preview"
  - "typeof linearProfiles guard used in buildDeliveryOptions() and showStepEditor() — allows pipeline builder to load independently before integrations-settings.js globals are populated"
patterns-established:
  - "New connector integration in pipeline builder: add to buildDeliveryOptions(), add to deliveryConnectors[], add option to connector select, add config fields block, add re-sync handler"
requirements-completed: [LINEAR-05, LINEAR-08]
duration: 2min
completed: 2026-02-19
---

# Phase 16 Plan 02: Pipeline Builder Linear Step + Re-sync UI Summary

**Linear delivery step added to pipeline builder with integration selector and schema re-sync from both step editor and settings connected card.**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-19T14:23:35Z
- **Completed:** 2026-02-19T14:25:02Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Linear appears as a delivery option in the pipeline builder step picker when linearProfiles is non-empty
- Linear step config editor shows integration selector dropdown (with team name) and Re-sync Schema button
- Re-sync from pipeline builder calls `sync_linear_schema`, updates global `linearProfiles`, and shows inline status feedback
- Re-sync button added to Linear connected cards in integration settings — calls `sync_linear_schema` and re-renders card with new timestamp
- Linear correctly listed in `deliveryConnectors` so it renders with delivery styling in pipeline preview

## Task Commits

Each task was committed atomically:

1. **Task 1: Linear delivery option and step config editor in pipeline builder** - `2008e87` (feat)
2. **Task 2: Schema re-sync button on Linear connected card in integration settings** - `ad866a6` (feat)

**Plan metadata:** (docs commit below)

## Files Created/Modified
- `src/pipeline-builder.js` - Added Linear delivery option in buildDeliveryOptions(), Linear in deliveryConnectors[], Linear option in connector select dropdown, Linear config fields block with integration selector and re-sync button, Linear re-sync button handler
- `src/integrations-settings.js` - Added resync-linear-btn between Test and Remove in Linear card actions, wired re-sync handler calling sync_linear_schema and re-rendering card

## Decisions Made
- `typeof linearProfiles` guard used consistently (same pattern as `typeof notionProfiles`) — pipeline builder can operate even if integrations-settings.js globals haven't been set yet
- Linear in `deliveryConnectors` array ensures the pipeline preview renders Linear steps with the delivery node class (distinct visual styling from processing steps)

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 16 complete — all Linear frontend work shipped (wizard UI in 16-01, pipeline builder + re-sync in 16-02)
- Linear connector is now end-to-end: backend (Phase 13), integration UI (16-01), pipeline builder integration (16-02)

---
*Phase: 16-linear-frontend*
*Completed: 2026-02-19*

## Self-Check: PASSED

- [x] src/pipeline-builder.js — modified
- [x] src/integrations-settings.js — modified
- [x] .planning/phases/16-linear-frontend/16-02-SUMMARY.md — created
- [x] 2008e87 — feat(16-02): add Linear delivery option and step config editor in pipeline builder
- [x] ad866a6 — feat(16-02): add Re-sync button on Linear connected card in integration settings
