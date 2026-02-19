---
phase: 07-pipeline-data-model-and-tags-migration
plan: 01
subsystem: database
tags: [rust, serde, pipeline, migration, metadata, storage]

# Dependency graph
requires:
  - phase: 06-pre-assignment-ux-and-execution
    provides: PipelineState type used in pipeline_engine.rs (now moved to pipelines.rs)
provides:
  - PipelineStatus, PipelineState, StepStatus, PipelineProgressPayload defined in pipelines.rs as shared types
  - RecordingMetadata.pipelines: Vec<PipelineState> field with serde(default) for backward compatibility
  - migrate_tags_to_pipeline_labels() lazy migration function in storage.rs
  - sanitize_pipeline_name() helper in storage.rs
  - Zero-step pipeline validation removed from validate_pipeline()
affects:
  - 07-02-pipeline-data-model-and-tags-migration
  - pipeline_engine.rs (re-imports types from pipelines.rs)
  - storage.rs (holds typed pipeline references)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Shared type module pattern: execution state types (PipelineState, PipelineStatus) live in pipelines.rs, imported by both pipeline_engine.rs and storage.rs
    - Lazy migration pattern: migrate on read (not batch) — read_metadata() and list_recordings() trigger tag-to-label migration transparently
    - Migration error swallowing: let _ = migrate_tags_to_pipeline_labels() so migration failure never blocks recording access

key-files:
  created: []
  modified:
    - src-tauri/src/pipelines.rs
    - src-tauri/src/pipeline_engine.rs
    - src-tauri/src/storage.rs

key-decisions:
  - "PipelineState PartialEq added to satisfy RecordingMetadata: PartialEq derive bound — RecordingMetadata derives PartialEq and its pipelines field is Vec<PipelineState>"
  - "Intra-crate circular module references (storage->pipelines->storage via get_data_dir) are valid in Rust — compiler sees whole crate at once"
  - "PipelineStatus::Done used for migrated labels (not Waiting) — label has no steps to execute, Done is semantically correct"
  - "Migration loads pipelines once, inserts all new entries, saves once — not per-tag to avoid repeated disk I/O"
  - "tags field retained permanently — never deleted for backward compatibility"

patterns-established:
  - "Type consolidation: runtime state types belong in the module that owns the concept (pipelines.rs) not the execution engine"
  - "Migration idempotency: filter by existing_names set before creating entries, so running twice produces same result"

requirements-completed: [PIPE-01, PIPE-03, PIPE-04]

# Metrics
duration: 3min
completed: 2026-02-19
---

# Phase 7 Plan 01: Pipeline Data Model and Tags Migration Summary

**PipelineState/PipelineStatus types consolidated to pipelines.rs, RecordingMetadata gains typed pipelines field, lazy tag-to-pipeline-label migration wired into both read paths**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-19T05:00:48Z
- **Completed:** 2026-02-19T05:03:48Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Moved PipelineStatus, PipelineState, StepStatus, PipelineProgressPayload from pipeline_engine.rs to pipelines.rs as shared types
- Added `pipelines: Vec<PipelineState>` field to RecordingMetadata with `#[serde(default)]` for backward compatibility with existing metadata.json files
- Implemented `migrate_tags_to_pipeline_labels()` — lazy, idempotent migration that creates zero-step pipeline definitions and Done pipeline states for existing tags
- Wired migration into both read paths (read_metadata and list_recordings) with error-swallowing so migration failure never blocks recording access
- Removed zero-step pipeline validation guard from validate_pipeline() and updated test to assert Ok

## Task Commits

Each task was committed atomically:

1. **Task 1: Move PipelineState types to pipelines.rs and remove zero-step validation guard** - `130d274` (feat)
2. **Task 2: Add pipelines field to RecordingMetadata and implement lazy tag migration** - `7fc3abf` (feat)

## Files Created/Modified

- `src-tauri/src/pipelines.rs` - Added PipelineStatus, PipelineState, StepStatus, PipelineProgressPayload definitions; removed zero-step validation guard; renamed test_empty_steps_fails to test_empty_steps_passes
- `src-tauri/src/pipeline_engine.rs` - Replaced local type definitions with imports from crate::pipelines; removed unused serde import
- `src-tauri/src/storage.rs` - Added PipelineState import; added pipelines field to RecordingMetadata; added sanitize_pipeline_name() and migrate_tags_to_pipeline_labels(); wired migration into read_metadata() and list_recordings()

## Decisions Made

- Added `PartialEq` derive to `PipelineState` to satisfy `RecordingMetadata: PartialEq` bound — RecordingMetadata already had PartialEq, and adding pipelines: Vec<PipelineState> requires PipelineState to also implement PartialEq
- Intra-crate circular module references (storage -> pipelines -> storage via get_data_dir) are valid in Rust — the compiler sees the whole crate at once
- `PipelineStatus::Done` used for migrated labels — a label has no steps to run, so Done is the correct terminal state
- Migration loads all pipelines once, adds all missing entries in a loop, then saves once — avoids N disk writes for N tags

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added PartialEq to PipelineState**
- **Found during:** Task 2 (Add pipelines field to RecordingMetadata)
- **Issue:** RecordingMetadata derives PartialEq. Adding `pipelines: Vec<PipelineState>` requires PipelineState to implement PartialEq. The plan did not account for this constraint.
- **Fix:** Added `PartialEq` to PipelineState's derive list in pipelines.rs
- **Files modified:** src-tauri/src/pipelines.rs
- **Verification:** PipelineStatus already had PartialEq; PipelineState fields are all types that implement PartialEq (String, Option<String>, Option<usize>, PipelineStatus)
- **Committed in:** 7fc3abf (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug — missing PartialEq derive)
**Impact on plan:** Necessary for correctness, no scope creep.

## Issues Encountered

- `cargo check` not available in execution environment (existing environment blocker from Phase 1). Structural code review performed instead. Real compilation deferred to first `cargo tauri dev` run.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 07-02 can now implement zero-step early return in pipeline_engine.execute_pipeline_internal() — it will check `pipeline.steps.is_empty()` and return Done immediately for label pipelines
- Phase 07-02 can remove the frontend zero-step guard in pipeline-builder.js
- Types are consolidated in pipelines.rs — no further refactoring needed for the data model

## Self-Check: PASSED

All files verified present:
- FOUND: src-tauri/src/pipelines.rs
- FOUND: src-tauri/src/pipeline_engine.rs
- FOUND: src-tauri/src/storage.rs
- FOUND: 07-01-SUMMARY.md

All commits verified:
- FOUND: 130d274 (Task 1 - types moved, zero-step guard removed)
- FOUND: 7fc3abf (Task 2 - pipelines field + migration)

---
*Phase: 07-pipeline-data-model-and-tags-migration*
*Completed: 2026-02-19*
