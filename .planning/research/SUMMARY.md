# Project Research Summary

**Project:** NBP Pipelines v2
**Domain:** Audio-to-structured-data pipeline automation, desktop app (Tauri/macOS)
**Researched:** 2026-02-18
**Confidence:** MEDIUM-HIGH

## Executive Summary

NBP Pipelines v2 is a milestone redesign of an existing local desktop pipeline engine built in Tauri (Rust + Vanilla JS). The app already ships audio capture, transcription, and a basic pipeline engine with LLM, Save, Webhook, and Slack connectors. V2 introduces a new mental model: unified pipelines-as-labels (replacing a split tags/pipelines concept), a pre-assignment chip bar for zero-friction recording workflow, a categorized step picker replacing the raw connector dropdown, and a schema-aware Notion connector that auto-injects structured format instructions into AI prompts based on the database schema. The defining competitive differentiator is the combination of pre-assignment chips, automatic post-recording execution, and schema-aware delivery — no current tool offers all three locally and offline.

The recommended approach is to build in strict backend-before-frontend order. The Notion integration profile infrastructure (Rust, schema snapshot to disk) must exist before the delivery step picker can render correctly, and prompt augmentation must be correct before the Notion connector is useful. The key stack additions are minimal: `notion-client` (1.0.11) for schema reading and page creation, `jsonschema` (0.29) for AI output validation, and SortableJS (local vendor file) for drag-and-drop step reordering. Everything else extends the existing stack. Macros and IPC patterns already in place require no new tooling.

The primary risks are Notion API-specific: integration sharing is a mandatory out-of-band Notion UI step that no API call can automate, the 2025-09-03 API version introduced a breaking change for multi-source databases (pin to `2022-06-28`), and the LLM will produce free-form prose rather than JSON if prompt augmentation silently fails. All three can be fully mitigated with explicit wizard instructions, a pinned API version header constant, and making `build_augmented_prompt()` return `Result<>` (hard failure) rather than a silent fallthrough. The vanilla JS pipeline builder also requires a strict state-first/full-re-render pattern from the outset to prevent DOM/state desync bugs.

---

## Key Findings

### Recommended Stack

The project adds three new dependencies to the locked existing stack. For Rust: `notion-client 1.0.11` (most actively maintained Notion crate, uses existing `reqwest`/`tokio`/`serde_json`), `jsonschema 0.29` (AI output validation against integration profile schema), and optionally `schemars 0.8` (schema generation from Rust structs). For the frontend: SortableJS 1.15.x loaded as a local vendor file — the native HTML5 DnD API is unreliable in macOS WKWebView and the alternative (`html5sortable`) is maintenance-risk. The UI health check is an in-app zero-dependency module, not a test framework — Tauri WebDriver has no macOS support. The existing `security-framework` crate covers Notion API key Keychain storage using the same pattern already used for Slack.

**Core new technologies:**
- `notion-client 1.0.11`: Notion API client — pre-built Rust types for all Notion property types; uses existing reqwest, no new HTTP runtime
- `jsonschema 0.29`: AI output validation — validates `serde_json::Value` against integration profile schema before Notion API calls; prevents confusing 400 errors from malformed LLM output
- `schemars 0.8` (optional): Schema generation — derive macro for `NotionIntegrationProfile` structs; useful if exposing schema to prompt augmentation layer
- `SortableJS 1.15.x` (vendor file): Drag-and-drop step reordering — reliable in WKWebView, one-line initialization; native HTML5 DnD is broken in Tauri on macOS
- `ui_health_check.js` (custom, no deps): Runtime DOM audit — uses `document.querySelector` and `invoke()` directly; Tauri WebDriver is macOS-unsupported

**Version to pin:** Notion API `2022-06-28` for v1. The `2025-09-03` version introduced multi-source database support with a breaking `data_source_id` change — irrelevant for NBP's single-workspace use case.

### Expected Features

Research confirms 14 features for the v2 milestone launch and 5 features deferred to v2.x, with a clear dependency graph that drives phase ordering.

