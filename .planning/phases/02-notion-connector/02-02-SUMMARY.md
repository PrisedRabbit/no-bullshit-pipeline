---
phase: 02-notion-connector
plan: 02
subsystem: pipelines
tags: [rust, tauri, pipelines, connector-type, pipeline-engine, serde, notion]

# Dependency graph
requires:
  - phase: 02-notion-connector
    plan: 01
    provides: "connectors/notion.rs with execute() entry point, pub mod notion in connectors/mod.rs"
provides:
  - "ConnectorType::Notion variant in pipelines.rs (serializes as 'notion')"
  - "validate_step_config() Notion arm requiring integration_id in config"
  - "pipeline_engine.rs ConnectorType::Notion match arm dispatching to connectors::notion::execute()"
  - "4 new unit tests covering Notion serialization, deserialization, and validation"
affects:
  - "03 (prompt augmentation) — Notion connector fully reachable from pipeline engine"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ConnectorType enum extended by appending new variant after Mcp — maintain enum ordering for readability"
    - "validate_step_config Notion arm mirrors Slack pattern (integration_id required, single config key)"
    - "pipeline_engine.rs Notion arm uses standard 6-arg connector signature — no imports needed (connectors already in scope)"

key-files:
  created: []
  modified:
    - "src-tauri/src/pipelines.rs"
    - "src-tauri/src/pipeline_engine.rs"

key-decisions:
  - "Notion validation requires only integration_id (not database_id) — database_id and other config come from the stored integration profile loaded at execute() time"
  - "Match arm placed between Slack and Mcp arms — preserves logical ordering (implemented connectors before stub)"

patterns-established:
  - "Pipeline wiring pattern: add enum variant → add validate_step_config arm → add pipeline_engine match arm → add tests"

requirements-completed: [NOTN-08, EXEC-04]

# Metrics
duration: 2min
completed: 2026-02-18
---

# Phase 2 Plan 02: Pipeline Wiring — ConnectorType::Notion Summary

**ConnectorType::Notion wired into pipeline system: enum variant with serde lowercase serialization, validate_step_config arm requiring integration_id, and pipeline_engine.rs match arm dispatching to connectors::notion::execute() using the standard 6-argument connector signature**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-02-18T22:26:26Z
- **Completed:** 2026-02-18T22:28:30Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Added `Notion` variant to `ConnectorType` enum in `pipelines.rs` — serializes as `"notion"` via `#[serde(rename_all = "lowercase")]`
- Added `ConnectorType::Notion` arm to `validate_step_config()` requiring `integration_id` in config (mirrors Slack pattern; only one key required since database_id and other config come from stored integration profile)
- Wired `ConnectorType::Notion` match arm in `pipeline_engine.rs` dispatching to `connectors::notion::execute()` with standard 6-argument signature
- Added 4 new unit tests: serialization (`"notion"`), deserialization (`"notion"` → `Notion`), missing `integration_id` fails, valid config passes
- Extended existing `test_connector_type_serialization` test to cover all 6 connector types (previously missing Slack and Notion assertions)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add ConnectorType::Notion to pipelines.rs with validation** - `de8a355` (feat)
2. **Task 2: Wire ConnectorType::Notion match arm in pipeline_engine.rs** - `199728c` (feat)

## Files Created/Modified

- `src-tauri/src/pipelines.rs` — Notion variant added to ConnectorType enum, Notion arm added to validate_step_config(), 4 new tests added, existing serialization test extended
- `src-tauri/src/pipeline_engine.rs` — ConnectorType::Notion arm added dispatching to connectors::notion::execute()

## Decisions Made

- **integration_id only in validation (not database_id):** The `database_id` and other Notion-specific config come from the stored integration profile loaded inside `execute()`. Requiring only `integration_id` at validation time keeps the config minimal and mirrors the Slack connector pattern exactly.
- **Match arm between Slack and Mcp:** Preserves logical ordering — fully implemented connectors (Llm, Save, Webhook, Slack, Notion) before the stub (Mcp).

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

- `cargo check` is not available in the execution environment (consistent with all Phase 1 and Phase 2 Plan 01). Code verified structurally:
  - `Notion` variant appended to `ConnectorType` enum with `#[serde(rename_all = "lowercase")]` — serializes as `"notion"`
  - `ConnectorType::Notion` arm in `validate_step_config()` checks `integration_id` via `.get("integration_id").and_then(|v| v.as_str()).is_none()`
  - `ConnectorType::Notion` arm in `pipeline_engine.rs` uses exact same 6 arguments as `slack::execute()` — both return `Result<PathBuf, String>`
  - `crate::connectors` already imported in `pipeline_engine.rs` — `connectors::notion::execute()` is reachable via `pub mod notion` added in Plan 02-01
  - All 4 new tests follow existing test patterns in the module

## Next Phase Readiness

- Phase 2 complete — Notion connector (`notion.rs`) implemented and wired into the pipeline engine
- Phase 3 (prompt augmentation) can now target the Notion connector end-to-end: LLM step produces JSON output → Notion connector step posts to Notion database
- No changes needed to `notion.rs` — the connector is complete and the pipeline engine can now dispatch to it

## Self-Check: PASSED

- src-tauri/src/pipelines.rs: FOUND
- src-tauri/src/pipeline_engine.rs: FOUND
- .planning/phases/02-notion-connector/02-02-SUMMARY.md: FOUND
- Commit de8a355: FOUND
- Commit 199728c: FOUND

---
*Phase: 02-notion-connector*
*Completed: 2026-02-18*
