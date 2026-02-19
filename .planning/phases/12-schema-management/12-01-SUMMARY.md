---
phase: 12-schema-management
plan: 01
subsystem: pipeline
tags: [rust, tauri, token-budget, notion, llm, schema-staleness]

# Dependency graph
requires:
  - phase: 11-ux-polish
    provides: augmented prompt sidecar file and per-step UI infrastructure used by this plan
provides:
  - Token budget validation gate before augmented LLM API calls (validate_augmented_prompt_budget)
  - pub estimate_tokens and pub context_limit_for_provider in llm.rs for cross-module use
  - Staleness warning UI on Notion integration cards in Settings
affects: [13-any-future-phase, pipeline_engine, llm-connector, integrations-settings]

# Tech tracking
tech-stack:
  added: []
  patterns: [pre-flight validation before expensive API calls, inline style for minimal-change UI additions]

key-files:
  created: []
  modified:
    - src-tauri/src/connectors/llm.rs
    - src-tauri/src/pipeline_engine.rs
    - src/integrations-settings.js

key-decisions:
  - "validate_augmented_prompt_budget called after sidecar write but before LLM API call — sidecar is UI display, budget check prevents API cost"
  - "Silent truncation in llm.rs retained for non-augmented path backward compatibility — new validation is explicit pre-flight for augmented path only"
  - "Inline style on staleness warning span — avoids touching styles.css for a single small element, consistent with project pattern"
  - "isStale threshold = 7 days — matches plan spec; never-synced treated as stale (most important case)"

patterns-established:
  - "Pre-flight validation pattern: check token budget before API call, not inside connector"
  - "pub helper functions in connectors for cross-module token estimation"

requirements-completed: [SCHM-01, SCHM-02]

# Metrics
duration: 5min
completed: 2026-02-19
---

# Phase 12 Plan 01: Schema Management Summary

**Token budget validation gate in pipeline engine (prevents over-limit LLM calls) and schema staleness amber warning on Notion integration cards**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-02-19T13:06:00Z
- **Completed:** 2026-02-19T13:11:01Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Added `validate_augmented_prompt_budget()` in `pipeline_engine.rs` — rejects augmented prompts exceeding model context limit before the LLM API call with a clear, actionable error message naming provider, token counts, and Notion schema as root cause
- Made `estimate_tokens` and `context_limit_for_provider` in `llm.rs` public for cross-module use by the pipeline engine
- Added schema staleness detection to Notion integration cards — shows amber warning when schema is older than 7 days or has never been synced

## Task Commits

Each task was committed atomically:

1. **Task 1: Add token budget validation before augmented LLM execution** - `d3a3508` (feat)
2. **Task 2: Add schema staleness warning to Notion integration cards** - `1362ddf` (feat)

**Plan metadata:** _(this commit)_ (docs: complete plan)

## Files Created/Modified
- `src-tauri/src/connectors/llm.rs` - Made estimate_tokens and context_limit_for_provider pub
- `src-tauri/src/pipeline_engine.rs` - Added validate_augmented_prompt_budget() and call site in Llm arm
- `src/integrations-settings.js` - Added daysSinceSync calculation and conditional staleness warning span

## Decisions Made
- `validate_augmented_prompt_budget` called after sidecar write but before `connectors::llm::execute()` — the sidecar is for UI display and is safe to write even for over-budget prompts; the budget check prevents the API call
- Silent truncation in `llm.rs` retained for non-augmented path — new validation is only for the augmented Notion path; this preserves backward compatibility
- Inline style `style="color: #e6a700; font-weight: 500;"` used on the warning span instead of adding to `styles.css` — keeps the change to one file, follows project pattern for minor UI additions
- Never-synced treated as stale (`!profile.synced_at`) — most important warning case, user needs to know before running pipelines

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed test 2-tuple destructuring to match 3-tuple parse_step_status return**
- **Found during:** Task 1 (pipeline_engine.rs modifications)
- **Issue:** Three tests in `pipeline_engine.rs` used `let (status, error) = parse_step_status(content)` but `parse_step_status` returns a 3-tuple `(String, Option<String>, Option<f64>)` — this would cause a compile error
- **Fix:** Changed all three test destructures to `let (status, error, _duration) = parse_step_status(content)`
- **Files modified:** `src-tauri/src/pipeline_engine.rs`
- **Verification:** All three test functions now match the actual return type
- **Committed in:** d3a3508 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug)
**Impact on plan:** Essential fix for compilation correctness. No scope creep.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- SCHM-01 complete: augmented prompts that exceed model context limits now fail fast with a clear error before any API call
- SCHM-02 complete: integration cards show staleness state at a glance
- Ready for Plan 12-02: re-sync schema button in pipeline builder (SCHM-03)

---
*Phase: 12-schema-management*
*Completed: 2026-02-19*
