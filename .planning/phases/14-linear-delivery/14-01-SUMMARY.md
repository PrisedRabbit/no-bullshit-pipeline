---
phase: 14-linear-delivery
plan: 01
status: done
commit: d413ca0
---

## What Was Done

### Task 1: Linear connector module (connectors/linear.rs)

Created `src-tauri/src/connectors/linear.rs` following the Notion connector pattern with these components:

- **Error types**: `LinearErrorKind::JsonParse` (retryable) and `LinearErrorKind::Other` (non-retryable), with `Display` and `From<LinearError> for String` impls
- **Config**: `LinearConnectorConfig` parsed from step config `integration_id`
- **JSON extraction**: `extract_json_object()` extracts a single JSON object (not array) from bare JSON, ```json fences, or bare ``` fences
- **Validation**: `validate_llm_output_for_linear()` checks the object has at least one known Linear field (title, description, priority, status, labels, assignee)
- **Field resolution** (4 functions, all case-insensitive):
  - `resolve_status()` → matches workflow state name → returns state ID
  - `resolve_priority()` → matches label string ("High") or validates 0-4 integer → returns i32
  - `resolve_labels()` → matches array/single string of label names → returns Vec of label IDs
  - `resolve_assignee()` → matches member name or display_name → returns member ID
- **GraphQL mutation**: `build_issue_input()` builds `IssueCreateInput` variables, `create_issue()` sends the mutation via shared `graphql_request` helper
- **Output helpers**: `write_success_output()` and `write_failure_output()` with Linear-specific frontmatter
- **Execute entry points**: `execute()`, `execute_structured()`, `execute_with_raw_preservation()` — all three calling shared `execute_inner()`

Also made `graphql_request` in `integrations/linear.rs` `pub(crate)` so the connector can reuse it.

### Task 2: Pipeline integration

- Added `ConnectorType::Linear` to the enum in `pipelines.rs`
- Added Linear to `is_delivery()` match (delivery connector)
- Added `integration_id` validation in `validate_step_config()`
- Added `ConnectorType::Linear` match arm in `pipeline_engine.rs` with full JSON retry logic:
  - Calls `execute_structured()` first
  - On `JsonParse` error: finds preceding LLM step, builds original prompt from step config, calls `execute_retry`, then `execute_with_raw_preservation` with retry output
  - On retry failure: writes failure output with raw AI output preserved
  - On `Other` error: no retry, surfaces error directly

## Deviations

None. Implementation follows the plan exactly.

## Verification

All structural checks pass:
- All 13 required identifiers present in `connectors/linear.rs`
- Module registered in `connectors/mod.rs`
- `graphql_request` is `pub(crate)` in `integrations/linear.rs`
- `ConnectorType::Linear` in enum with `is_delivery=true` and config validation
- Pipeline engine Linear arm references `execute_structured`, `execute_with_raw_preservation`, `execute_retry`
- All error messages correctly reference "Linear" (not "Notion")
- JSON extraction handles single object (not array) — key difference from Notion

Note: cargo check unavailable in execution environment — compilation deferred to first `cargo tauri dev` run.
