# Feature Research

**Domain:** Audio-to-structured-data pipeline automation, desktop app (Tauri/Mac)
**Researched:** 2026-02-18
**Confidence:** MEDIUM-HIGH — core UX patterns verified via multiple sources; Notion API details HIGH confidence from official docs

---

## Context

This is a **subsequent milestone** (Pipelines v2). The app already ships:
- Audio capture (mic + system) and transcription
- Basic pipeline engine: LLM, Save, Webhook, Slack connectors
- Prompt templates (named, reusable)
- Developer-oriented pipeline builder (connector dropdown + raw field forms)
- Slack integration settings (token + workspace)
- Tags concept (separate from pipelines, `Vec<String>` in metadata)

Pipelines v2 is a full redesign of the pipeline mental model, builder UX, pre-assignment flow, integrations layer, and Notion connector.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist in a pipeline automation tool. Missing these = product feels incomplete or broken.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Pipeline assignment before recording** | Users expect zero-click post-recording work; pre-assignment is the most valuable moment in any recording workflow | MEDIUM | Chips in app bar; click chip = start recording with pipeline pre-assigned. Material Design filter chips are the standard pattern for this. |
| **Named pipeline presets (Meeting Notes, Action Items, Summary)** | Any voice-to-text tool in 2025 ships pre-built processing presets. Users do not expect to write prompts for common use cases. | LOW | Templates already exist in backend; need to surface as one-click step presets in builder. |
| **Processing + Delivery distinction in builder** | Zapier, Make, n8n all use a categorized step picker (not a flat connector dropdown). Users think in "what to do with text" vs "where to send it." | MEDIUM | Replaces current connector dropdown. Two categories: Processing (AI steps) and Delivery (send somewhere). |
| **Integrations settings page (connected / available)** | Every SaaS tool with connectors exposes a dedicated settings section showing what's connected. Users expect to see connection health without hunting. | MEDIUM | Three-state per integration: connected, disconnected, error. Must show "Test" and "Remove" actions inline. |
| **Integration health visibility (test / status)** | Users expect to know if a connection is broken before they run a pipeline. Broken integrations discovered mid-workflow are a primary UX frustration. | LOW | "Test" button per integration; shows success/error inline. Connection status badge (green/red). |
| **Default pipeline setting** | Power users and daily users expect a "set and forget" default so they never have to select a pipeline manually. | LOW | Single global setting in Settings > General. Applies to all new recordings unless overridden via chip. |
| **Pipeline run status per recording** | Users need to know whether automation completed. Post-processing status (Waiting / Running / Done / Failed) is expected after v1 ships. Already in engine. | LOW | Already in `PipelineState` struct. Needs surface in recording detail view. |
| **Multiple pipelines per recording** | Power users immediately want different "lenses" on the same recording (team notes → Slack, personal notes → file). | MEDIUM | Each pipeline runs independently, writes to own directory. Chips support multi-select. |
| **Pipeline step reordering** | Every step-based builder (n8n, Make, Zapier) supports drag-to-reorder. Users expect it. | MEDIUM | Drag-and-drop step list in builder. Critical for UX polish. |
| **Input chaining (each step uses previous output)** | Linear chain is the 90% case. Users expect this to be automatic, not manually configured. | LOW | Already in engine: `input = "previous_step_name"`. Surface as smart default in UI (auto-filled, not shown). |
| **Inline error display for failed pipeline steps** | Users need to know which step failed and why without reading logs. n8n, ADF, Jenkins BlueOcean all show step-level error details. | LOW | Show error inline in pipeline run status in recording detail view. |
| **Pipeline chip overflow handling** | When more than ~5 pipelines exist, chips need a "+N more" overflow. Material Design: 6-7 options max before dropdown. | LOW | Top N chips (configurable or auto), overflow to a small dropdown. |

### Differentiators (Competitive Advantage)

