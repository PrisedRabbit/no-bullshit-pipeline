# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-18)

**Core value:** Zero post-recording work — select pipeline, record, stop, everything happens automatically.
**Current focus:** Phase 6 (Pre-Assignment UX and Execution) — executing

## Current Position

Phase: 6 of 8 (Pre-Assignment UX and Execution) — in progress
Plan: 2 of 3 in current phase — 06-01 and 06-02 complete, 06-03 remains
Status: 06-02 complete — default pipeline setting, last-used persistence, detail view pipeline assignment
Last activity: 2026-02-19 — 06-02 default/last-used pipeline plan executed

Progress: [█████░░░░░] 58% (15/26 plans)

## Performance Metrics

**Velocity:**
- Total plans completed: 7
- Average duration: 3.4 min
- Total execution time: 25 min

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-notion-integration-infrastructure | 3 | 11 min | 3.7 min |
| 02-notion-connector | 2 | 6 min | 3.0 min |
| 03-prompt-augmentation | 2 | 8 min | 4.0 min |

**Recent Trend:**
- Last 5 plans: 4 min, 4 min, 2 min, 5 min, 3 min
- Trend: fast (3.4 min avg)

*Updated after each plan completion*
| Phase 02-notion-connector P02 | 2 | 2 tasks | 2 files |
| Phase 03-prompt-augmentation P01 | 5 | 2 tasks | 2 files |
| Phase 03-prompt-augmentation P02 | 3 | 2 tasks | 1 file |
| Phase 04-integrations-settings-ui P01 | 8 | 1 tasks | 6 files |
| Phase 04-integrations-settings-ui P02 | 2 | 2 tasks | 3 files |
| Phase 04 P03 | 2 | 2 tasks | 5 files |
| Phase 05-pipeline-builder-redesign P01 | 4 | 2 tasks | 5 files |
| Phase 05-pipeline-builder-redesign P02 | 2 | 2 tasks | 3 files |
| Phase 05-pipeline-builder-redesign P03 | 5 | 2 tasks | 5 files |
| Phase 06-pre-assignment-ux-and-execution P01 | 2 | 2 tasks | 4 files |
| Phase 06-pre-assignment-ux-and-execution P02 | 2 | 2 tasks | 4 files |

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
- [Phase 02-02]: integration_id only required in Notion step config — database_id comes from stored integration profile loaded at execute() time
- [Phase 02-02]: Notion match arm placed between Slack and Mcp in pipeline_engine.rs — preserves ordering (implemented before stub)
- [03-01]: augmented_prompt: Option<&str> as 7th param to llm::execute() — None path is identical to pre-Phase-3 behavior, no impact on non-Notion pipelines
- [03-01]: build_augmented_prompt() hard-fails when profile missing or properties/database_id empty — prevents expensive LLM API call with guaranteed non-JSON output
- [03-01]: WRITABLE_TYPES excludes formula/relation/rollup/computed fields — only LLM-settable fields in format spec
- [03-01]: MAX_OPTIONS_IN_SPEC=12 caps select options to prevent token budget overflow; overflow count shown in spec
- [03-02]: validate_llm_output_for_notion() uses fully-qualified std::collections::HashSet path — no new use statement needed
- [03-02]: WRITABLE_TYPES in connectors/notion.rs is separate from pipeline_engine.rs — intentional per-module independence
- [03-02]: Empty array check fires before per-element checks — most common LLM failure mode gets a distinct message
- [Phase 04-01]: integrations-settings.js loads after main.js — escapeHtml and invoke available as globals without re-declaration
- [Phase 04-01]: MutationObserver on integrations tab class fires loadAllIntegrations only when tab becomes active (lazy loading)
- [Phase 04-01]: var declarations for notionProfiles and _slackIntegrations put them on window — accessible from main.js for pipeline step editor
- [Phase 04-integrations-settings-ui]: Cancel wired once in openNotionWizard() via node clone — not re-attached on each step render
- [Phase 04-integrations-settings-ui]: replaceNextBtn() clones Next button node on each step to prevent stacked async event listeners
- [Phase 04-integrations-settings-ui]: Finish skips update_notion_people_mappings call entirely when cleanMappings array is empty
- [Phase 04]: save_path.rs follows notion.rs I/O pattern — same directory, 0o600 permissions, idempotent remove via ErrorKind::NotFound
- [Phase 04]: Save connector falls back to free-text path input when no save path integrations exist — preserves backward compatibility with existing pipelines
- [Phase 04]: notionProfiles and savePathIntegrations accessed in main.js via typeof guard — safe before integrations tab has loaded
- [Phase 05-01]: var allPipelineDefs in pipeline-builder.js makes pipeline count accessible to main.js updateSidebarCounts() at runtime
- [Phase 05-01]: SortableJS destroyed and re-created in renderPipelineSteps() because innerHTML replacement creates new DOM nodes
- [Phase 05-01]: Script load order: sortable.min.js -> main.js -> integrations-settings.js -> pipeline-builder.js for correct global availability
- [Phase 05-02]: Picker inserted inline after addPipelineStepBtn — simpler than positioned dropdown, scrolls with content
- [Phase 05-02]: PROCESSING_PRESETS[n].step=null signals Custom Prompt deferred to Plan 05-03
- [Phase 05-02]: Builtin merge uses contains_key check — existing installs get new templates without overwriting user-modified built-ins
- [Phase 05-03]: prompt_template and prompt_inline are both Option<String> in LlmConfig — either accepted, validation requires at least one
- [Phase 05-03]: build_augmented_prompt() reads input file inside each branch independently — self-contained, no stale references
- [Phase 05-03]: deliveryConnectors hardcoded in renderPipelinePreview() — mirrors Rust ConnectorType enum values
- [Phase 05-03]: <details> Advanced section in step editor — querySelectorAll('[data-field]') finds elements inside details, no handler changes needed
- [Phase 06-pre-assignment-ux-and-execution]: Pipeline chip bar uses position:relative on container and position:absolute on overflow popover — chips stay in app bar flow without affecting layout
- [Phase 06-pre-assignment-ux-and-execution]: renderPipelineChips() replaces entire innerHTML atomically then re-attaches event listeners — consistent with pipeline-builder state-first pattern, prevents DOM/state desync
- [Phase 06-pre-assignment-ux-and-execution]: stoppedPipeline local variable captured from currentAssignedPipeline at start of stopRecording() try block — 06-03 can read this for auto-execute without race condition
- [06-02]: PipelineState.name field accessed in JS (not pipeline_name) — Rust struct uses .name for the pipeline name field
- [06-02]: detailPipelineHandler module-level variable enables removeEventListener before re-attaching on each showDetailView call — prevents stacked async change listeners
- [06-02]: last_used_pipeline saved immediately after assign_pipeline succeeds in startRecordingWithPipeline() — persists before any UI updates
- [06-02]: populateDefaultPipelineSelect() called via typeof guard from loadPipelineDefs() — same cross-module pattern as renderPipelineChips()

### Pending Todos

None yet.

### Blockers/Concerns

- [Phase 1, cargo check]: cargo check not available in execution environment — all three plans' code verified structurally. Real compilation verification deferred to first cargo tauri dev run.
- [Phase 1, Status property]: `Status` DatabaseProperty options extraction deferred — inner struct field access not in research docs. Can be added in Phase 2/3 once cargo doc confirms field names.
- [Phase 3]: Prompt augmentation token budget estimate ("< 500 tokens") needs validation against real Notion databases with 10-20 properties before field relevance filter is deprioritized
- [Phase 7]: Exact shape of existing `metadata.json` files with `tags` field should be audited against real data before writing migration code

## Session Continuity

Last session: 2026-02-19
Stopped at: Completed 06-pre-assignment-ux-and-execution-06-02-PLAN.md
Resume file: None

## Next Step

**Action:** execute Phase 6 Plan 03 (auto-execute pipeline on recording stop)
**Command:** /gsd:execute-phase 6
**Context:** 06-01 and 06-02 complete. Pipeline chip bar live, last-used pipeline highlighting persists across launches, detail view pipeline assignment dropdown works. 06-03 adds auto-execute: when a recording stops with an assigned pipeline (stoppedPipeline local var), automatically trigger pipeline execution.
