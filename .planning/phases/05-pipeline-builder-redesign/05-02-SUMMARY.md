---
phase: 05-pipeline-builder-redesign
plan: 02
subsystem: ui
tags: [pipeline-builder, step-picker, presets, prompt-templates, javascript]

# Dependency graph
requires:
  - phase: 05-pipeline-builder-redesign
    provides: "pipeline-builder.js module extracted with SortableJS, addPipelineStepBtn DOM element"
provides:
  - "PROCESSING_PRESETS array in pipeline-builder.js with 5 one-click presets"
  - "Step picker UI (Processing / Delivery sections) shown on addPipelineStepBtn click"
  - "buildDeliveryOptions() reading notionProfiles, savePathIntegrations, slackIntegrations globals"
  - "addPresetStep() adds fully configured steps with zero fields to fill"
  - "Built-in action-items, summary, structure templates in prompt_templates.rs"
  - "Builtin merge logic in load_prompt_templates() for existing installs"
affects:
  - 05-03-pipeline-builder-redesign

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Step picker rendered as DOM element inserted after addPipelineStepBtn, dismissed on outside click via stopPropagation"
    - "PROCESSING_PRESETS[n].step === null signals special handling (Custom Prompt placeholder for 05-03)"
    - "buildDeliveryOptions() uses typeof guards for all three integration globals — safe before integrations tab loads"

key-files:
  created: []
  modified:
    - src/pipeline-builder.js
    - src-tauri/src/prompt_templates.rs
    - src/styles.css

key-decisions:
  - "Picker inserted inline (not floating/absolute) after addPipelineStepBtn — simpler than positioned dropdown, scrolls with content"
  - "Custom Prompt preset.step=null signals deferred implementation — addPresetStep falls through to blank LLM step placeholder until 05-03"
  - "Builtin merge uses contains_key check to preserve user-modified built-ins — new installs get fresh defaults, existing installs get new templates without overwriting their changes"
  - "Delivery options built dynamically from globals at showPicker() call time — always current with latest integration state"

patterns-established:
  - "One-click preset adds fully-configured step — name, connector, config.prompt_template, description all set from PROCESSING_PRESETS definition"
  - "Picker dismissed via document click listener added with setTimeout(0) delay to avoid immediate dismissal on the open click"

requirements-completed: [BLDR-01, BLDR-02, BLDR-03]

# Metrics
duration: 2min
completed: 2026-02-19
---

# Phase 5 Plan 02: Categorized Step Picker Summary

**Categorized step picker with Processing/Delivery sections and 5 one-click presets (Meeting Notes, Action Items, Summary, Structure, Custom Prompt) replacing the blank-step add button**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-19T00:15:11Z
- **Completed:** 2026-02-19T00:17:18Z
- **Tasks:** 2
- **Files modified:** 3 (pipeline-builder.js, prompt_templates.rs, styles.css)

## Accomplishments
- Added `PROCESSING_PRESETS` with 5 entries; clicking Meeting Notes/Action Items/Summary/Structure adds a fully configured LLM step with correct `prompt_template` reference in one click
- Implemented `buildDeliveryOptions()` rendering connected Notion profiles, save paths, and Slack workspaces with typeof guards; shows helpful empty message when none are configured
- Added three new built-in templates (action-items, summary, structure) to `get_builtin_templates()` and merge logic in `load_prompt_templates()` so existing users get them automatically without losing customizations
- Replaced blank-step button with `togglePicker()` — picker shows/hides on click, dismisses on outside click

## Task Commits

Each task was committed atomically:

1. **Task 1: Add built-in action-items, summary, and structure templates** - `94cc7ea` (feat)
2. **Task 2: Implement categorized step picker with processing presets and delivery integrations** - `88cdabc` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/pipeline-builder.js` - Added PROCESSING_PRESETS, picker functions (buildDeliveryOptions, togglePicker, showPicker, dismissPicker, closePicker, addPresetStep), replaced addPipelineStepBtn listener
- `src-tauri/src/prompt_templates.rs` - Added action-items, summary, structure built-in templates; added builtin merge logic in load_prompt_templates(); updated test assertion to expect 6 built-ins
- `src/styles.css` - Added .step-picker, .step-picker-section, .step-picker-section-title, .step-picker-option, .step-picker-icon, .step-picker-empty CSS classes

## Decisions Made
- Picker inserted inline (not floating/absolute positioned) after `addPipelineStepBtn` — simpler, scrolls naturally with content
- `PROCESSING_PRESETS[n].step === null` signals Custom Prompt deferred case — placeholder adds blank LLM step until Plan 05-03 implements the full form
- Builtin merge uses `contains_key` check: new installs get fresh default timestamps, existing installs get new templates without overwriting user-modified built-ins
- Delivery options built at `showPicker()` call time so they reflect latest integration state

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- `cargo` not available in execution environment (known constraint from STATE.md blockers) — Rust code verified structurally; compilation will be confirmed at first `cargo tauri dev` run

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Step picker is functional with Processing and Delivery sections
- Custom Prompt preset is a placeholder (blank LLM step) — Plan 05-03 replaces it with a full prompt form
- 05-03 (Custom Prompt form + assembly preview + Advanced section + prompt_inline Rust backend) can proceed immediately
- `showStepEditor()` in pipeline-builder.js is unchanged and ready for modification in 05-03

## Self-Check: PASSED

- `src/pipeline-builder.js` exists with PROCESSING_PRESETS (5 presets) and picker functions
- `src-tauri/src/prompt_templates.rs` exists with action-items, summary, structure templates
- `src/styles.css` exists with step-picker CSS classes
- `05-02-SUMMARY.md` exists
- Commit `94cc7ea` (Task 1: backend templates) verified
- Commit `88cdabc` (Task 2: step picker UI) verified

---
*Phase: 05-pipeline-builder-redesign*
*Completed: 2026-02-19*