Features that set this product apart. Not baseline expected, but create strong loyalty when present.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Tags → Pipelines unification (0-step pipeline = label)** | Eliminates split mental model. User manages one concept instead of two. No other voice app does this cleanly. | MEDIUM | Pipeline with 0 steps is purely organizational. Tags in metadata.json are fully replaced. Migration path needed for existing tags. |
| **Schema-aware Notion connector with prompt augmentation** | User never writes format specs. App reads DB schema, injects field/type/value constraints into the AI prompt automatically. Structured JSON output maps to Notion pages. Only VOMO AI and similar cloud tools approach this — no desktop app does it offline/locally. | HIGH | Schema read via `GET /databases/{id}` (or new `data_source_id` in API v2025-09-03). People mapping step. Prompt augmentation in pipeline engine before LLM call. Requires Notion API key for v1 (OAuth adds complexity without multi-user value for a personal desktop app). |
| **Prompt augmentation visibility toggle** | Show the user what format instructions were auto-injected ("Advanced: view augmented prompt"). Builds trust; users learn what the system does. | LOW | Optional expand section in step config. Does not block the feature; add after core works. |
| **Save paths as named integrations** | Raw text path input in delivery step is fragile. Pre-configured named locations ("~/Documents/notes/" as "Notes folder") are reusable and safer. No voice app treats save paths as first-class integrations. | LOW | Same Settings > Integrations list. Save type integration: name + path. Referenced by name in builder. |
| **Schema re-sync button per integration** | Notion DB fields change over time. Most tools require reconnecting to refresh schema. An explicit "Sync schema" button per integration card is a power-user feature that prevents silent schema drift. | LOW | Calls `GET /databases/{id}` again, updates stored Integration Profile. Shows last-synced timestamp. |
| **UI health check (automated + interactive walkthrough)** | Zero other desktop apps ship a self-verification mode. Catches silent UI regressions before users hit them. Doubles as first-launch onboarding. | MEDIUM | Level 1: silent DOM audit on startup; badge in status bar. Level 2: interactive walkthrough on first launch and on demand. Lightweight — no test framework dependency. |
| **Mid-recording pipeline assignment** | Pipeline chips remain active during recording, allowing context-switching mid-session ("this turned into an HLTM conversation"). No voice recorder app does this. | LOW | Chips are always visible and clickable; pipeline assignment just writes to recording metadata, which pipeline engine reads when transcript is ready. |
| **Per-step provider/model override (hidden in Advanced)** | Power users want Claude for structured extraction, GPT-4o for summaries. Hiding it by default removes noise; surfacing it as Advanced respects power users. n8n and Zapier both surface this progressively. | LOW | Collapsed "Advanced" section in custom prompt step editor. Shows provider + model dropdowns only when expanded. |
| **Reusable prompt templates from inline prompts ("Save as template")** | Inline-first authoring with promote-to-reusable flow mirrors how users actually think. Other tools force templates upfront. | LOW | Checkbox "Save as reusable template" on custom prompt step. Saves to existing `prompt_templates.json`. |
| **Pipeline assembly preview (visual chain)** | Visual chain (`transcript → meeting_notes → action_items → Slack #hltm`) confirms what the user built is what they intended. n8n does this natively; Zapier does it linearly. | LOW | Read-only ASCII/visual chain below step list. Updates live as steps are added. |

### Anti-Features (Commonly Requested, Often Problematic)

