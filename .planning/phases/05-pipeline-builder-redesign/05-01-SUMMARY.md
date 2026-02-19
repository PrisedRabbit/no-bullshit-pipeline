---
phase: 05-pipeline-builder-redesign
plan: 01
subsystem: ui
tags: [pipeline-builder, sortablejs, drag-and-drop, javascript, module-extraction]

# Dependency graph
requires:
  - phase: 04-integrations-settings-ui
    provides: "notionProfiles, savePathIntegrations globals from integrations-settings.js accessed via typeof guards"
provides:
  - "src/pipeline-builder.js: standalone module with all pipeline state and functions"
  - "SortableJS 1.15.6 vendor file at src/vendor/sortable.min.js"
  - "index.html script load order: sortable.min.js -> main.js -> integrations-settings.js -> pipeline-builder.js"
  - "Pipeline preview moved below steps list in editor HTML"
affects:
  - 05-02-pipeline-builder-redesign
  - 05-03-pipeline-builder-redesign

# Tech tracking
tech-stack:
  added: ["SortableJS 1.15.6 (src/vendor/sortable.min.js)"]
  patterns:
    - "pipeline-builder.js uses var allPipelineDefs for window-global access by main.js updateSidebarCounts()"
    - "SortableJS re-initialized via initSortable() on editor open, destroyed on editor close"
    - "renderPipelineSteps() destroys and re-creates Sortable instance after innerHTML replacement"
    - "notionProfiles and savePathIntegrations accessed via typeof guards (safe before integrations tab loads)"

key-files:
  created:
    - src/pipeline-builder.js
    - src/vendor/sortable.min.js
  modified:
    - src/index.html
    - src/styles.css
    - src/main.js

key-decisions:
  - "SortableJS re-initialized in renderPipelineSteps() after innerHTML replacement — Sortable instances become stale when DOM is replaced"
  - "var allPipelineDefs in pipeline-builder.js makes pipeline count accessible to main.js updateSidebarCounts() at runtime without re-declaration"
  - "initSortable() called from openPipelineEditor() after renderPipelineSteps() so SortableJS is ready when editor opens"
  - "Pipeline preview div moved below steps in HTML — better UX flow: steps first, then visual chain"

patterns-established:
  - "Script load order pattern: vendor libs -> main.js -> feature-specific modules that depend on main.js globals"
  - "Cross-module globals via var declarations — earlier-loaded script globals accessed by later-loaded scripts without re-declaration"

requirements-completed: [BLDR-05, BLDR-06]

# Metrics
duration: 4min
completed: 2026-02-19
---

# Phase 5 Plan 01: Pipeline Builder Extraction Summary

**Extracted ~427-line pipeline builder from main.js into pipeline-builder.js and replaced native HTML5 DnD with SortableJS 1.15.6 for reliable drag-and-drop in macOS WKWebView**

## Performance

- **Duration:** 4 min
- **Started:** 2026-02-19T00:08:19Z
- **Completed:** 2026-02-19T00:12:31Z
- **Tasks:** 2
- **Files modified:** 4 (src/pipeline-builder.js created, src/vendor/sortable.min.js created, src/index.html modified, src/styles.css modified, src/main.js modified)

## Accomplishments
- Created `src/pipeline-builder.js` (439 lines) with all pipeline state variables, DOM references, and functions extracted from main.js
- Downloaded SortableJS 1.15.6 (45KB) to `src/vendor/sortable.min.js` for reliable drag-and-drop
- Replaced unreliable native HTML5 dragstart/dragover/drop handlers with SortableJS `onEnd` callback using handle-based dragging
- Updated index.html script load order and moved pipeline preview below steps for better UX flow
- Added `.pipeline-step-item--ghost` CSS class for drag visual feedback

## Task Commits

Each task was committed atomically:

1. **Task 1: Download SortableJS vendor file and update index.html script load order** - `0276902` (feat)
2. **Task 2: Extract pipeline builder code from main.js into pipeline-builder.js with SortableJS integration** - `746189d` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/pipeline-builder.js` - Standalone pipeline builder module with all pipeline state, functions, and event listeners
- `src/vendor/sortable.min.js` - SortableJS 1.15.6 library for reliable drag-and-drop
- `src/index.html` - Updated script load order (sortable.min.js first, pipeline-builder.js last), pipeline preview moved below steps list
- `src/styles.css` - Added `.pipeline-step-item--ghost` class for SortableJS drag feedback
- `src/main.js` - Removed pipeline builder code (427 lines); `loadPipelineDefs()` call in `init()` and `updateSidebarCounts()` preserved

## Decisions Made
- `var allPipelineDefs` (not `let`) in pipeline-builder.js so `window.allPipelineDefs` is available to `updateSidebarCounts()` in main.js at runtime
- SortableJS destroyed and re-created in `renderPipelineSteps()` because innerHTML replacement creates new DOM nodes; the old Sortable instance would reference stale nodes
- `initSortable()` called from `openPipelineEditor()` after `renderPipelineSteps()` ensures SortableJS is initialized when editor opens

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Pipeline builder module is fully extracted and working with SortableJS
- 05-02 (categorized step picker + presets + backend templates) can proceed — `showStepEditor()` is in pipeline-builder.js ready for modification
- 05-03 (Custom Prompt form + assembly preview + prompt_inline backend) can proceed after 05-02

---
*Phase: 05-pipeline-builder-redesign*
*Completed: 2026-02-19*
