---
phase: 06-pre-assignment-ux-and-execution
plan: 03
subsystem: ui
tags: [tauri, javascript, pipeline-execution, auto-execute, event-driven]

# Dependency graph
requires:
  - phase: 06-pre-assignment-ux-and-execution-01
    provides: stoppedPipeline local variable captured in stopRecording(), pipeline chip bar, assign_pipeline command
  - phase: 06-pre-assignment-ux-and-execution-02
    provides: detail view pipeline assignment dropdown, get_all_pipeline_states, execute_pipeline command
provides:
  - autoTranscribeAndExecute() — fires transcribe_recording then execute_pipeline sequentially on recording stop
  - subscribeToProgress() — subscribes to pipeline-progress Tauri event with unlisten cleanup
  - renderPipelineStatus() — loads and renders pipeline state badges per recording in detail view
  - Pipeline status section in detail view (Waiting/Running/Done/Failed badges)
  - Inline failed step error display showing step name and error message
affects: [07-migration-and-cleanup, future-pipeline-phases]

# Tech tracking
tech-stack:
  added: []
  patterns: [fire-and-forget async auto-execute after recording stop, Tauri event listener with module-level unlisten cleanup, parallel render (pipeline status loads in parallel with transcript)]

key-files:
  created: []
  modified:
    - src/main.js
    - src/styles.css
    - src/index.html

key-decisions:
  - "autoTranscribeAndExecute() is NOT awaited in stopRecording() — fire-and-forget so user can continue using app while transcription and pipeline run in background"
  - "Auto-execute is gated on appSettings?.transcription?.enabled — if transcription is disabled, pipeline stays Waiting (no error, no attempt)"
  - "renderPipelineStatus() called without await in showDetailView() — runs in parallel with transcript loading for responsiveness"
  - "partial status from Rust displayed as Failed in UI — Rust enum variant name preserved in CSS class but display text uses PIPELINE_STATUS_DISPLAY map"
  - "subscribeToProgress() replaces previous listener on each showDetailView() call — prevents stacked pipeline-progress listeners across navigation"

patterns-established:
  - "Module-level unlisten variable pattern (pipelineProgressUnlisten) for Tauri event cleanup — same pattern as detailPipelineHandler"
  - "PIPELINE_STATUS_DISPLAY const maps Rust status strings to user-facing text — single source of truth for status display"

requirements-completed: [EXEC-01, EXEC-02, EXEC-03]

# Metrics
duration: 5min
completed: 2026-02-19
---

# Phase 6 Plan 03: Auto-Execute Pipeline on Recording Stop Summary

**Zero-touch pipeline execution after recording stop: autoTranscribeAndExecute() chains transcription then pipeline run with live status badges (Waiting/Running/Done/Failed) and inline error display in detail view**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-19T00:53:16Z
- **Completed:** 2026-02-19T00:58:29Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- autoTranscribeAndExecute() wired to stopRecording() as fire-and-forget — user continues using app while transcription + pipeline run in background
- pipeline-progress Tauri event subscription with per-detail-view unlisten cleanup — live status updates without listener accumulation
- Pipeline status section in detail view with Waiting/Running/Done/Failed color-coded badges
- Failed pipelines show inline error block with specific step name and error message via get_step_outputs

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire autoTranscribeAndExecute() to stopRecording() with pipeline-progress event subscription** - `2799d1c` (feat)
2. **Task 2: Render pipeline run status badges and failed step error in detail view** - `c132bce` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/main.js` - autoTranscribeAndExecute(), subscribeToProgress(), renderPipelineStatus(), PIPELINE_STATUS_DISPLAY, pipelineProgressUnlisten variable, stopRecording() wiring, showDetailView() and hideDetailView() integration
- `src/styles.css` - .pipeline-status-row, .pipeline-status-badge, .status-waiting/.running/.done/.partial, .pipeline-step-error
- `src/index.html` - pipeline-status-section content-block inside detail-content-grid

## Decisions Made
- autoTranscribeAndExecute() is NOT awaited — fire-and-forget pattern so user can continue using app while background processing occurs
- Auto-execute gated on transcription.enabled — if transcription is off, pipeline stays in Waiting state with no error shown
- renderPipelineStatus() runs without await in showDetailView() — parallel with transcript loading for responsiveness
- Rust "partial" status displayed as "Failed" via PIPELINE_STATUS_DISPLAY map — preserves class name for CSS but maps to friendly display text
- pipelineProgressUnlisten follows the same module-level variable pattern established by detailPipelineHandler in 06-02

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 6 complete — all 3 plans executed (chip bar, default/last-used pipeline, auto-execute with status display)
- Phase 7: Migration and cleanup (lazy migration of recordings with legacy tag field to pipeline assignment)
- The stoppedPipeline local variable pattern and auto-execute gate are finalized — Phase 7 can rely on this behavior

---
*Phase: 06-pre-assignment-ux-and-execution*
*Completed: 2026-02-19*