Features to explicitly NOT build in this milestone.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Branching / conditional logic in pipelines** | Power users want "if transcript contains X, send to Y else Z" | Adds DAG complexity to an already-redesigned builder; 90% of use cases are linear chains; complicates step editor, serialization, and execution engine. Zapier shows that branching is a premium power feature that confuses most users. | Defer to v3. Linear chains satisfy 90% of use cases. |
| **Real-time sync of Notion schema (on every pipeline run)** | Seems correct — always have fresh schema | Schema API calls add latency to every run; schema rarely changes; API rate limits become a concern for frequent recordings; silent failures if Notion is unreachable. | Manual "Sync" button + last-synced timestamp. User controls when to refresh. |
| **OAuth for Notion in v1** | OAuth is better UX for multi-user apps | Desktop app is personal/single-workspace; OAuth adds redirect URI complexity (custom scheme or localhost server required), requires Notion security review for public integration, and adds token refresh management. API key (internal integration) is faster, simpler, and sufficient for a personal desktop tool. Notion docs confirm internal integrations are the recommended path for single-workspace tools. | API key (internal integration token) for v1. Revisit OAuth if app becomes multi-user or distributed. |
| **Workflow execution history / audit log screen** | Enterprise tools (Azure Data Factory, Harness) ship full run history dashboards | This app is personal; full history dashboards are over-engineering for a single-user desktop app. Pipeline run status per recording (already in engine) is sufficient. | Per-recording pipeline run status in detail view. Add a simple "Last run" timestamp to pipeline list. |
| **AI-suggested pipeline creation ("build me a pipeline for X")** | LLM-as-builder seems clever | Adds LLM API dependency to pipeline creation; output is hard to validate; users don't know what they got; n8n tried this and it's still experimental. Preset picker is faster and more reliable. | Preset step picker with named templates. Zero config for the 80% case. |
| **Shared/team pipelines (sync across devices)** | Multiple users, multiple devices | NBP is a local desktop app with local config; adding sync introduces cloud infrastructure, conflict resolution, and auth complexity that is out of scope for this milestone. | Local-only. Export/import JSON manually if sharing is needed. |
| **Telegram, Linear, Jira connectors in v2** | More integrations = more value | Partially-implemented connectors are worse than no connectors. Notion is the specific connector needed. Adding more connectors fragments the v2 scope and delays the schema-aware architecture being proven. | Ship Notion + existing Slack/Save/Webhook. List Telegram/Linear as "coming soon" in integrations UI. |

---

## Feature Dependencies

```
[Tags → Pipelines unification]
    └──requires──> [Pipeline CRUD with 0-step support]
                       └──requires──> [Pipeline chips in app bar]

[Pre-assignment chips in app bar]
    └──requires──> [Pipeline list loaded on app start]
    └──requires──> [Chip overflow handling]

[Preset step picker (Processing + Delivery categories)]
    └──requires──> [Integrations settings page]
                       └──because──> [Delivery picker shows ONLY connected integrations]

[Schema-aware Notion connector]
    └──requires──> [Notion integration setup wizard]
                       └──requires──> [Integration Profile storage]
    └──requires──> [Prompt augmentation in pipeline engine]
                       └──requires──> [Integration Profile storage]
    └──requires──> [Output parser in Notion connector]

[Save paths as integrations]
    └──requires──> [Integrations settings page]
    └──enhances──> [Delivery step picker]

[UI health check]
    └──requires──> [All other UI components rendered]
    └──enhances──> [First-launch onboarding]

[Per-step provider/model override (Advanced)]
    └──enhances──> [Custom prompt step editor]

[Prompt augmentation visibility toggle]
    └──enhances──> [Schema-aware Notion connector]

[Reusable prompt template from inline prompt]
    └──enhances──> [Custom prompt step]
    └──requires──> [Existing prompt_templates.json backend]
```

### Dependency Notes

- **Integration settings must exist before the delivery step picker**: The picker renders only connected integrations, so the integration storage layer must be complete first.
- **Integration Profile must exist before schema-aware Notion**: The profile stores the schema, people mappings, and defaults that the prompt augmentation reads at run time.
- **Chips require pipeline list**: Chips are rendered from the pipeline list. Pipeline CRUD is the foundation of the entire feature set.
- **Tags → Pipelines unification blocks chip display**: Until tags are merged into pipelines (including 0-step pipelines), the chip bar has no clean data source.

---

## MVP Definition

This is a subsequent milestone (Pipelines v2), not a greenfield MVP. "Launch with" means the v2 milestone ships these. "Add after" means future patch iterations.

### Launch With (Pipelines v2)

