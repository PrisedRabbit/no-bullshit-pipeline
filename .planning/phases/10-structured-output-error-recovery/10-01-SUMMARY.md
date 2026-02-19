---
phase: 10-structured-output-error-recovery
plan: 01
subsystem: pipeline
tags: [rust, notion, llm, error-recovery, retry, json-parsing]

# Dependency graph
requires:
  - phase: 05-pipeline-builder-redesign
    provides: pipeline execution engine (pipeline_engine.rs) and step dispatch
  - phase: 06-notion-integration
    provides: Notion connector (notion.rs) and integration profiles
  - phase: 07-pipeline-data-model-and-tags-migration
    provides: LLM connector (llm.rs) and prompt augmentation
provides:
  - NotionErrorKind enum distinguishing JsonParse (with raw_output) from Other errors
  - execute_structured() for typed error returns from the Notion connector
  - execute_with_raw_preservation() for final-failure writes with raw AI output
  - execute_retry() in llm.rs for corrective-prompt LLM calls
  - Pipeline engine retry orchestration: one automatic retry on JSON parse failures
affects:
  - 10-02 (partial-success pipeline execution — consumes retry infrastructure)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Error categorization via enum variants (JsonParse vs Other) for retry-aware dispatch"
    - "Corrective-prompt retry: failed LLM output fed back to same model with fix instructions"
    - "Raw output preservation in step .md files (## Raw AI Output section) for diagnosis"
    - "Structured error return (execute_structured) alongside backward-compat execute() wrapper"

key-files:
  created: []
  modified:
    - src-tauri/src/connectors/notion.rs
    - src-tauri/src/connectors/llm.rs
    - src-tauri/src/pipeline_engine.rs

key-decisions:
  - "Use NotionErrorKind enum (not a bool) to distinguish JSON parse failures — enables retry-aware dispatch without string matching on error messages"
  - "Max 1 retry — no loop — prevents infinite retry storms on consistently bad models"
  - "Retry looks backward to the immediately-preceding LLM step (N-1 step) for provider/model config, consistent with N+1 look-ahead augmentation pattern"
  - "execute_with_raw_preservation() as final-failure path — always writes raw output regardless of retry outcome"
  - "execute() wrapper preserved with identical signature for backward compatibility with non-retry callers"

patterns-established:
  - "Retry-aware connector pattern: execute_structured() for typed errors, execute() for backward compat"
  - "Corrective prompt includes first 2000 chars of failed output + last 500 chars of original prompt"

requirements-completed: [ERR-01, ERR-02]

# Metrics
duration: 3min
completed: 2026-02-19
---

# Phase 10 Plan 01: Structured Output Error Recovery Summary

**JSON retry-on-failure with raw output preservation for Notion delivery steps: NotionErrorKind enum, execute_structured/execute_retry/execute_with_raw_preservation, and pipeline engine one-shot retry orchestration**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-19T07:40:26Z
- **Completed:** 2026-02-19T07:43:30Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Added `NotionErrorKind` enum that distinguishes `JsonParse { message, raw_output }` from `Other(String)` — enables the pipeline engine to dispatch retry logic without string matching on error text
- Added `execute_retry()` in `llm.rs` that builds a corrective prompt (failed output + original format spec) and calls the same provider/model, writing output to `{step}-retry.md`
- Pipeline engine now automatically retries a Notion step exactly once on JSON parse failure; on double failure writes the step .md with `## Raw AI Output` section for user diagnosis

## Task Commits

Each task was committed atomically:

1. **Task 1: Categorize Notion connector errors and preserve raw output on failure** - `e5a21b2` (feat)
2. **Task 2: Add LLM retry function and pipeline engine retry orchestration** - `026d3a0` (feat)

## Files Created/Modified

- `src-tauri/src/connectors/notion.rs` - Added NotionErrorKind, NotionError, execute_structured(), execute_with_raw_preservation(), write_failure_output(); refactored to execute_inner() core
- `src-tauri/src/connectors/llm.rs` - Added execute_retry() with corrective prompt construction and retry output file writing
- `src-tauri/src/pipeline_engine.rs` - Updated Notion dispatch to use execute_structured(), added JSON parse retry with execute_retry() and execute_with_raw_preservation()

## Decisions Made

- Used `NotionErrorKind` enum variants rather than a boolean `is_json_error` flag — enables exhaustive pattern matching and carries the `raw_output` string without a separate out-parameter
- Max 1 retry (no loop) — prevents retry storms if a model consistently returns bad JSON; failure is always terminal after one retry
- Retry looks backward to N-1 step (immediately-preceding LLM step) for provider/model config — consistent with the existing N+1 look-ahead augmentation pattern
- `execute_with_raw_preservation()` used as the final-failure write path in all double-failure branches — ensures `raw_output` is always written to the step .md file body even when the retry Notion call also fails
- `execute()` preserved with identical `Result<PathBuf, String>` signature — backward compatible with any non-retry callers

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `cargo check` not available in execution environment (known blocker from STATE.md) — all types, signatures, and cross-references verified structurally. Compilation deferred to first `cargo tauri dev` run.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 10-02 (partial-success pipeline execution and per-step status UI) can now consume the error infrastructure built here
- `NotionErrorKind::JsonParse` is accessible from `connectors::notion` for any future connector that needs similar error categorization
- No blockers

---
*Phase: 10-structured-output-error-recovery*
*Completed: 2026-02-19*
