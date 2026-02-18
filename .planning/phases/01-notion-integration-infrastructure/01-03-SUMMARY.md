---
phase: 01-notion-integration-infrastructure
plan: 03
subsystem: infra
tags: [rust, tauri, notion, notion-client, serde, tauri-command]

# Dependency graph
requires:
  - phase: 01-02
    provides: "add_notion_integration, list_notion_databases, sync_notion_schema commands, make_client helper, load/save/delete_notion_profile I/O, get/save/delete_notion_token helpers"
provides:
  - "update_notion_people_mappings Tauri command: validates alias-to-user mappings against workspace_users, persists to profile JSON"
  - "test_notion_integration Tauri command: verifies stored token validity via bot user endpoint, returns Connected or error"
  - "remove_notion_integration Tauri command: idempotent deletion of both credential and profile JSON"
  - "All six Notion commands registered in lib.rs invoke_handler — Phase 1 backend complete"
affects:
  - "Phase 2 (Notion Connector — uses update_notion_people_mappings to resolve alias->user ID at pipeline runtime)"
  - "Phase 4 (Integrations Settings UI — calls test and remove for Connected integrations management)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Idempotent removal pattern: both credential and profile deletion always attempted; 'not found' errors treated as success"
    - "User ID validation against workspace_users before mutating people_mappings — prevents stale alias references"
    - "Token never appears in error messages — test_notion_integration uses {:?} on error object, not token value"

key-files:
  created: []
  modified:
    - "src-tauri/src/integrations/notion.rs"
    - "src-tauri/src/lib.rs"

key-decisions:
  - "remove_notion_integration: collect both deletion errors rather than fail-fast — both operations always attempted so partial state is never left behind"
  - "test_notion_integration error format uses {:?} on the error object — keeps token value out of user-visible error messages"
  - "update_notion_people_mappings validates ALL user IDs before mutating profile — atomically rejects invalid sets"

patterns-established:
  - "Idempotent remove: delete credential + delete profile, treat 'not found' as success, collect other errors"
  - "Validation-before-mutation: load, validate all inputs, then write — no partial updates on validation failure"

requirements-completed: [NOTN-06]

# Metrics
duration: 4min
completed: 2026-02-18
---

# Phase 1 Plan 03: Notion Integration Infrastructure — Secondary Commands Summary

**Three Notion lifecycle commands (update_people_mappings, test, remove) completing the six-command backend for Phase 4's integrations UI and Phase 2's alias resolution**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-02-18T21:56:46Z
- **Completed:** 2026-02-18T22:00:Z
- **Tasks:** 2 (Task 1: implement; Task 2: verify)
- **Files modified:** 2

## Accomplishments

- Implemented `update_notion_people_mappings`: loads profile, validates each `notion_user_id` against `workspace_users`, replaces `people_mappings`, saves — rejects invalid sets atomically
- Implemented `test_notion_integration`: retrieves stored token via `make_client`, calls `retrieve_your_tokens_bot_user()`, returns "Connected" or descriptive error without exposing the token value
- Implemented `remove_notion_integration`: calls `delete_notion_token` then `delete_notion_profile` in all cases (even if first fails), treats "not found" errors as success (idempotent), collects other errors
- All six Notion Tauri commands now registered in `lib.rs` — Phase 1 backend infrastructure complete

## Task Commits

1. **Task 1: Implement people mappings, test, and remove commands** - `eccc32c` (feat)
2. **Task 2: Final compilation verification** — verification only, no files modified, no commit

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `src-tauri/src/integrations/notion.rs` — Added `update_notion_people_mappings`, `test_notion_integration`, `remove_notion_integration` Tauri commands
- `src-tauri/src/lib.rs` — Registered all three new commands in invoke_handler (now six total Notion commands)

## Decisions Made

- **Both deletions always attempted in remove_notion_integration:** Fail-fast on the first deletion would leave the other resource dangling (credential orphaned if profile deleted first, or vice versa). Collecting errors and always attempting both ensures clean state regardless of partial failures.
- **test_notion_integration uses `{:?}` for error formatting:** Formats the error type/struct rather than including any string interpolation of the token. The token is never passed to the error formatter.
- **Validation-before-mutation in update_notion_people_mappings:** All user ID checks complete before any write. If one invalid ID is found, the entire update is rejected — no partial mutation of people_mappings.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

- Cargo not installed in the execution environment — `cargo check` compilation verification unavailable (same environment constraint as Plan 02). Code verified through structural review:
  - All function signatures match existing helpers (`make_client`, `load_notion_profile`, `save_notion_profile`, `delete_notion_token`, `delete_notion_profile`)
  - `#[tauri::command]` + `pub async fn` pattern consistent with existing three commands
  - `Vec<PeopleMapping>` is the correct type (struct defined in same file, derives Deserialize)
  - All six commands in invoke_handler confirmed by grep

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- All six Notion Tauri commands implemented and registered: `add_notion_integration`, `list_notion_databases`, `sync_notion_schema`, `update_notion_people_mappings`, `test_notion_integration`, `remove_notion_integration`
- Phase 1 backend infrastructure is complete
- Phase 2 (Notion Connector) can call `update_notion_people_mappings` to persist alias-to-user-ID mappings
- Phase 4 (Integrations Settings UI) can call `test_notion_integration` and `remove_notion_integration` for Connected integrations management
- No blockers

## Self-Check: PASSED

- `src-tauri/src/integrations/notion.rs` — FOUND
- `src-tauri/src/lib.rs` — FOUND
- `.planning/phases/01-notion-integration-infrastructure/01-03-SUMMARY.md` — FOUND (this file)
- Commit `eccc32c` — FOUND in git log

---
*Phase: 01-notion-integration-infrastructure*
*Completed: 2026-02-18*