- [x] **Tags → Pipelines unification** — eliminates split concept; chips have no data source without this
- [x] **Pipeline chips in app bar (pre-assignment)** — the flagship v2 feature; zero post-recording work
- [x] **Default pipeline setting** — zero-friction daily use without chip selection
- [x] **Preset step picker (Processing + Delivery categories)** — replaces developer-oriented connector dropdown
- [x] **Built-in processing presets** (Meeting Notes, Action Items, Summary, Journal, Structure) — most users pick these; no prompt writing required
- [x] **Integrations settings page** (Connected / Available sections) — foundation for delivery step picker
- [x] **Save paths as integrations** — moves delivery config out of individual pipeline steps
- [x] **Integration health check buttons** (Test / Remove per connection) — prevents silent failures
- [x] **Notion setup wizard** (API key → pick DB → read schema → map people) — schema-aware connector
- [x] **Prompt augmentation in pipeline engine** — auto-injects format constraints from Notion schema before LLM call
- [x] **Notion output parser** — maps AI JSON to Notion API calls with people/select value mapping
- [x] **Chip overflow (top N + overflow popover)** — required once users have more than 5 pipelines
- [x] **Pipeline step reordering (drag)** — table stakes for any step-based builder
- [x] **Pipeline assembly preview (visual chain)** — confirms intent before saving

### Add After Validation (v2.x)

- [ ] **Schema re-sync button** — needed once Notion DBs change in practice; low complexity, add when first user reports schema drift
- [ ] **Prompt augmentation visibility toggle** — "show augmented prompt" advanced option; add once core schema-aware connector ships
- [ ] **Per-step provider/model override (Advanced)** — power user feature; add when users ask for Claude vs GPT-4o per step
- [ ] **UI health check Level 1 (silent DOM audit)** — add after all v2 UI components are finalized; requires complete component list
- [ ] **UI health check Level 2 (interactive walkthrough)** — add for first-launch onboarding once Level 1 is stable

### Future Consideration (v3+)

- [ ] **Branching/conditional logic** — requires DAG execution engine rewrite; defer until linear chains are proven insufficient
- [ ] **Telegram/Linear/Jira connectors** — follow schema-aware pattern once Notion proves the architecture
- [ ] **Shared/team pipelines** — requires cloud infrastructure; out of scope for local desktop app
- [ ] **Workflow execution history dashboard** — enterprise feature; per-recording status is sufficient for personal use

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Tags → Pipelines unification | HIGH | MEDIUM | P1 |
| Pipeline chips in app bar | HIGH | MEDIUM | P1 |
| Default pipeline setting | HIGH | LOW | P1 |
| Preset step picker | HIGH | MEDIUM | P1 |
| Integrations settings page | HIGH | MEDIUM | P1 |
| Save paths as integrations | HIGH | LOW | P1 |
| Integration health (Test/Remove) | HIGH | LOW | P1 |
| Notion setup wizard | HIGH | HIGH | P1 |
| Prompt augmentation in engine | HIGH | MEDIUM | P1 |
| Notion output parser | HIGH | MEDIUM | P1 |
| Built-in processing presets | HIGH | LOW | P1 |
| Chip overflow handling | MEDIUM | LOW | P1 |
| Pipeline step reorder (drag) | MEDIUM | MEDIUM | P1 |
| Pipeline assembly preview | MEDIUM | LOW | P1 |
| Schema re-sync button | MEDIUM | LOW | P2 |
| Prompt augmentation visibility | MEDIUM | LOW | P2 |
| Per-step provider/model override | MEDIUM | LOW | P2 |
| UI health check Level 1 | MEDIUM | MEDIUM | P2 |
| UI health check Level 2 | LOW | MEDIUM | P2 |
| Branching / conditional logic | HIGH | HIGH | P3 |
| Additional connectors (Telegram, Linear) | MEDIUM | HIGH | P3 |

**Priority key:**
- P1: Must have for Pipelines v2 launch
- P2: Should have, add in v2.x patch
- P3: Nice to have, future milestone

---

## Competitor Feature Analysis

Compared against: VOMO AI (closest feature-match voice pipeline app), n8n (pipeline builder UX reference), Zapier (step picker UX reference), Notion native integrations.

