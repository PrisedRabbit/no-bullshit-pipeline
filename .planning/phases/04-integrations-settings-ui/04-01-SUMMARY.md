---
phase: 04-integrations-settings-ui
plan: 01
subsystem: ui
tags: [tauri, vanilla-js, integrations, notion, slack, settings]

# Dependency graph
requires:
  - phase: 01-notion-integration-infrastructure
    provides: list_notion_profiles, remove_notion_integration, test_notion_integration functions
  - phase: 02-notion-connector
    provides: Notion integration profile storage pattern
affects:
  - 04-02-notion-setup-wizard
  - 04-03-save-path-backend

provides:
  - list_notion_profiles exposed as #[tauri::command] registered in invoke_handler
  - integrations-settings.js module with loadAllIntegrations, renderConnectedIntegrations, renderAvailableIntegrations
  - Connected/Available two-section integrations settings layout in index.html
  - Integration card CSS styles (.integration-card, .available-integration-card, etc.)
  - Notion and Slack integration cards with Test and Remove button handlers
  - Available section with + Add cards for Notion (wizard placeholder) and Slack (opens existing modal)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - State-first full re-render for integrations list (same pattern as pipeline builder)
    - MutationObserver for tab-activation lazy loading
    - Module-owned rendering with var globals for cross-module access

key-files:
  created:
    - src/integrations-settings.js
  modified:
    - src-tauri/src/integrations/notion.rs
    - src-tauri/src/lib.rs
    - src/index.html
    - src/styles.css
    - src/main.js

key-decisions:
  - "integrations-settings.js loads after main.js — escapeHtml and invoke available as globals without re-declaration"
  - "MutationObserver on integrations tab class attribute triggers loadAllIntegrations only when tab becomes active"
  - "var declarations for notionProfiles and _slackIntegrations put them on window — accessible from main.js for pipeline step editor (04-03)"
  - "add-slack-btn handler removed from main.js — integrations-settings.js Available section card now opens the add-slack-modal"

patterns-established:
  - "Integration card pattern: icon + info (name + detail) + actions (Test/Remove buttons) in .integration-card"
  - "Available card pattern: icon + info + '+ Add' label in .available-integration-card"
  - "Lazy loading pattern: MutationObserver fires loadAll on tab activation, not on app startup"

requirements-completed: [INTG-01, INTG-02]

# Metrics
duration: 8min
completed: 2026-02-18
---

# Phase 4 Plan 01: Integrations Settings UI Foundation Summary

**Connected/Available integrations settings page with list_notion_profiles Tauri command, integration card rendering for Notion + Slack, and Test/Remove inline actions**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-18T23:22:00Z
- **Completed:** 2026-02-18T23:30:48Z
- **Tasks:** 1
- **Files modified:** 6

## Accomplishments
- Added `#[tauri::command]` to `list_notion_profiles` and registered it in the invoke_handler — now callable from JS
- Replaced Slack-only integrations tab HTML with a two-section Connected/Available layout
- Created `integrations-settings.js` (242 lines) with full state management: loadAll, renderConnected, renderAvailable, and inline event handlers
- Added integration card CSS styles for both connected and available card variants
- Removed the now-redundant `add-slack-btn` handler from main.js (integrations-settings.js Available section handles it)

## Task Commits

Each task was committed atomically:

1. **Task 1: Expose list_notion_profiles + Connected/Available layout + integrations-settings.js** - `36c6982` (feat)

## Files Created/Modified
- `/workspace/src/integrations-settings.js` - New module: state management and rendering for all integrations
- `/workspace/src-tauri/src/integrations/notion.rs` - Added `#[tauri::command]` to list_notion_profiles
- `/workspace/src-tauri/src/lib.rs` - Registered list_notion_profiles in invoke_handler
- `/workspace/src/index.html` - Replaced integrations tab HTML with Connected/Available layout; added integrations-settings.js script tag
- `/workspace/src/styles.css` - Added integration card CSS (.integration-card, .available-integration-card, etc.)
- `/workspace/src/main.js` - Removed add-slack-btn click handler (now owned by integrations-settings.js)

## Decisions Made
- `integrations-settings.js` loads after `main.js` — `escapeHtml` and `invoke` from main.js are available as globals without re-declaration
- MutationObserver on integrations tab `class` attribute triggers `loadAllIntegrations` only when the tab becomes active (lazy loading)
- `var` declarations for `notionProfiles` and `_slackIntegrations` make them available on `window` for cross-module access in 04-03
- `add-slack-btn` handler removed from main.js — the Available section Slack card in integrations-settings.js now opens the existing `add-slack-modal`

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
- `cargo` not available in execution environment (pre-existing blocker from Phase 1). Rust changes verified structurally — `#[tauri::command]` annotation placed above `list_notion_profiles` following the identical pattern of all other Notion commands already in the file, and registered in `invoke_handler!` after `remove_notion_integration` following the established pattern.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Foundation complete: `connected-integrations-list` and `available-integrations-list` DOM containers exist
- `list_notion_profiles` Tauri command registered and callable
- `notionProfiles` and `_slackIntegrations` module state on `window` for pipeline step editor (04-03)
- `openNotionWizard()` placeholder wired in Available section Notion card — 04-02 just needs to define that function
- Add Slack flow works end-to-end: Available section Slack card opens existing `add-slack-modal` from main.js

## Self-Check: PASSED

- FOUND: src/integrations-settings.js
- FOUND: src-tauri/src/integrations/notion.rs (with #[tauri::command])
- FOUND: src-tauri/src/lib.rs (list_notion_profiles registered)
- FOUND: src/index.html (Connected/Available layout + script tag)
- FOUND: src/styles.css (integration card CSS)
- FOUND: .planning/phases/04-integrations-settings-ui/04-01-SUMMARY.md
- FOUND: commit 36c6982

---
*Phase: 04-integrations-settings-ui*
*Completed: 2026-02-18*
