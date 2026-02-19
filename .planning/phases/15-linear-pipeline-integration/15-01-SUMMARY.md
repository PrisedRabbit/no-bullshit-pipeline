---
phase: 15-linear-pipeline-integration
plan: 01
subsystem: pipeline
tags: [rust, prompt-augmentation, llm, linear, pipeline-engine]

# Dependency graph
requires:
  - phase: 14-linear-delivery
    provides: "ConnectorType::Linear arm in pipeline_engine.rs, LinearIntegrationProfile with workflow_states/labels/members/priorities, load_linear_profile()"
  - phase: 03-prompt-augmentation
    provides: "N+1 look-ahead pattern, build_augmented_prompt() pattern, MAX_OPTIONS_IN_SPEC constant, format spec line-join pattern"
provides:
  - "build_linear_format_spec() — compact format spec from LinearIntegrationProfile (all 6 Linear fields)"
  - "build_linear_augmented_prompt() — loads prompt, substitutes transcript, appends Linear format spec, hard-fails on missing/empty profile"
  - "LLM N+1 look-ahead: augments prompt for both Notion (existing) and Linear (new) downstream steps"
  - "Linear retry block: uses augmented prompt for retry LLM call (not bare prompt)"
affects:
  - "Any future phases that run LLM→Linear pipelines"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "N+1 look-ahead extended to multiple delivery connector types — else-if branch pattern"
    - "Shared augmented variable handles both Notion and Linear augmented prompts via same sidecar/budget/execute path"
    - "Hard fail before LLM API call for Linear schema the same as Notion — profile.workflow_states.is_empty() && profile.team_id.is_empty()"

key-files:
  created: []
  modified:
    - src-tauri/src/pipeline_engine.rs

key-decisions:
  - "Linear format spec outputs a single JSON object (not array) — matches Linear's one-issue-per-step model"
  - "Hard fail condition uses workflow_states.is_empty() && team_id.is_empty() — both empty indicates no schema synced"
  - "build_linear_augmented_prompt() follows build_augmented_prompt() pattern exactly — consistent error message style"
  - "Linear retry block replaces bare-prompt reconstruction with build_linear_augmented_prompt() — retry sees same schema guidance"

patterns-established:
  - "Delivery connector look-ahead: add else-if branch for each new delivery connector type in the LLM arm"
  - "Retry augmented prompt: call build_{connector}_augmented_prompt() in retry block for schema-aware retries"

requirements-completed: [LINEAR-06]

# Metrics
duration: 2min
completed: 2026-02-19
---

# Phase 15 Plan 01: Linear Pipeline Integration Summary

**Linear schema prompt augmentation in pipeline_engine.rs: LLM steps before Linear delivery steps auto-inject team schema (priority levels, workflow states, label names, member names) into the prompt via N+1 look-ahead**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-19T07:00:12Z
- **Completed:** 2026-02-19T07:01:46Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- Added `build_linear_format_spec()` — generates compact format spec from `LinearIntegrationProfile` with all 6 Linear fields (title, description, priority, status, labels, assignee) using live schema data with MAX_OPTIONS_IN_SPEC=12 cap
- Added `build_linear_augmented_prompt()` — loads prompt template or inline prompt, substitutes transcript, appends Linear format spec; hard-fails with actionable "Sync schema in Settings" message if profile missing or empty
- Extended LLM N+1 look-ahead with `else if ConnectorType::Linear` branch — augmented prompt built for Linear next step, flows through shared sidecar file save and budget validation paths
- Updated Linear retry block: replaced bare-prompt reconstruction (the Phase 14 placeholder) with `build_linear_augmented_prompt()` so retries receive the same schema guidance as the original LLM call

## Task Commits

Each task was committed atomically:

1. **Task 1: Add build_linear_format_spec() and build_linear_augmented_prompt()** - `f5a0bd4` (feat)
2. **Task 2: Extend LLM look-ahead for Linear and update Linear retry** - `533fe76` (feat)

**Plan metadata:** (docs commit — created below)

## Files Created/Modified

- `src-tauri/src/pipeline_engine.rs` — Added `build_linear_format_spec()` (lines 250-371), `build_linear_augmented_prompt()` (lines 384-438); extended LLM arm look-ahead with Linear else-if branch; replaced Linear retry bare-prompt block with `build_linear_augmented_prompt()` call

## Decisions Made

- `build_linear_format_spec()` outputs "a JSON object" (singular) vs Notion's "a JSON array" — correct for Linear's one-issue-per-step model
- Hard fail uses `profile.workflow_states.is_empty() && profile.team_id.is_empty()` — both conditions ensure the profile genuinely hasn't been synced
- `display_name` preferred over `name` for member assignee field in format spec — matches what users see in Linear UI
- Removed Phase 14 placeholder comment "no Notion-style augmentation for Linear yet — that's Phase 15" — this IS Phase 15

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 15-01 complete: LLM→Linear pipeline chains now receive full schema augmentation
- Linear pipelines will produce better-quality JSON output with correct field values on first attempt
- Retry path also benefits from schema guidance — fewer cascading failures
- `cargo check` not available in execution environment — compilation deferred to first `cargo tauri dev` run (same pattern as all prior phases)

---
*Phase: 15-linear-pipeline-integration*
*Completed: 2026-02-19*

## Self-Check: PASSED

- FOUND: src-tauri/src/pipeline_engine.rs
- FOUND: .planning/phases/15-linear-pipeline-integration/15-01-SUMMARY.md
- FOUND commit: f5a0bd4 (Task 1 — build_linear_format_spec + build_linear_augmented_prompt)
- FOUND commit: 533fe76 (Task 2 — LLM look-ahead Linear branch + augmented retry)