**Must have (table stakes — v2 launch):**
- Tags-to-pipelines unification — eliminates split mental model; chips have no clean data source without this
- Pipeline chips in app bar (pre-assignment) — the flagship v2 feature; click chip = start recording with pipeline pre-assigned
- Default pipeline setting — zero-friction for daily users who never need to select manually
- Preset step picker (Processing + Delivery categories) — replaces developer-oriented connector dropdown; Zapier/n8n UX pattern
- Built-in processing presets (Meeting Notes, Action Items, Summary, Journal, Structure) — most users pick these; no prompt writing required
- Integrations settings page (Connected / Available sections) — prerequisite for delivery picker (which shows only connected integrations)
- Save paths as named integrations — moves fragile raw-path delivery config into reusable integration records
- Integration health check (Test/Remove per connection) — prevents silent failures discovered mid-workflow
- Notion setup wizard (API key → pick DB → read schema → map people) — enables schema-aware delivery
- Prompt augmentation in pipeline engine — auto-injects format constraints from integration profile before LLM call; invisible to user
- Notion output parser — maps AI JSON to Notion API calls with people/select value normalization
- Chip overflow handling (top 5 + overflow popover) — required from first render, not retrofit
- Pipeline step reordering (drag-and-drop) — table stakes for any step-based builder
- Pipeline assembly preview (visual chain) — confirms user intent before saving

**Should have (competitive differentiators — v2.x patches):**
- Schema re-sync button — needed when Notion DB fields change; low complexity
- Prompt augmentation visibility toggle — "show augmented prompt" for power users
- Per-step provider/model override (collapsed Advanced) — Claude for extraction, GPT-4o for summaries
- UI health check Level 1 (silent DOM audit on startup) — unique to NBP; no other desktop app ships this
- UI health check Level 2 (interactive first-launch walkthrough)

**Defer (v3+):**
- Branching/conditional logic — DAG execution engine rewrite; 90% of cases are linear chains
- Additional connectors (Telegram, Linear, Jira) — follow Notion schema-aware pattern once proven
- Shared/team pipelines — requires cloud infrastructure; out of scope for local desktop app
- OAuth for Notion — internal API key is correct for single-user personal desktop; OAuth adds redirect URI and security review complexity without value

### Architecture Approach

The architecture separates two distinct Notion modules: `integrations/notion.rs` (owns API auth, schema fetching, profile storage — setup-time operations) and `connectors/notion.rs` (owns structured JSON parsing, people alias resolution, Notion page creation — runtime operations). These have different lifecycles and failure modes and must never be merged. Integration profiles are stored as per-integration JSON files at `~/.nbp/config/integrations/{type}-{id}.json`, never inside `AppSettings`/`settings.json` — schema profiles can be large and the settings mutex would become a bottleneck. The pipeline builder must extract from `main.js` early: `pipeline-builder.js` and `integrations-settings.js` as separate JS modules before they grow unmanageable.

**Major components:**

1. **Integration Profile Store** (`integrations/notion.rs` + `~/.nbp/config/integrations/`) — schema snapshot written at setup time, read at execution time; manual sync only; never fetched per-run
2. **Prompt Augmentation Layer** (`pipeline_engine.rs`) — look-ahead: if next step is a structured connector, load profile and append JSON format spec to LLM prompt; returns `Result<>` (hard fail if profile missing)
3. **Notion Connector** (`connectors/notion.rs`) — strips frontmatter, extracts JSON array (tolerates markdown code fences), normalizes select/people values, creates Notion pages via REST
4. **Pipeline Chips** (`main.js`) — rendered from `list_pipelines`; chip click sets `pendingPipelineId` and calls `start_recording`; overflow at 5 chips with dropdown; never navigate on click
5. **Pipeline Builder** (`pipeline-builder.js`) — strict state-first pattern: every action updates state object, then calls `renderSteps()` full re-render; drag-and-drop via SortableJS
6. **Integrations Settings** (`integrations-settings.js`) — multi-step wizard (API key → DB selection → schema sync → people mapping); modal overlay, not full-page navigation
7. **UI Health Check** (`ui-health-check.js`) — deferred via `requestIdleCallback` after app startup; DOM audit returns `{ passed, failed, issues[] }`; badge in status bar

**Key patterns:**
- Integration Profile snapshot (setup-time schema → disk, read at runtime) — avoids per-run API calls and rate limits
- Prompt Augmentation via look-ahead (N+1 step inspection) — schema injected by engine, never by user
- State-first re-render for pipeline builder — prevents DOM/state desync in vanilla JS
- Separate connector vs. integration module boundary — setup-time writes vs. runtime reads

### Critical Pitfalls

1. **Notion API version mismatch (2025-09-03 breaking change)** — Pin `Notion-Version: 2022-06-28` header as a single named constant in `integrations/notion.rs`; never use `2025-09-03` for v1. Multi-source databases are enterprise-only and not an NBP use case. Record the API version used in each integration profile JSON for future migration.

2. **Integration not shared with the Notion database** — The Notion security model requires the user to explicitly connect the integration to the database via the Notion UI (not the API). This is invisible to code: token validates fine, but every database call returns 404. The setup wizard must include this as an explicit, illustrated step ("open your database → ••• → Connections → Add integration"). The 404 error handler must show this instruction, not a generic error.

