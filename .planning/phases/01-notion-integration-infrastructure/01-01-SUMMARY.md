---
phase: 01-notion-integration-infrastructure
plan: 01
subsystem: infra
tags: [rust, tauri, notion, keychain, security-framework, serde, dev-mode-bypass]

# Dependency graph
requires: []
provides:
  - "integrations/ module directory with mod.rs, slack.rs, notion.rs"
  - "Shared save_token/get_token/delete_token credential helpers with dev-mode bypass"
  - "NotionIntegrationProfile struct and profile I/O functions"
  - "get_integrations_dir() config helper returning ~/.nbp/integrations/"
  - "notion-client 1.0.11 dependency in Cargo.toml"
affects:
  - "01-02 (Notion core commands — builds on NotionIntegrationProfile and token helpers)"
  - "01-03 (Notion secondary commands — builds on profile I/O and schema types)"

# Tech tracking
tech-stack:
  added: ["notion-client = 1.0.11"]
  patterns:
    - "#[cfg(debug_assertions)] / #[cfg(not(debug_assertions))] credential helper split"
    - "Module directory pattern for integrations (mod.rs + submodules)"
    - "super::save_token / super::get_token delegation from submodule to mod.rs shared helpers"
    - "Profile I/O as plain JSON files in ~/.nbp/integrations/ with 0o600 permissions"

key-files:
  created:
    - "src-tauri/src/integrations/mod.rs"
    - "src-tauri/src/integrations/slack.rs"
    - "src-tauri/src/integrations/notion.rs"
  modified:
    - "src-tauri/Cargo.toml"
    - "src-tauri/src/config.rs"
    - ".gitignore"

key-decisions:
  - "Dev-mode credential bypass uses .dev-credentials.json (gitignored) at project root, avoiding macOS Keychain permission dialogs during development"
  - "get_integrations_dir() returns ~/.nbp/integrations/ (matching existing ~/.nbp/ config root convention, not ~/.nbp/config/integrations/)"
  - "IntegrationsConfig re-exported from mod.rs so crate::integrations::IntegrationsConfig path unchanged in config.rs"

patterns-established:
  - "Credential helper pattern: save_token/get_token/delete_token in mod.rs, submodules delegate via super::"
  - "Dev bypass: #[cfg(debug_assertions)] writes JSON file, #[cfg(not(debug_assertions))] uses macOS Keychain"
  - "Profile I/O: separate JSON files per integration, named notion-{id}.json, 0o600 permissions"

requirements-completed: [NOTN-02, NOTN-06, INTG-05]

# Metrics
duration: 4min
completed: 2026-02-18
---

# Phase 1 Plan 01: Notion Integration Infrastructure — Module Foundation Summary

**Integrations module refactored to directory structure with shared dev-mode Keychain bypass, notion-client dependency, and NotionIntegrationProfile types with profile I/O ready for Plan 02 Tauri commands**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-02-18T21:43:11Z
- **Completed:** 2026-02-18T21:46:30Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Restructured `integrations.rs` flat file into `integrations/` module directory (`mod.rs` + `slack.rs` + `notion.rs`) — all existing Slack command paths in `invoke_handler` remain unchanged
- Implemented shared credential helpers (`save_token`/`get_token`/`delete_token`) with `#[cfg(debug_assertions)]` dev-mode bypass writing to `.dev-credentials.json`, and macOS Keychain in release builds
- Defined `NotionIntegrationProfile`, `NotionPropertyDef`, `PeopleMapping`, `WorkspaceUser` structs with full profile I/O (`save_notion_profile`, `load_notion_profile`, `delete_notion_profile`, `list_notion_profiles`)

## Task Commits

Each task was committed atomically:

1. **Task 1: Restructure integrations module and dev-mode bypass** - `8fe2bd9` (feat)
2. **Task 2: Notion profile types, profile I/O, config helper** - `c5ce8e5` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `src-tauri/src/integrations/mod.rs` - KEYCHAIN_SERVICE, IntegrationsConfig, shared credential helpers with dev-mode bypass
- `src-tauri/src/integrations/slack.rs` - All Slack code migrated, internal token helpers now delegate to `super::save_token/get_token/delete_token`
- `src-tauri/src/integrations/notion.rs` - NotionIntegrationProfile and supporting types, save/load/delete/list profile I/O, token helper wrappers
- `src-tauri/Cargo.toml` - Added `notion-client = "1.0.11"`
- `src-tauri/src/config.rs` - Added `get_integrations_dir()` returning `~/.nbp/integrations/`
- `.gitignore` - Added `.dev-credentials.json`

## Decisions Made

- `get_integrations_dir()` returns `~/.nbp/integrations/` rather than `~/.nbp/config/integrations/` — the existing codebase uses `~/.nbp/` as the config root (settings.json, templates/, models/ all live there directly), so integrations follows the same pattern
- Dev bypass writes to `.dev-credentials.json` at the project working directory (where `cargo tauri dev` runs from), not at `~/.nbp/` — keeps credentials accessible to the running process without touching user config
- `pub use slack::*` in `mod.rs` preserves backward-compatible re-exports so `crate::integrations::get_slack_token` path still works in `connectors/slack.rs`

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Rust toolchain (`cargo`) is not installed in the execution environment, so compilation could not be verified with `cargo check`. Code was verified through structural review:
  - All import paths analyzed against existing module references
  - `crate::integrations::IntegrationsConfig` — satisfied by `pub struct IntegrationsConfig` in `mod.rs`
  - `crate::integrations::get_slack_token` — satisfied by `pub use slack::*` re-export in `mod.rs`
  - All 5 Slack Tauri commands (`list_slack_integrations`, `add_slack_integration`, `remove_slack_integration`, `test_slack_integration`, `list_slack_channels`) — moved verbatim to `slack.rs`, re-exported, invoke_handler paths unchanged in `lib.rs`
  - `#[cfg(debug_assertions)]` / `#[cfg(not(debug_assertions))]` blocks validated for mutual exclusivity

## User Setup Required

None - no external service configuration required for this plan. The dev-mode bypass (`save_token`/`get_token`) is transparent; `.dev-credentials.json` is created automatically when credentials are saved.

## Next Phase Readiness

- Plan 02 (Notion core commands) can now implement: `add_notion_integration`, `remove_notion_integration`, `list_notion_integrations`, `sync_notion_schema` as Tauri commands using `save_notion_token`/`get_notion_token` and `save_notion_profile`/`load_notion_profile`
- `notion-client` dependency is available for API calls in Plan 02
- No blockers

---
*Phase: 01-notion-integration-infrastructure*
*Completed: 2026-02-18*
