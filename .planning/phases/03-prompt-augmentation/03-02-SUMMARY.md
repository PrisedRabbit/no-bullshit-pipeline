---
phase: 03-prompt-augmentation
plan: "02"
subsystem: api
tags: [rust, notion, validation, error-handling, llm]

# Dependency graph
requires:
  - phase: 03-01
    provides: build_augmented_prompt() prompt spec builder that generates the LLM output this plan validates
  - phase: 02-01
    provides: build_notion_properties() and NotionIntegrationProfile schema used for key matching
provides:
  - validate_llm_output_for_notion() function in connectors/notion.rs validates LLM JSON output against integration profile schema before Notion API call
  - WRITABLE_TYPES constant in connectors/notion.rs listing 12 Notion property types LLM can populate
  - Clear error messages with raw LLM output preview for all invalid-output cases
affects:
  - 03-prompt-augmentation (AUGM-04, AUGM-05 now satisfied)
  - Any future connector that needs similar validation gating

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Validation gate pattern — validate parsed array before delivery, fail with actionable error showing raw LLM output
    - Module-local WRITABLE_TYPES constant per connector — keeps connectors independent from pipeline engine

key-files:
  created: []
  modified:
    - src-tauri/src/connectors/notion.rs

key-decisions:
  - "validate_llm_output_for_notion() uses fully-qualified std::collections::HashSet path — no new use statement needed"
  - "WRITABLE_TYPES in connectors/notion.rs is separate from the one in pipeline_engine.rs (Plan 03-01) — intentional per-module independence"
  - "Empty array check fires before profile key matching — provides a clearer message for this common LLM failure mode"

patterns-established:
  - "Validation gate: parse -> validate against schema -> deliver; never skip validation"
  - "Error messages always include raw LLM output (first 500 chars) for actionable debugging"

requirements-completed: [AUGM-04, AUGM-05]

# Metrics
duration: 3min
completed: 2026-02-18
---

# Phase 3 Plan 02: Notion Output Validation Summary

**validate_llm_output_for_notion() gates Notion page creation behind schema key matching — empty arrays, non-objects, and schema mismatches fail with raw LLM output shown**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-18T22:51:50Z
- **Completed:** 2026-02-18T22:54:00Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- Added `WRITABLE_TYPES` constant (12 property types) and `validate_llm_output_for_notion()` function in `connectors/notion.rs`
- Wired validation call into `execute()` between `extract_json_array()` and the page creation loop — invalid output is caught before any Notion API call
- Strengthened `extract_json_array()` error message to explicitly label the preview as "Raw LLM output" per AUGM-05

## Task Commits

Each task was committed atomically:

1. **Task 1: Add validate_llm_output_for_notion() and wire into execute()** - `52640d6` (feat)
2. **Task 2: Strengthen extract_json_array() error message** - `52640d6` (feat — committed together as single file change)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src-tauri/src/connectors/notion.rs` - Added `WRITABLE_TYPES`, `validate_llm_output_for_notion()`, validation call in `execute()`, updated `extract_json_array()` error label

## Decisions Made
- `validate_llm_output_for_notion()` uses `std::collections::HashSet` fully qualified — avoids adding a new `use` statement to the imports list
- `WRITABLE_TYPES` in `connectors/notion.rs` is separate from the one in `pipeline_engine.rs` — keeps each module independently correct without cross-module coupling
- Empty array check comes before per-element checks — most common LLM failure mode (hallucinated non-array) gets a distinct message

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- `cargo check` unavailable in execution environment — structural verification used instead (all 4 verification criteria confirmed via grep/file inspection)

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 3 complete (both plans done): AUGM-01 through AUGM-05 satisfied
- Prompt augmentation pipeline fully wired: profile spec injected into LLM prompt (03-01), LLM output validated against profile schema before Notion delivery (03-02)
- Ready to advance to Phase 4

---
*Phase: 03-prompt-augmentation*
*Completed: 2026-02-18*
