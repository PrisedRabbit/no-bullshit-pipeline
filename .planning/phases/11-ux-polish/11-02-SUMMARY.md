---
phase: 11-ux-polish
plan: 02
subsystem: ui
tags: [pipeline, step-status, timing, augmented-prompt, transparency]

# Dependency graph
requires:
  - phase: 03-prompt-augmentation
    provides: build_augmented_prompt() function and Notion schema augmentation logic
  - phase: 10-structured-output-error-recovery
    provides: per-step status rendering in renderPipelineStatus(), StepStatus struct
provides:
  - StepStatus with duration_secs (from created_at/completed_at timestamps) and augmented_prompt (from sidecar file)
  - Per-step wall-clock timing displayed in pipeline run output for done and partial pipelines
  - Expandable augmented prompt section in pipeline UI for LLM steps with Notion augmentation
affects:
  - any phase using StepStatus or get_step_outputs Tauri command

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Sidecar file pattern for auxiliary data (.augmented-prompt.txt alongside step .md)
    - 3-tuple return from parse_step_status() for structured extraction
    - onclick toggle pattern using classList.toggle('expanded') for expandable sections

key-files:
  created: []
  modified:
    - src-tauri/src/pipelines.rs
    - src-tauri/src/pipeline_engine.rs
    - src/main.js
    - src/styles.css

key-decisions:
  - "Show per-step detail for 'done' pipelines too (not just 'partial') — transparency on successful runs"
  - "Sidecar .augmented-prompt.txt file for storing augmented prompt text — avoids modifying connector frontmatter format"
  - "parse_step_status returns 3-tuple (status, error, duration_secs) — clean separation, augmented_prompt loaded separately in get_step_outputs"
  - "Duration computed from existing created_at/completed_at timestamps in step .md frontmatter — no new fields needed in connectors"

patterns-established:
  - "Sidecar file pattern: {step_name}.augmented-prompt.txt written alongside {step_name}.md for auxiliary LLM step data"
  - "Expandable section pattern: .expanded class toggle via onclick, display:none/block for content visibility"

requirements-completed: [UX-02, UX-03]

# Metrics
duration: 2min
completed: 2026-02-19
---

# Phase 11 Plan 02: Augmented Prompt Visibility and Per-Step Timing Summary

**Extended StepStatus with wall-clock duration from timestamps and expandable augmented prompt from sidecar file, shown for all completed pipeline runs**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-19T09:30:00Z
- **Completed:** 2026-02-19T09:32:03Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Extended StepStatus struct with `duration_secs` (computed from `created_at`/`completed_at` frontmatter) and `augmented_prompt` (read from `.augmented-prompt.txt` sidecar file)
- LLM step execution now writes augmented prompt text to a sidecar file when prompt augmentation occurs, making it available to the UI
- Per-step detail section now shows for both 'done' and 'partial' pipeline states (previously only 'partial')
- Duration displayed per step (e.g., "3.2s" or "1m 15s") and expandable "Augmented prompt" section for LLM steps with Notion augmentation

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend StepStatus and parse_step_status** - `9bf8274` (feat)
2. **Task 2: Add expandable augmented prompt and per-step duration UI** - `8bd5991` (feat)

**Plan metadata:** _(final commit in progress)_

## Files Created/Modified
- `src-tauri/src/pipelines.rs` - Added `duration_secs: Option<f64>` and `augmented_prompt: Option<String>` to StepStatus struct
- `src-tauri/src/pipeline_engine.rs` - parse_step_status now returns 3-tuple with duration; get_step_outputs reads sidecar file; execute_pipeline_internal writes sidecar file before LLM call
- `src/main.js` - renderPipelineStatus() shows per-step detail for done+partial, adds duration label and augmented prompt expandable section
- `src/styles.css` - CSS for .step-duration, .augmented-prompt-section, .augmented-prompt-toggle, .augmented-prompt-content with .expanded toggle

## Decisions Made
- Show per-step detail for 'done' pipelines too — users benefit from timing transparency on successful runs, not just failed ones
- Sidecar .augmented-prompt.txt file for storing augmented prompt text — avoids modifying connector frontmatter format across all connectors (llm.rs, save.rs, webhook.rs, slack.rs, notion.rs)
- parse_step_status returns 3-tuple (status, error, duration_secs) — clean separation; augmented_prompt loaded separately in get_step_outputs via sidecar file
- Duration computed from existing created_at/completed_at timestamps already written by all connectors — no new fields needed

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Phase 11 UX Polish complete (both plans executed)
- Phase 12 (Schema sync/validation) can proceed — no blockers
- Compilation deferred per project convention (cargo check not available in environment)

---
*Phase: 11-ux-polish*
*Completed: 2026-02-19*
