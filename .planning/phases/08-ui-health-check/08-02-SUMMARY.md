---
phase: 08-ui-health-check
plan: 02
subsystem: ui
tags: [vanilla-js, walkthrough, onboarding, overlay, spotlight]

# Dependency graph
requires:
  - phase: 08-01
    provides: initHealthCheck() extension point in ui-health-check.js, health badge visible in DOM
  - phase: 06-pre-assignment-ux-and-execution
    provides: pipeline chip bar and all v2 UI elements that walkthrough spotlights
provides:
  - Interactive 7-step UI walkthrough with spotlight positioning
  - First-launch auto-trigger gated on onboarding_completed && !walkthrough_completed
  - On-demand re-launch from Settings > Audio "Start Tour" button
  - walkthrough_completed persistence via AppSettings and save_settings Tauri command
affects: [ui-health-check.js, index.html, styles.css, main.js, config.rs]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Walkthrough engine appended to ui-health-check.js after audit/badge/report code — no ES modules, no globals
    - getBoundingClientRect() + box-shadow 9999px technique for spotlight with transparent hole over target
    - pointer-events: none on overlay, pointer-events: auto on card — clicks pass through background, card is interactive
    - walkthroughStep module-local counter — no var globals
    - finishWalkthrough() uses try/catch around invoke — non-fatal on settings save failure

key-files:
  created: []
  modified:
    - src-tauri/src/config.rs
    - src/ui-health-check.js
    - src/index.html
    - src/styles.css
    - src/main.js

key-decisions:
  - "walkthrough_completed uses #[serde(default)] — existing settings.json files missing this field deserialize safely without resetting all user settings"
  - "Walkthrough engine appended to ui-health-check.js (not a new file) — consistent with no-bundler static file pattern"
  - "CSS uses --border and --bg-card variables (defined in :root) rather than --border-color/--surface-color — those variables are undefined in root scope"
  - "walkthroughStep counter is module-local (not var) — consistent with rest of ui-health-check.js no-globals pattern"
  - "First-launch trigger: onboarding_completed && !walkthrough_completed in scheduleAudit — walkthrough shows after audit so health badge is visible in step 7"

patterns-established:
  - "Pattern: Spotlight overlay uses box-shadow: 0 0 0 9999px technique — dark veil with transparent hole at target element"
  - "Pattern: Walkthrough card positioned below spotlight, flipped above if near viewport bottom"

requirements-completed: [HLTH-04]

# Metrics
duration: 2min
completed: 2026-02-19
---

# Phase 8 Plan 02: Interactive UI Walkthrough Summary

**7-step spotlight walkthrough with first-launch auto-trigger (onboarding_completed && !walkthrough_completed) and on-demand Start Tour button in Settings > Audio; walkthrough_completed persisted via AppSettings.save_settings**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-19T01:53:39Z
- **Completed:** 2026-02-19T01:55:54Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Added `walkthrough_completed: bool` with `#[serde(default)]` to `AppSettings` in `config.rs` — safe for existing settings files (missing field defaults to false, never resets other settings)
- Added walkthrough engine to `ui-health-check.js`: `WALKTHROUGH_STEPS` (7 steps), `startWalkthrough()`, `showWalkthroughStep()`, `finishWalkthrough()` — module-local state, no var globals
- Spotlight uses `getBoundingClientRect()` + `box-shadow: 0 0 0 9999px rgba(0,0,0,0.6)` — dark veil with transparent hole at each target element; card positioned below spotlight (or above if near viewport bottom)
- Walkthrough overlay HTML added to `index.html` — prev/next/skip buttons, step counter, title/desc card; "Start Tour" button in Settings > Audio Help section
- CSS added to `styles.css` for overlay, spotlight, card, and navigation buttons using defined CSS variables (`--border`, `--bg-card`, `--accent`, `--bg-input`)
- First-launch trigger in `main.js` `scheduleAudit`: fires `startWalkthrough()` when `onboarding_completed && !walkthrough_completed` — never auto-triggers again after completion/skip

## Task Commits

Each task was committed atomically:

1. **Task 1: walkthrough_completed in config.rs + walkthrough engine in ui-health-check.js** - `1e2f182` (feat)
2. **Task 2: Walkthrough overlay HTML, CSS, Settings button, first-launch trigger** - `67c0ce7` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified

- `src-tauri/src/config.rs` — added `walkthrough_completed: bool` field with `#[serde(default)]` to `AppSettings` struct and `Default` impl
- `src/ui-health-check.js` — appended `WALKTHROUGH_STEPS`, `walkthroughStep` counter, `startWalkthrough()`, `showWalkthroughStep()`, `finishWalkthrough()`; wired nav/skip/start buttons in `initHealthCheck()`
- `src/index.html` — added `#walkthrough-overlay` div with spotlight, card, navigation buttons; added "Start Tour" button in Settings > Audio Help section
- `src/styles.css` — appended walkthrough overlay, spotlight, card, step counter, action buttons, and nav button CSS
- `src/main.js` — updated `scheduleAudit` to call `startWalkthrough()` when `onboarding_completed && !walkthrough_completed`

## Decisions Made

- `#[serde(default)]` on `walkthrough_completed` is critical — without it, any `settings.json` missing this field fails to deserialize and all settings reset to defaults
- CSS variables `--border` and `--bg-card` used instead of plan's `--border-color`/`--surface-color` — those variables are not defined in `:root` scope; the defined variables produce correct themed styling
- Walkthrough engine appended to `ui-health-check.js` rather than a new file — no bundler, static files served directly, one fewer script tag
- `walkthroughStep` is a `let` (not `var`) — module-local, consistent with no-globals constraint in ui-health-check.js

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] CSS variable names corrected**
- **Found during:** Task 2
- **Issue:** Plan specified `var(--border-color)`, `var(--surface-color)`, `var(--bg-hover)` in walkthrough CSS, but these variables are not defined in `:root`. Using undefined CSS variables causes invisible/broken styling.
- **Fix:** Used `var(--border)`, `var(--bg-card)`, `var(--bg-input)` — the variables actually defined in `:root` for all three themes.
- **Files modified:** `src/styles.css`
- **Commit:** `67c0ce7`

## Issues Encountered

None beyond the CSS variable deviation above.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- All 2 plans in Phase 8 (UI Health Check) complete
- Full Pipelines v2 milestone (Phases 1-8) complete
- No blockers

---
*Phase: 08-ui-health-check*
*Completed: 2026-02-19*

## Self-Check: PASSED

- src-tauri/src/config.rs: FOUND
- src/ui-health-check.js: FOUND
- src/index.html: FOUND
- src/styles.css: FOUND
- src/main.js: FOUND
- 08-02-SUMMARY.md: FOUND
- Commit 1e2f182: FOUND
- Commit 67c0ce7: FOUND
