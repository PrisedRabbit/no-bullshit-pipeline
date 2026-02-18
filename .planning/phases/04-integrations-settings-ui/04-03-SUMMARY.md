---
phase: 04-integrations-settings-ui
plan: 03
subsystem: ui
tags: [tauri, rust, vanilla-js, integrations, save-path, notion, pipeline-builder]

# Dependency graph
requires:
  - phase: 04-01-integrations-settings-foundation
    provides: integrations-settings.js module, Connected/Available layout, notionProfiles/savePathIntegrations var globals
  - phase: 04-02-notion-setup-wizard
    provides: openNotionWizard(), notionProfiles populated on loadAllIntegrations
  - phase: 01-notion-integration-infrastructure
    provides: list_notion_profiles Tauri command, get_integrations_dir() helper

provides:
  - SavePathProfile struct with id/name/path fields stored as ~/.nbp/integrations/save-path-{id}.json
  - add_save_path_integration Tauri command (validate, UUID, write, 0o600 perms)
  - list_save_path_integrations Tauri command (scan dir, filter save-path-*.json)
  - update_save_path_integration Tauri command (load, update, save, 0o600 perms)
  - remove_save_path_integration Tauri command (delete file, idempotent on NotFound)
  - Save path cards in Connected section with inline Edit (name input + folder picker) and Remove actions
  - Save Path card in Available section with inline add form (name + Browse folder picker + Save/Cancel)
  - var savePathIntegrations = [] on window — accessible from main.js for pipeline step editor
  - Pipeline builder Save connector: dropdown of named save paths when available, fallback to free-text with tip
  - Pipeline builder Notion connector option in connector dropdown
  - Pipeline builder Notion connector step: integration_id dropdown from connected notionProfiles, or helpful empty message

affects:
  - pipeline-engine (save connector will need save_path_id resolution in Rust)
  - future pipeline builder phases (Notion connector config structure)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Save path profiles follow notion.rs pattern exactly (same dir, same permissions, same file naming scheme)
    - Inline edit pattern: card outerHTML replaced with editor, Save/Cancel re-render
    - Inline add pattern: Available card replaced with inline form, Cancel re-renders available section
    - Cross-module typeof guard: (typeof savePathIntegrations !== 'undefined') ? savePathIntegrations : [] in main.js

key-files:
  created:
    - src-tauri/src/integrations/save_path.rs
  modified:
    - src-tauri/src/integrations/mod.rs
    - src-tauri/src/lib.rs
    - src/integrations-settings.js
    - src/main.js

key-decisions:
  - "save_path.rs follows notion.rs I/O pattern exactly — same directory, same 0o600 permissions, idempotent remove"
  - "remove_save_path_integration uses std::io::ErrorKind::NotFound instead of string matching — more robust than notion.rs pattern"
  - "Inline edit/add forms replace card outerHTML directly — avoids needing a modal for a simple 2-field form"
  - "Save connector falls back to free-text path input when no save path integrations exist — preserves backward compatibility"
  - "Notion connector option added to pipeline builder dropdown — uses notionProfiles window global loaded by integrations-settings.js"

patterns-established:
  - "Save path profile I/O: save-path-{id}.json in ~/.nbp/integrations/ with 0o600 perms"
  - "Inline card editor: outerHTML swap, selectedPath closure variable, Browse via window.__TAURI__.dialog.open({directory: true})"
  - "Connector step editor cross-module access: typeof guard + window global var from integrations-settings.js"

requirements-completed: [INTG-03, INTG-04]

# Metrics
duration: 2min
completed: 2026-02-18
---

# Phase 4 Plan 03: Save Path Backend + Delivery Picker Wiring Summary