| Feature | VOMO AI | n8n | Zapier | NBP v2 (target) |
|---------|---------|-----|--------|-----------------|
| Pre-assignment chip bar | No — post-recording only | N/A | N/A | YES — primary v2 feature |
| Auto-run pipeline after transcription | Yes | Yes (trigger-based) | Yes (trigger-based) | YES — fully automatic |
| Preset processing steps | Yes (Summary, Action Items) | Yes (LLM nodes) | Yes (AI steps) | YES — Meeting Notes, Action Items, Summary, Journal, Structure |
| Schema-aware structured delivery | No | Partial (manual JSON mapping) | No | YES — Notion schema read + prompt augmentation |
| Save paths as named integrations | No (raw path) | N/A | N/A | YES |
| Integration health check | No | No | No | YES (Test button per connection) |
| UI self-health check | No | No | No | YES (v2.x) |
| Multi-pipeline per recording | No | N/A | N/A | YES |
| Local/offline processing | No (cloud) | Yes (self-host) | No (cloud) | YES — fully local |
| Notion integration | No | Yes (manual mapping) | Yes (manual mapping) | YES — schema-aware, automatic |

**Key competitive differentiator:** NBP is the only local desktop app that combines pre-assignment chips, automatic post-recording pipeline execution, and schema-aware structured delivery (Notion) with zero user-written format specs. This combination does not exist in any current tool.

---

## Notion API: Key Technical Facts

**Confidence: HIGH** — verified against official Notion API documentation.

- **Schema reading:** `GET /v1/databases/{database_id}` returns `properties` object with all column types (title, rich_text, number, select, multi_select, date, people, checkbox, url, email, phone_number, formula, relation, etc.)
- **API versioning:** Notion released API version `2025-09-03` (multi-source databases). For v1 NBP integration, use `2022-06-28` API version (single data source per database). Migration to `2025-09-03` can be done later. Breaking change only affects multi-source databases.
- **Internal integration (API key):** Single workspace, no OAuth flow, no security review required, no redirect URI complexity. Correct choice for a personal desktop app.
- **People property:** Returns array of Notion user objects with IDs and names. People mapping in integration profile maps display names (e.g., "СК") to Notion user IDs.
- **Select options:** Select and multi-select properties return the full list of valid option names and IDs in the schema. Prompt augmentation can inject valid option values directly.
- **Schema size limit:** Notion recommends max 50KB per schema. Not a practical concern for typical databases.
- **No webhooks in current API:** Notion does not provide native webhooks (as of Feb 2026). Schema refresh must be polling/manual. Manual "Sync" button is the correct UX.

---

## Sources

- [Notion API - Working with Databases](https://developers.notion.com/docs/working-with-databases) — schema reading, property types (HIGH confidence)
- [Notion API - Authorization](https://developers.notion.com/docs/authorization) — OAuth vs internal integration comparison (HIGH confidence)
- [Notion API Upgrade Guide 2025-09-03](https://developers.notion.com/guides/get-started/upgrade-guide-2025-09-03) — API version changes (HIGH confidence)
- [Material Design 3 - Chips](https://m3.material.io/components/chips/guidelines) — chip UX patterns, 6-7 max before dropdown (HIGH confidence)
- [n8n vs Zapier Comparison 2026](https://hatchworks.com/blog/ai-agents/n8n-vs-zapier/) — step picker categories, linear vs branching patterns (MEDIUM confidence)
- [n8n Features](https://n8n.io/features/) — node-based step picker, category patterns (MEDIUM confidence)
- [Voice Memo App Features 2025 - MeowTXT](https://www.meowtxt.com/blog/voice-memo-transcription-app) — post-processing feature landscape (MEDIUM confidence)
- [VOMO AI product page](https://moge.ai/product/vomo-ai) — competitive feature comparison (MEDIUM confidence, single source)
- [Pipeline Execution UX - Jenkins Blue Ocean](https://www.jenkins.io/doc/book/blueocean/pipeline-run-details/) — step-level error display patterns (MEDIUM confidence)
- [SaaS Integration Best Practices 2026 - Skyvia](https://blog.skyvia.com/saas-integration-best-practices/) — integration reliability patterns (MEDIUM confidence)

---

*Feature research for: NBP Pipelines v2 — audio-to-structured-data pipeline automation desktop app*
*Researched: 2026-02-18*
