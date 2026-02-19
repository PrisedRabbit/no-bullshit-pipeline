---
phase: 06-pre-assignment-ux-and-execution
plan: 01
subsystem: ui
tags: [pipeline-chips, app-bar, recording, vanilla-js, tauri]

# Dependency graph
requires:
  - phase: 05-pipeline-builder-redesign
    provides: allPipelineDefs global array, loadPipelineDefs(), pipeline CRUD via Tauri invoke
provides:
  - renderPipelineChips() rendering pipeline chips in app bar from allPipelineDefs
  - handleChipClick() dispatching to start-recording-with-pipeline or mid-recording assign
  - startRecordingWithPipeline() starting recording and immediately assigning pipeline
  - showOverflowPopover() overflow dropdown for >5 pipelines
  - currentAssignedPipeline state variable tracking active pipeline assignment
  - pipeline-chip-bar HTML container in capture-section
  - CSS classes for chip bar, chips, overflow button, overflow popover
affects:
  - 06-02-PLAN (last-used pipeline default chip highlighting uses appSettings.last_used_pipeline)
  - 06-03-PLAN (stoppedPipeline local variable in stopRecording() ready for auto-execute use)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - typeof guard for cross-module function calls (renderPipelineChips called from pipeline-builder.js with typeof check)
    - Chip bar innerHTML replaced atomically then event listeners attached via querySelectorAll
    - setTimeout(..., 0) for outside-click dismiss listeners (same pattern as step-picker in pipeline-builder.js)
    - currentAssignedPipeline captured to local variable in stopRecording() before clearing global (prevents race condition)

key-files:
  created: []
  modified:
    - src/index.html
    - src/styles.css
    - src/main.js
    - src/pipeline-builder.js

key-decisions:
  - "Pipeline chip bar uses position:relative on container and position:absolute on overflow popover — chips stay in app bar flow"
  - "renderPipelineChips() replaces entire innerHTML atomically and re-attaches listeners — consistent with pipeline-builder state-first pattern"
  - "startRecordingWithPipeline() sets isRecording=true before invoke('assign_pipeline') so chips can show is-assigned immediately"
  - "stoppedPipeline captured from currentAssignedPipeline at start of stopRecording() try block — 06-03 can use this local variable for auto-execute trigger"

requirements-completed: [ASGN-01, ASGN-02, ASGN-03, ASGN-07]

# Metrics
duration: 2min
completed: 2026-02-19
---

# Phase 6 Plan 01: Pipeline Chip Bar Summary

**Pipeline chip bar in app bar with one-click recording+assignment, 5-chip cap with overflow popover, and mid-recording pipeline switching**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-19T00:47:34Z
- **Completed:** 2026-02-19T00:49:29Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Pipeline chips rendered in capture-section app bar from allPipelineDefs on app load (via loadPipelineDefs() hook)
- Clicking a chip when not recording calls startRecordingWithPipeline() — starts recording AND assigns pipeline in one action
- 5-chip maximum enforced with +N overflow button revealing popover dropdown for additional pipelines
- Chips remain clickable during recording and call assign_pipeline mid-recording with visual is-assigned state
- stopRecording() correctly captures currentAssignedPipeline to local variable before clearing global (race-condition-free, ready for 06-03 auto-execute)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add chip bar HTML, CSS, and renderPipelineChips() with overflow popover** - `d12ebad` (feat)
2. **Task 2: Implement startRecordingWithPipeline() and mid-recording chip assignment** - `d97e0fd` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/index.html` - Added `<div id="pipeline-chip-bar" class="pipeline-chip-bar">` inside #capture-section before .capture-controls
- `src/styles.css` - Added .pipeline-chip-bar, .pipeline-chip, .pipeline-chip.is-last-used, .pipeline-chip.is-assigned, .chip-overflow-btn, .chip-overflow-popover CSS classes
- `src/main.js` - Added currentAssignedPipeline state, renderPipelineChips(), showOverflowPopover(), handleChipClick(), startRecordingWithPipeline(); updated stopRecording() to capture+clear pipeline state
- `src/pipeline-builder.js` - Added renderPipelineChips() call in loadPipelineDefs() via typeof guard after updateSidebarCounts()

## Decisions Made
- Pipeline chip bar uses `position: relative` on container and `position: absolute` on overflow popover — chips stay in app bar flow without affecting layout
- `renderPipelineChips()` replaces entire innerHTML atomically then re-attaches event listeners via querySelectorAll — consistent with pipeline-builder state-first pattern, prevents DOM/state desync
- `startRecordingWithPipeline()` sets `isRecording = true` and `currentAssignedPipeline = pipelineName` before calling `assign_pipeline` so chips immediately show `is-assigned` visual state
- `stoppedPipeline` local variable captured from `currentAssignedPipeline` at start of `stopRecording()` try block — 06-03 can read this local for auto-execute without race condition

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- 06-02: Pipeline chip bar ready for last-used pipeline highlighting — `appSettings.last_used_pipeline` field referenced in renderPipelineChips() is-last-used logic
- 06-03: `stoppedPipeline` local variable in `stopRecording()` is in place and ready for 06-03 to wire auto-execute logic

---
*Phase: 06-pre-assignment-ux-and-execution*
*Completed: 2026-02-19*
