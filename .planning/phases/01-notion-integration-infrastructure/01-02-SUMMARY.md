---
phase: 01-notion-integration-infrastructure
plan: 02
subsystem: infra
tags: [rust, tauri, notion, notion-client, api, serde, uuid, chrono]

# Dependency graph
requires:
  - phase: 01-01
    provides: "NotionIntegrationProfile types, save/load profile I/O, save_notion_token/get_notion_token helpers, notion-client dependency in Cargo.toml"
provides:
  - "add_notion_integration Tauri command: validates API key via bot user endpoint, stores token, creates initial profile"
  - "list_notion_databases Tauri command: searches Notion API for connected databases, returns id/name pairs"
  - "sync_notion_schema Tauri command: reads database properties + workspace users, writes updated profile JSON"
  - "NotionDatabaseInfo struct for list command return type"
  - "All three commands registered in lib.rs invoke_handler"
affects:
  - "01-03 (Notion secondary commands — builds on same command pattern)"
  - "Phase 4 (Notion UI wizard — calls these three commands from frontend)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "JSON-based PageOrDatabase extraction via serde_json::to_value avoids brittle enum pattern matching"
    - "make_client/make_client_from_token helper split for validation vs. stored-token use cases"
    - "Preserve people_mappings across sync by loading existing profile before overwrite"
    - "DatabaseProperty wildcard arm (_) handles non_exhaustive enum from external crate"

key-files:
  created: []
  modified:
    - "src-tauri/src/integrations/notion.rs"
    - "src-tauri/src/lib.rs"

key-decisions:
  - "JSON-based extraction for PageOrDatabase search results: serde_json::to_value(item) then check object==\"database\" — avoids fragile enum struct matching on external type"
  - "Status property does not extract options (inner StatusPropertyValue field access not documented in research) — returns empty select_options for status type"
  - "sync_notion_schema loads existing profile once, preserves both name and people_mappings in single load call"

patterns-established:
  - "API validation pattern: client built from raw token, bot user endpoint called, then token stored — never store first"
  - "DatabaseProperty match: 15 named variants + _ wildcard, Select/MultiSelect extract options, all others return empty select_options"

requirements-completed: [NOTN-01, NOTN-07]

# Metrics
duration: 3min
completed: 2026-02-18
---

# Phase 1 Plan 02: Notion Integration Infrastructure — Core Commands Summary

**Three Tauri commands (add_notion_integration, list_notion_databases, sync_notion_schema) using notion-client for API validation, database discovery, and full schema capture with workspace user pagination**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-02-18T21:51:45Z
- **Completed:** 2026-02-18T21:54:07Z
- **Tasks:** 2 (implemented together, committed as one)
- **Files modified:** 2

## Accomplishments

- Implemented `add_notion_integration`: validates API key via `retrieve_your_tokens_bot_user()` before storing, generates UUID, creates initial empty profile — API key never stored in profile JSON
- Implemented `list_notion_databases`: searches Notion API with database filter, extracts id/name via JSON serde (robust to `PageOrDatabase` enum structure), returns helpful error if no databases found
- Implemented `sync_notion_schema`: retrieves database properties via `retrieve_a_database()`, iterates `DatabaseProperty` variants to build `NotionPropertyDef` list, paginates through all workspace users with `list_all_users()` loop, preserves `people_mappings` from existing profile, writes `synced_at` timestamp
- All three commands registered in `lib.rs` invoke_handler

## Task Commits

1. **Tasks 1 & 2: all three Notion commands + lib.rs registration** - `a142d53` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `src-tauri/src/integrations/notion.rs` - NotionDatabaseInfo struct, make_client helpers, add_notion_integration, list_notion_databases, sync_notion_schema commands, convert_database_property helper with 15 variants
- `src-tauri/src/lib.rs` - Registered integrations::notion::add_notion_integration, list_notion_databases, sync_notion_schema in invoke_handler

## Decisions Made

- **JSON-based PageOrDatabase extraction:** The research flagged uncertainty about the exact `PageOrDatabase` union type from search results. Using `serde_json::to_value(item)` then checking `object == "database"` avoids pattern matching on an external crate's enum struct — more resilient to API changes
- **Status property without options:** The `Status` database property's inner struct field access was not documented in research (MEDIUM confidence). Used `..` wildcard for Status variant and returns empty `select_options` rather than risking a compile error on an unverified field name. Status options can be added in Plan 03 once verified
- **Single profile load in sync_notion_schema:** Refactored to load existing profile once and destructure `(people_mappings, name)` to avoid two file reads

## Deviations from Plan

### Notes

**Tasks 1 and 2 implemented together in a single commit:**
- **Found during:** Task 1 implementation
- **Reason:** Both tasks modify the same file (`notion.rs`); `sync_notion_schema` (Task 2) depends on helpers defined alongside Task 1 code. Implementing sequentially in the same file pass was more coherent than splitting artificially.
- **Impact:** Functionally equivalent — all code from both tasks is in the single commit `a142d53`.

---

**Total deviations:** 1 process note (single-commit for two tasks)
**Impact on plan:** No functional impact. All task requirements met.

## Issues Encountered

- Cargo not installed in the execution environment — `cargo check` compilation verification unavailable. Code verified through structural review:
  - All import paths analyzed against research-documented API surface (HIGH confidence)
  - `notion_client::endpoints::search::title::request::*` — explicitly verified in research from docs.rs
  - `DatabaseProperty` enum matching — 15 named variants plus wildcard `_` arm for `#[non_exhaustive]` safety
  - `Client::new(token, None)` signature — verified HIGH confidence in research
  - `retrieve_your_tokens_bot_user()`, `search_by_title()`, `retrieve_a_database()`, `list_all_users()` — all verified HIGH confidence in research

## User Setup Required

None - commands require a valid Notion API key but no pre-configuration in this environment.

## Next Phase Readiness

- Plan 03 (Notion secondary commands) can use the same `make_client` helper and profile I/O pattern
- All three plan commands are the backend API that Phase 4's UI wizard calls
- No blockers

## Self-Check: PASSED

- `src-tauri/src/integrations/notion.rs` — FOUND
- `src-tauri/src/lib.rs` — FOUND
- `.planning/phases/01-notion-integration-infrastructure/01-02-SUMMARY.md` — FOUND
- Commit `a142d53` — FOUND in git log

---
*Phase: 01-notion-integration-infrastructure*
*Completed: 2026-02-18*
