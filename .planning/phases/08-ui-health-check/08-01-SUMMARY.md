---
phase: 08-ui-health-check
plan: 01
subsystem: ui
tags: [vanilla-js, dom-audit, health-check, badge, modal]

# Dependency graph
requires:
  - phase: 05-pipeline-builder-redesign
    provides: pipeline-builder.js global (allPipelineDefs, escapeHtml) available as script load target
  - phase: 06-pre-assignment-ux-and-execution
    provides: v2 interactive elements (pipeline chips, chip bar) that health check audits
provides:
  - DOM health audit engine that checks 35 always-present v2 element IDs on startup
  - Persistent health badge in app bar visible in all view states (recordings, detail, settings)
  - Health report modal with per-element issue cards and re-run capability
  - Post-init audit scheduling via requestIdleCallback with setTimeout fallback
affects: [08-02-walkthrough, future-phases-ui]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - ui-health-check.js loads last after pipeline-builder.js to access all globals
    - Health badge placed as direct .app-bar child (not inside .capture-section) for view-state persistence
    - initHealthCheck() wires modal button listeners once after init() resolves (same pattern as other modals)
    - requestIdleCallback with 2000ms timeout falls back to setTimeout(fn, 500) for WKWebView compatibility

key-files:
  created:
    - src/ui-health-check.js
  modified:
    - src/index.html
    - src/styles.css
    - src/main.js

key-decisions:
  - "health badge placed as direct .app-bar child between .app-logo and permission-warning — avoids CSS body.settings-open/.detail-open hiding .capture-section"
  - "AUDIT_ELEMENTS has 35 entries covering all always-present v2 element IDs; lazy-loaded integrations tab content is excluded (correct per research)"
  - "initHealthCheck() wired in init().finally() — ensures listeners attach exactly once regardless of init() success/failure"
  - "window._lastHealthResult stores last audit result for re-opening report without re-running audit"

patterns-established:
  - "Pattern: health check module loads last and references globals set by earlier scripts without import/export"
  - "Pattern: init().catch().finally() chain for post-init side effects that must run regardless of init() outcome"

requirements-completed: [HLTH-01, HLTH-02, HLTH-03]

# Metrics
duration: 2min
completed: 2026-02-19
---

# Phase 8 Plan 01: UI Health Check Engine Summary

**Silent DOM audit with persistent app bar badge — 35 v2 element IDs checked via requestIdleCallback after init(), green checkmark when all pass, red warning count when failures exist with clickable report modal**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-19T01:47:00Z
- **Completed:** 2026-02-19T01:49:42Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Created `src/ui-health-check.js` with `runHealthAudit()`, `renderHealthBadge()`, `showHealthReport()`, `initHealthCheck()` — 35 always-present v2 element IDs checked, no var globals, no ES modules
- Health badge added as direct `.app-bar` child (between `.app-logo` and permission warning) — visible in recordings, detail, and settings view states
- Health report modal follows existing modal pattern (`modal-overlay` + `modal-card`) with Close and Re-run Audit buttons
- main.js `init()` call updated to `init().catch().finally()` to wire initHealthCheck() once and schedule audit via `requestIdleCallback` (2000ms timeout, `setTimeout` fallback)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create ui-health-check.js** - `92e8fca` (feat)
2. **Task 2: Add badge HTML, modal, CSS, audit scheduling** - `58c4fa1` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified

- `src/ui-health-check.js` — DOM audit engine, badge renderer, report modal logic, initHealthCheck() wiring
- `src/index.html` — health badge div after .app-logo, health report modal before onboarding overlay, ui-health-check.js script tag last
- `src/styles.css` — .health-badge, .health-badge-ok, .health-badge-fail, .health-issue-row, .health-issue-id, .health-issue-desc, .health-issue-fix CSS classes
- `src/main.js` — replaced `init().catch()` with `init().catch().finally()` containing initHealthCheck() call and requestIdleCallback audit scheduling

## Decisions Made

- Health badge placed as direct `.app-bar` child (not inside `.capture-section`) to remain visible when CSS view-state logic hides `.capture-section` via `body.settings-open` and `body.detail-open`
- `window._lastHealthResult` stores last audit result so report can be re-opened without re-running
- `initHealthCheck()` called in `init().finally()` — attaches modal listeners exactly once regardless of init() success or failure
- 35 AUDIT_ELEMENTS (exceeds plan's 25+ requirement) covering all v2 always-present elements; lazy-loaded integrations tab children correctly excluded

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Health audit engine and badge complete — ready for Phase 8 Plan 02 (interactive walkthrough)
- `initHealthCheck()` extension point is clean for adding walkthrough button wiring in 08-02
- No blockers

---
*Phase: 08-ui-health-check*
*Completed: 2026-02-19*

## Self-Check: PASSED

- src/ui-health-check.js: FOUND
- src/index.html: FOUND
- src/styles.css: FOUND
- src/main.js: FOUND
- 08-01-SUMMARY.md: FOUND
- Commit 92e8fca: FOUND
- Commit 58c4fa1: FOUND