3. **Prompt augmentation silent fallthrough produces non-JSON LLM output** — If the integration profile is missing (deleted, wrong ID), the look-ahead falls through and the LLM produces free-form prose instead of JSON. The connector fails with a cryptic parse error after wasting an expensive AI API call. `build_augmented_prompt()` must return `Result<String, String>` and hard-fail before the LLM call: "Notion integration X not configured. Sync schema in Settings."

4. **Vanilla JS state/DOM desync in pipeline builder** — Without a framework's virtual DOM, direct DOM manipulation and state mutations diverge. Delete removes the wrong step, drag-drop shows correct visual order but saves wrong order. Prevention: establish the state-first full-re-render pattern (`renderSteps()` clears and rebuilds from `pipelineState.steps`) before any drag-and-drop code.

5. **Notion `select` value case mismatch / `rich_text` wrong key** — LLM output uses "high" when Notion requires "High"; LLM uses `"text"` when Notion API requires `"rich_text"`. Prevention: `resolve_select_value()` does case-insensitive match with fallback to profile default; property formatter uses a hardcoded type-to-API-key mapping table, never LLM-generated keys.

---

## Implications for Roadmap

Based on the dependency graph in FEATURES.md and the build order from ARCHITECTURE.md, the following 8-phase structure is recommended. The architecture research explicitly derives this order; it is not speculative.

### Phase 1: Notion Integration Infrastructure (Rust backend)
**Rationale:** Everything else depends on it. The integration profile storage must exist before the delivery picker, builder, or connector can function. Dev-mode Keychain bypass must be in place before any new credential integration begins.
**Delivers:** `integrations/notion.rs` with full API client; `NotionIntegrationProfile` struct and per-integration JSON storage; new Tauri commands (`add_notion_integration`, `list_notion_databases`, `sync_notion_schema`, `update_notion_people_mappings`, `test_notion_integration`, `remove_notion_integration`); `config.rs` integration directory helpers; dev-mode `#[cfg(debug_assertions)]` Keychain bypass.
**Addresses features:** Notion setup wizard (backend), integration health check (backend)
**Avoids pitfalls:** API version mismatch (pin `2022-06-28` here), profile-in-settings.json anti-pattern (separate files from day one), dev-mode Keychain prompts (bypass implemented before new credentials)

### Phase 2: Notion Connector (Rust backend)
**Rationale:** Depends on Phase 1 profile storage. The connector reads profiles written in Phase 1. JSON parsing, people alias resolution, and select value normalization are isolated here.
**Delivers:** `connectors/notion.rs` with `extract_json_array()`, `resolve_people_aliases()`, `resolve_select_value()`; `jsonschema` validation of AI output; `pipelines.rs` extended with `ConnectorType::Notion` and `integration_id` field; full Notion page creation via `notion-client` crate.
**Addresses features:** Notion output parser, AI output validation
**Avoids pitfalls:** `select` case mismatch (case-insensitive `resolve_select_value` with default fallback), `rich_text` wrong key (property type mapping table), AI output not validated (jsonschema validation before API call)

### Phase 3: Prompt Augmentation (Rust backend)
**Rationale:** Depends on Phases 1 and 2. The augmentation reads integration profiles (Phase 1) and its output feeds the Notion connector (Phase 2). This must be correct and tested before any end-to-end pipeline run is meaningful.
**Delivers:** `build_augmented_prompt()` in `pipeline_engine.rs` with look-ahead N+1 inspection; `build_notion_format_spec()` generating field/type/option constraints from profile; hard `Result<>` return on missing profile (not silent fallthrough); field relevance filtering (to prevent context window overflow for large schemas).
**Addresses features:** Prompt augmentation in pipeline engine, prompt augmentation visibility toggle (foundation)
**Avoids pitfalls:** Silent fallthrough to non-JSON LLM output (returns `Result<>` not `String`), context window overflow from full property/people list (field relevance filter)

### Phase 4: Integration Settings UI (Frontend)
**Rationale:** Depends on Phase 1 Tauri commands. The wizard calls `invoke("add_notion_integration")` etc., which must exist first. The delivery step picker (Phase 5) requires integration data to render — so this must come before the builder redesign.
**Delivers:** `integrations-settings.js` with Notion setup wizard (API key → DB picker → schema display → people mapping); explicit "share with database" instruction as mandatory wizard step; Test/Remove buttons per integration; schema display with last-synced timestamp; error messages on 404 showing Notion sharing instructions.
**Addresses features:** Integrations settings page, integration health check (UI), Notion setup wizard (UI), save paths as integrations (settings UI)
**Avoids pitfalls:** Integration not shared with DB (wizard includes explicit illustrated step and 404 error handler shows instructions), people property not queryable by name (full user list fetched and mapped client-side), prompt augmentation context overflow (field relevance toggle in wizard)

