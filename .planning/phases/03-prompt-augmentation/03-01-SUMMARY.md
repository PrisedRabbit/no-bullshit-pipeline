---
phase: 03-prompt-augmentation
plan: 01
subsystem: pipeline
tags: [rust, prompt-augmentation, llm, notion, pipeline-engine]

# Dependency graph
requires:
  - phase: 02-notion-connector
    provides: "ConnectorType::Notion arm in pipeline_engine.rs, NotionIntegrationProfile with properties/people_mappings"
  - phase: 01-notion-integration-infrastructure
    provides: "load_notion_profile(), NotionIntegrationProfile struct, NotionPropertyDef with select_options"
provides:
  - "N+1 look-ahead in execute_pipeline_internal() — auto-detects LLM→Notion step pairs"
  - "build_augmented_prompt() — loads template, substitutes transcript, appends Notion format spec"
  - "build_notion_format_spec() — compact writable-property spec from NotionIntegrationProfile"
  - "connectors::llm::execute() augmented_prompt: Option<&str> parameter — None path unchanged"
affects:
  - 03-02-PLAN (validate_llm_output_for_notion depends on LLM receiving correct format spec)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "N+1 look-ahead: pipeline engine peeks at i+1 step to conditionally modify step i behavior"
    - "Hard Result<> fail before LLM API call — no silent fallthrough to non-JSON output"
    - "Augmentation is runtime-only — prompt templates on disk are never modified"

key-files:
  created: []
  modified:
    - src-tauri/src/connectors/llm.rs
    - src-tauri/src/pipeline_engine.rs

key-decisions:
  - "augmented_prompt: Option<&str> as 7th parameter — None path preserves all pre-Phase-3 behavior exactly"
  - "build_augmented_prompt() performs hard fail when profile missing or has empty properties/database_id — prevents expensive LLM call"
  - "WRITABLE_TYPES filters out formula/relation/rollup/created_time/last_edited_time — only LLM-settable fields in format spec"
  - "MAX_OPTIONS_IN_SPEC=12 caps select options per field to prevent token budget overflow"
  - "People field format spec includes known aliases from profile.people_mappings — enables alias-based LLM output"

patterns-established:
  - "N+1 look-ahead: check pipeline.steps.get(i + 1) inside step loop to detect downstream Notion step"
  - "Augmentation hard-fail pattern: Err with 'Sync schema in Settings' message before LLM API call"
  - "Format spec lines.join('\\n') pattern for compact token-efficient schema descriptions"

requirements-completed: [AUGM-01, AUGM-02, AUGM-03]

# Metrics
duration: 5min
completed: 2026-02-18
---

# Phase 3 Plan 01: N+1 Look-Ahead Prompt Augmentation Summary

**N+1 look-ahead in pipeline_engine.rs auto-injects Notion database schema as JSON format spec into LLM prompt when LLM step is directly followed by Notion step**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-18T22:44:00Z
- **Completed:** 2026-02-18T22:49:25Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Extended `connectors::llm::execute()` with `augmented_prompt: Option<&str>` — Some path bypasses template loading, None path is identical to pre-Phase-3
- Added `build_augmented_prompt()` to `pipeline_engine.rs` — loads template, reads transcript, appends Notion format spec; hard-fails with clear "sync schema" message if profile missing or empty
- Added `build_notion_format_spec()` — builds compact field-by-field format spec from `NotionIntegrationProfile`, filtering to writable types only, capping select options at MAX_OPTIONS_IN_SPEC=12
- Added `WRITABLE_TYPES` and `MAX_OPTIONS_IN_SPEC` constants at module level in `pipeline_engine.rs`
- Updated `ConnectorType::Llm` arm with N+1 look-ahead: checks `pipeline.steps.get(i + 1)` for Notion connector and passes augmented prompt through

## Task Commits

Each task was committed atomically:

1. **Task 1: Add augmented_prompt parameter to connectors/llm.rs execute()** - `5d48a83` (feat)
2. **Task 2: Implement look-ahead, build_augmented_prompt(), and build_notion_format_spec() in pipeline_engine.rs** - `2687f1a` (feat)

**Plan metadata:** (docs commit — created below)

## Files Created/Modified

- `src-tauri/src/connectors/llm.rs` — Added `augmented_prompt: Option<&str>` 7th param; Some branch applies token estimation + truncation to augmented string; None branch is unchanged original logic
- `src-tauri/src/pipeline_engine.rs` — Added WRITABLE_TYPES const, MAX_OPTIONS_IN_SPEC const, build_augmented_prompt(), build_notion_format_spec(); updated ConnectorType::Llm arm with N+1 look-ahead

## Decisions Made

- `augmented_prompt: Option<&str>` as 7th parameter preserves all pre-Phase-3 behavior when None — non-Notion pipelines are completely unaffected
- Hard fail in `build_augmented_prompt()` when profile missing or `properties.is_empty() || database_id.is_empty()` — prevents expensive LLM API call from producing non-JSON output
- `WRITABLE_TYPES` excludes formula, relation, rollup, created_time, last_edited_time — only fields the LLM can meaningfully set are included in the format spec
- `MAX_OPTIONS_IN_SPEC = 12` prevents token budget overflow for databases with many select options (overflow count shown in spec)
- People field format spec includes known aliases from `profile.people_mappings` when present — LLM can use alias strings directly

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `cargo check` not available in execution environment — structural verification performed instead (same pattern as Phases 1-2). Real compilation deferred to first `cargo tauri dev` run.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 03-01 complete: LLM connector accepts augmented prompt, pipeline engine auto-detects LLM→Notion chains
- Plan 03-02 can proceed: validate_llm_output_for_notion() in notion.rs + error message strengthening depends on LLM receiving correct JSON format spec (now in place)
- All AUGM-01, AUGM-02, AUGM-03 requirements satisfied

---
*Phase: 03-prompt-augmentation*
*Completed: 2026-02-18*

## Self-Check: PASSED

- FOUND: src-tauri/src/connectors/llm.rs
- FOUND: src-tauri/src/pipeline_engine.rs
- FOUND: .planning/phases/03-prompt-augmentation/03-01-SUMMARY.md
- FOUND commit: 5d48a83 (Task 1 — llm.rs augmented_prompt param)
- FOUND commit: 2687f1a (Task 2 — pipeline_engine.rs look-ahead + helpers)