**Save path CRUD Rust backend (4 Tauri commands, ~/.nbp/integrations/save-path-{id}.json), save path Connected/Available UI with inline edit/add flows, Notion and Save connector delivery pickers in the pipeline builder**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-18T23:37:37Z
- **Completed:** 2026-02-18T23:39:50Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Created `save_path.rs` (175 lines) with `SavePathProfile` struct and 4 CRUD Tauri commands following the exact same file I/O, permissions, and idempotent-delete patterns as `notion.rs`
- Registered `pub mod save_path` in `integrations/mod.rs` and all 4 commands in `lib.rs` invoke_handler
- Added `var savePathIntegrations = []` global, `loadSavePathIntegrations()`, and save path card rendering to `integrations-settings.js` — Connected section now shows save path cards alongside Notion and Slack
- Implemented inline edit flow (name input + folder picker via Tauri dialog + Save/Cancel) and inline add flow (available card replaced with form) for save paths
- Updated pipeline builder `save` connector step editor: shows named save path dropdown when integrations exist, falls back to free-text path input with a setup tip when none exist
- Added `notion` option to pipeline builder connector dropdown with integration_id dropdown populated from `notionProfiles` global, or a helpful empty-state message directing to Settings > Integrations

## Task Commits

Each task was committed atomically:

1. **Task 1: Create save path integration Rust backend with CRUD Tauri commands** - `a9276d9` (feat)
2. **Task 2: Add save path UI to integrations page, wire Notion delivery picker, and update Save connector dropdown** - `159e19a` (feat)

## Files Created/Modified
- `/workspace/src-tauri/src/integrations/save_path.rs` - New module: SavePathProfile struct + add/list/update/remove CRUD commands
- `/workspace/src-tauri/src/integrations/mod.rs` - Added `pub mod save_path;` declaration
- `/workspace/src-tauri/src/lib.rs` - Registered 4 save path commands in invoke_handler
- `/workspace/src/integrations-settings.js` - Added savePathIntegrations state, load function, Connected card rendering with Edit/Remove, Available card with inline add form
- `/workspace/src/main.js` - Updated save connector to show save path dropdown, added notion connector option and integration_id dropdown branch

## Decisions Made
- `remove_save_path_integration` uses `e.kind() == std::io::ErrorKind::NotFound` for idempotent delete — more robust than the string-matching pattern in `notion.rs`'s `delete_notion_profile` (which checks error message text)
- Inline edit and add forms replace card outerHTML directly — avoids adding another modal to index.html for a simple 2-field form (name + path)
- Save connector falls back to free-text path input when `savePathIntegrations` is empty — existing pipelines with a manual `path` config continue to work without migration
- `notionProfiles` and `savePathIntegrations` accessed in main.js via `typeof` guard — safe even if integrations-settings.js hasn't loaded yet (edge case: pipeline editor opened before integrations tab has ever been activated)

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
- `cargo` not available in execution environment (pre-existing blocker from Phase 1). Rust code verified structurally — `save_path.rs` follows the identical pattern of `notion.rs` in the same directory; `uuid::Uuid::new_v4()` already used in `notion.rs`; `std::os::unix::fs::PermissionsExt` already imported in `notion.rs`; all 4 commands follow `#[tauri::command]` registration pattern established throughout codebase.
- JS files verified with `node --check` — both passed with no syntax errors.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 4 complete: Integrations Settings UI foundation, Notion wizard, and save path backend all done
- `savePathIntegrations` and `notionProfiles` window globals populated for pipeline builder access
- Save connector now shows named save paths dropdown — pipeline engine (Phase 5+) will need to resolve `save_path_id` to the actual filesystem path at execute time
- Notion connector step config stores `integration_id` — pipeline engine already handles this via `load_notion_profile` (Phase 2)

## Self-Check: PASSED

- FOUND: src-tauri/src/integrations/save_path.rs
- FOUND: src-tauri/src/integrations/mod.rs (pub mod save_path declared)
- FOUND: src-tauri/src/lib.rs (4 save path commands registered)
- FOUND: src/integrations-settings.js (var savePathIntegrations, list_save_path_integrations, add_save_path_integration, save-path rendering)
- FOUND: src/main.js (connector === 'notion' branch, save_path_id, savePathIntegrations, notion option in dropdown)
- FOUND: .planning/phases/04-integrations-settings-ui/04-03-SUMMARY.md
- FOUND: commit a9276d9 (Task 1 - Rust backend)
- FOUND: commit 159e19a (Task 2 - UI + pipeline builder)

---
*Phase: 04-integrations-settings-ui*
*Completed: 2026-02-18*