### Phase 5: Pipeline Builder Redesign (Frontend)
**Rationale:** Depends on Phase 4 — the delivery step picker renders only connected integrations. Depends on Phase 2's `ConnectorType::Notion`. Must establish state-first pattern before any drag-and-drop code is written.
**Delivers:** `pipeline-builder.js` extracted from `main.js`; preset step picker (Processing and Delivery categories); delivery picker filtered to connected integrations only; SortableJS drag-and-drop step reordering; state-first re-render pattern (`pipelineState.steps` as authoritative source); pipeline assembly preview (visual chain); MCP connector filtered from picker until implemented; auto-generated step names from presets.
**Addresses features:** Preset step picker, pipeline step reordering, pipeline assembly preview, per-step provider/model override (Advanced collapsed section, foundation), reusable prompt template from inline prompt
**Avoids pitfalls:** State/DOM desync (state-first pattern established before drag-and-drop), missing `preventDefault()` (manual test of drop event before building full reordering), drag ghost image (custom setDragImage)

### Phase 6: Pipeline Chips and Pre-Assignment (Frontend)
**Rationale:** Depends on Phase 5 pipeline model being stable (chips render from `list_pipelines`). The chip is the primary user-facing entry point — it must be built after the pipeline model is correct and the builder works.
**Delivers:** Pipeline chip rendering in `main.js`; `startRecordingWithPipeline(name)` flow (chip click = immediate record, not navigate); 5-chip cap with overflow popover from first render; mid-recording pipeline assignment (chips remain active); default pipeline setting in Settings > General; pipeline run status surface in recording detail view.
**Addresses features:** Pipeline chips in app bar, default pipeline setting, multiple pipelines per recording, chip overflow handling, mid-recording pipeline assignment, pipeline run status
**Avoids pitfalls:** Chip click navigates to builder (chip click = record, always), chip overflow breaks layout (5-chip cap built in from start, not retrofit), last-used chip persisted across restarts (localStorage or settings)

### Phase 7: Tags to Pipeline Labels Migration (Rust + data)
**Rationale:** Deferred until the pipeline model (Phases 5-6) is stable. Migration is destructive — it modifies `metadata.json` for existing recordings. Running it before the pipeline model is proven risks corrupting user data. Lazy per-recording migration eliminates batch corruption risk.
**Delivers:** `storage::migrate_tags_to_pipeline_labels()` with lazy per-recording migration (on access, not all-at-once); `RecordingMetadata` `pipelines` field addition; `tags` field retained for two versions before removal; 0-step pipeline support for organizational labels.
**Addresses features:** Tags to pipelines unification
**Avoids pitfalls:** Tags migration data corruption (lazy per-recording migration; backup `tags` field for 2 versions; never batch-migrate all recordings at startup)

### Phase 8: UI Health Check (Frontend)
**Rationale:** Last, because it audits all other UI components. Cannot be written until all v2 UI components exist. Runs deferred via `requestIdleCallback` — never blocks startup.
**Delivers:** `ui-health-check.js` with `runAudit()` returning `{ passed, failed, issues[] }`; status bar badge (clickable, links to diagnostic report); deferred execution via `requestIdleCallback` after all startup `invoke()` calls complete; Level 1 silent audit on startup.
**Addresses features:** UI health check Level 1 (v2 launch), UI health check Level 2 interactive walkthrough (v2.x)
**Avoids pitfalls:** Health check runs synchronously during startup (`requestIdleCallback` deferred), badge shows green before async data loads (health check deferred until after all startup `invoke()` calls complete)

### Phase Ordering Rationale

The order is driven by three constraints from research:

1. **Backend infrastructure before frontend:** The delivery step picker cannot render connected integrations until integration storage exists (Phases 1 must precede Phase 4 and 5). Prompt augmentation must be correct before any end-to-end Notion pipeline is meaningful.

2. **Data model before UI that depends on it:** Tags-to-pipelines unification (Phase 7) is deferred deliberately — the pipeline model must be stable before a destructive migration modifies existing recording metadata.

