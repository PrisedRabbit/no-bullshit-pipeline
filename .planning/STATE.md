# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-18)

**Core value:** Zero post-recording work — select pipeline, record, stop, everything happens automatically.
**Current focus:** Phase 2 (Notion Connector) — IN PROGRESS (1/2 plans complete)

## Current Position

Phase: 2 of 8 (Notion Connector) — IN PROGRESS
Plan: 1 of 2 in current phase — plan 01 complete, plan 02 ready
Status: 02-01 complete — connectors/notion.rs implemented with execute(), JSON extraction, property mapping
Last activity: 2026-02-18 — 02-01 executed (connectors/notion.rs created)

Progress: [█░░░░░░░░░] 17% (4/23 plans)

## Performance Metrics

**Velocity:**
- Total plans completed: 4
- Average duration: 3.8 min
- Total execution time: 15 min

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-notion-integration-infrastructure | 3 | 11 min | 3.7 min |
| 02-notion-connector | 1 | 4 min | 4.0 min |

**Recent Trend:**
- Last 5 plans: 4 min, 3 min, 4 min, 4 min
- Trend: fast (3.8 min avg)

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Pre-roadmap]: Tags replaced by pipelines — zero-step pipeline = label; migration is lazy (per-recording on access, not batch)
- [Pre-roadmap]: Notion auth via API key only (no OAuth) — internal integration token is sufficient for single-user desktop app
- [Pre-roadmap]: Prompt augmentation uses hard `Result<>` return — no silent fallthrough to non-JSON LLM output
- [Pre-roadmap]: Pin Notion API version to `2022-06-28` — 2025-09-03 introduced breaking multi-source DB change irrelevant to NBP
- [Pre-roadmap]: SortableJS loaded as local vendor file — native HTML5 DnD is unreliable in macOS WKWebView
- [Pre-roadmap]: Integration profiles stored as separate JSON files in `~/.nbp/config/integrations/` — never inside `settings.json`
- [Pre-roadmap]: Pipeline builder uses state-first full re-render pattern before any drag-and-drop code — prevents DOM/state desync
- [01-01]: Dev-mode credential bypass uses .dev-credentials.json (gitignored) at project root — avoids macOS Keychain permission dialogs during development
- [01-01]: get_integrations_dir() returns ~/.nbp/integrations/ (not ~/.nbp/config/integrations/) — matches existing ~/.nbp/ config root convention
- [01-01]: pub use slack::* in mod.rs preserves crate::integrations::get_slack_token backward-compatible path for connectors/slack.rs
- [Phase 01]: JSON-based PageOrDatabase extraction via serde_json::to_value avoids brittle enum pattern matching on external crate type
- [Phase 01]: Status database property returns empty select_options (inner struct field unverified); can be extended in Phase 2 once confirmed
- [01-03]: remove_notion_integration always attempts both deletions — prevents partial state from credential-only or profile-only orphans
- [01-03]: test_notion_integration formats error with {:?} — keeps token value out of user-visible error messages
- [01-03]: update_notion_people_mappings validates ALL user IDs before any write — rejects invalid sets atomically
- [02-01]: User struct constructed explicitly (not ..Default::default()) — notion-client User may not implement Default; safer to list all 4 known fields
- [02-01]: Empty people array skipped entirely — sending empty Vec<User> could clear existing Notion page assignees
- [02-01]: Profile-driven property iteration (not JSON keys) — prevents sending unknown property names that Notion API would reject with 400
- [02-01]: DateOrDateTime variant selected by T-character detection — handles both YYYY-MM-DD and YYYY-MM-DDTHH:MM:SSZ without additional parsing

### Pending Todos

None yet.

### Blockers/Concerns

- [Phase 1, cargo check]: cargo check not available in execution environment — all three plans' code verified structurally. Real compilation verification deferred to first cargo tauri dev run.
- [Phase 1, Status property]: `Status` DatabaseProperty options extraction deferred — inner struct field access not in research docs. Can be added in Phase 2/3 once cargo doc confirms field names.
- [Phase 3]: Prompt augmentation token budget estimate ("< 500 tokens") needs validation against real Notion databases with 10-20 properties before field relevance filter is deprioritized
- [Phase 7]: Exact shape of existing `metadata.json` files with `tags` field should be audited against real data before writing migration code

## Session Continuity

Last session: 2026-02-18
Stopped at: Completed 02-01-PLAN.md — connectors/notion.rs implemented, pipeline wiring (02-02) ready
Resume file: None

## Next Step

**Action:** execute Phase 2 Plan 02 (pipeline wiring)
**Command:** /gsd:execute-phase 2
**Context:** 02-01 complete. 02-02 must add ConnectorType::Notion to pipelines.rs and wire connectors::notion::execute() into pipeline_engine.rs match arm. Wave 2 (02-02) depends on Wave 1 (02-01) which is now done.