3. **State-first pattern must be established before drag-and-drop:** Pitfall research shows that introducing drag-and-drop into a vanilla JS builder without a state-first pattern guarantees DOM/state desync bugs. Phase 5 must establish the pattern before adding reordering.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 1:** Notion API client crate (`notion-client 1.0.11`) — API surface not fully verified; actual endpoint method signatures should be confirmed before writing integration module. Run `cargo doc` after adding dependency to verify method names.
- **Phase 3:** Prompt augmentation field filtering — the optimal heuristic for which fields to include in the injected format spec (to avoid context window overflow) is not fully specified. Needs validation with a real Notion schema + long transcript before the feature is marked complete.
- **Phase 7:** Tags migration — exact data shape of existing `metadata.json` files with `tags` field needs audit against real user data before migration code is written. Check `~/.nbp/` to confirm schema.

Phases with well-documented patterns (can skip research-phase):
- **Phase 2:** Notion connector JSON parsing — patterns are fully specified in ARCHITECTURE.md with working code examples; `strip_frontmatter` already exists in codebase.
- **Phase 5:** SortableJS drag-and-drop — one-line initialization, well-documented; pitfall (`preventDefault`) is documented with exact fix.
- **Phase 6:** Pipeline chips — architecture and JS code examples fully specified in ARCHITECTURE.md.
- **Phase 8:** UI health check — pattern is zero-dependency DOM APIs; fully specified in FEATURES.md and PITFALLS.md.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM | `notion-client` API surface not fully verified at method level; SortableJS WebView behavior not independently tested on all macOS versions; core recommendations solid |
| Features | MEDIUM-HIGH | Core UX patterns (chips, categorized picker) verified against Material Design 3 and n8n/Zapier; Notion API facts HIGH confidence from official docs; competitive analysis MEDIUM (limited VOMO AI sources) |
| Architecture | HIGH | Existing codebase directly analyzed; Notion API endpoints verified against official reference; integration profile pattern is directly derived from existing Slack pattern; build order derived from dependency analysis not speculation |
| Pitfalls | HIGH | Notion API pitfalls from official docs + GitHub issue tracker; Tauri Keychain dev mode from official GitHub issue; DnD pitfalls from multiple sources; AI structured output pitfalls from OWASP 2025 |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- **`notion-client` method signatures:** The crate's exact method names for `list_databases`, `get_database`, `create_page`, and `get_users` should be confirmed via `cargo doc` or crate source before writing `integrations/notion.rs`. The crate is actively maintained but specific API surface was not fully verified in research.
- **Notion API `2022-06-28` deprecation timeline:** No deprecation timeline was announced as of Feb 2026, but this should be re-checked before v2 ships. The upgrade to `2025-09-03` may become mandatory — `notion-client` crate version compatibility with the new API is unknown.
- **SortableJS in WKWebView on macOS Sonoma / Sequoia:** SortableJS WebView reliability is documented in general but not specifically tested against Tauri's WKWebView on recent macOS versions. Build a minimal proof-of-concept drag-and-drop before committing to Phase 5 reordering.
- **Prompt augmentation token budget:** The "< 500 tokens" format spec estimate needs validation against real Notion databases with 10-20 properties and 10-20 team members before the field relevance toggle feature is deprioritized.

---

## Sources

### Primary (HIGH confidence)
- Notion REST API reference (property-object, authorization, users, pages): developers.notion.com — schema types, API versioning, authentication model, user listing behavior
- Notion API Upgrade Guide 2025-09-03: developers.notion.com — breaking change confirmed, single-source vs multi-source database model
- Tauri WebDriver docs: v2.tauri.app — macOS WebDriver not supported, confirmed
- Tauri GitHub issue #8662: Keychain dev mode prompts — confirmed behavior, `#[cfg(debug_assertions)]` workaround
- Existing NBP codebase: `/workspace/src-tauri/src/` — pipeline_engine.rs, connectors/, integrations.rs, config.rs — direct analysis

### Secondary (MEDIUM confidence)
- `notion-client` 1.0.11 on crates.io and GitHub (takassh/notion-client) — active maintenance confirmed, specific method signatures not fully verified
- Material Design 3 Chip guidelines — chip overflow patterns, 6-7 max before dropdown
- SortableJS GitHub (SortableJS/Sortable) — no-framework usage confirmed; WKWebView behavior not independently verified
- n8n and Zapier feature analysis — categorized step picker patterns, linear vs branching use cases
- OWASP LLM01:2025 Prompt Injection — structured output validation requirements

### Tertiary (LOW confidence)
- VOMO AI product page — competitive feature comparison (single source, product marketing material)
- LLM structured output reliability guides — retry strategy patterns; general principles apply but NBP-specific validation approach is straightforward enough to implement without following any single guide

---

*Research completed: 2026-02-18*
*Ready for roadmap: yes*
