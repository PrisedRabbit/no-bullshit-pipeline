# Architecture Research

**Domain:** Schema-aware pipeline connectors, integration profiles, and prompt augmentation in a Tauri desktop app
**Researched:** 2026-02-18
**Confidence:** HIGH (existing codebase analyzed directly; Notion API verified against official docs)

---

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        FRONTEND (Vanilla JS)                         │
├──────────────────────┬──────────────────────┬───────────────────────┤
│   App Bar            │   Pipeline Builder   │   Settings            │
│   ┌──────────────┐   │   ┌──────────────┐   │   ┌───────────────┐   │
│   │ Pipeline     │   │   │ Preset       │   │   │ Integrations  │   │
│   │ Chips        │   │   │ Picker       │   │   │ Manager       │   │
│   │ + Record Btn │   │   │ Step List    │   │   │ (Notion/Slack)│   │
│   └──────────────┘   │   │ Chain Preview│   │   └───────────────┘   │
│                      │   └──────────────┘   │                       │
│   ui_health_check.js │                      │   Diagnostics Tab     │
└──────────────────────┴──────────────────────┴───────────────────────┘
               │ invoke() / listen()              │
               │ Tauri IPC Bridge                 │
               ▼                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        BACKEND (Rust)                                │
├──────────────────┬─────────────────┬──────────────────┬─────────────┤
│  pipeline_engine │  connectors/    │  integrations/   │  config/    │
│  ┌────────────┐  │  ┌───────────┐  │  ┌────────────┐  │  storage    │
│  │ Step       │  │  │ llm.rs    │  │  │ slack.rs   │  │  Keychain   │
│  │ Executor   │  │  │ save.rs   │  │  │ notion.rs  │  │             │
│  │ Prompt Aug │  │  │ slack.rs  │  │  │ profiles   │  │             │
│  │ State Mgr  │  │  │ notion.rs │  │  │ (schema)   │  │             │
│  └────────────┘  │  └───────────┘  │  └────────────┘  │             │
└──────────────────┴─────────────────┴──────────────────┴─────────────┘
               │                                  │
               ▼                                  ▼
┌─────────────────────────────┐  ┌────────────────────────────────────┐
│  ~/.nbp/config/             │  │  External APIs                     │
│  settings.json              │  │  Notion REST API (v1)              │
│  pipelines.json             │  │  Slack Web API                     │
│  prompt-templates.json      │  │  OpenAI / Anthropic / Google       │
│  integrations/              │  └────────────────────────────────────┘
│    notion-{id}.json         │
│    (schema profiles)        │
└─────────────────────────────┘
```

---

## Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| **Pipeline Chips (JS)** | Display pipeline list in app bar; click = start recording with pre-assigned pipeline | `invoke("assign_pipeline")`, `invoke("start_recording")` |
| **Pipeline Builder (JS)** | Preset picker UI; step add/remove/reorder; config form per step type | `invoke("list_pipelines")`, `invoke("save_pipeline")`, `invoke("list_integrations_all")` |
| **Integrations Manager (JS)** | Setup wizard per integration type; schema sync; health test | `invoke("add_notion_integration")`, `invoke("sync_notion_schema")`, `invoke("test_integration")` |
| **UI Health Check (JS)** | DOM audit, click simulation, state verification; runs on startup | Internal JS only; reads health_report via `invoke("get_health_report")` |
| **pipeline_engine.rs** | Sequential step execution; prompt augmentation; state tracking | `connectors/*`, `integrations/*`, `prompt_templates`, `storage` |
| **connectors/llm.rs** | AI prompt execution; accepts augmented prompt; writes `.md` output | `cloud_ai/*`, `prompt_templates` |
| **connectors/notion.rs** (new) | Parse structured JSON from LLM output; map people names → user IDs; create Notion pages | `integrations::notion`, `reqwest` |
| **integrations/notion.rs** (new) | Notion API client: list DBs, read schema, list users, save profile to disk | Notion REST API, `~/.nbp/config/integrations/` |
| **integrations/slack.rs** (existing) | Slack token management; channel listing | Slack Web API, macOS Keychain |
| **prompt_templates.rs** (existing) | Template registry; `substitute_variables(prompt, transcript)` | `~/.nbp/config/prompt-templates.json` |
| **config.rs / storage.rs** (existing) | Settings persistence; recording metadata; file paths | Filesystem (`~/.nbp/`, `~/nbp-data/`) |

---

## Recommended Project Structure

```
src-tauri/src/
├── connectors/
│   ├── mod.rs              # strip_frontmatter, ConnectorTrait (new)
│   ├── llm.rs              # AI step — accepts augmented prompt (extend)
│   ├── save.rs             # File save — no changes
│   ├── slack.rs            # Slack post — no changes
│   ├── webhook.rs          # HTTP post — no changes
│   └── notion.rs           # NEW: JSON parser + Notion page creator
│
├── integrations/
│   ├── mod.rs              # IntegrationProfile enum, list_all_integrations command
│   ├── slack.rs            # Existing Slack integration
│   └── notion.rs           # NEW: schema sync, user listing, profile storage
│
├── pipeline_engine.rs      # EXTEND: add prompt augmentation step before LLM calls
├── pipelines.rs            # EXTEND: add ConnectorType::Notion, integration_id field
├── prompt_templates.rs     # Existing
├── config.rs               # EXTEND: integration profile directory helpers
└── storage.rs              # EXTEND: tags → pipeline_labels migration

src/
├── main.js                 # EXTEND: pipeline chip rendering, chip click → record
├── pipeline-builder.js     # NEW: extracted from main.js; preset picker logic
├── integrations-settings.js # NEW: Notion/Slack setup wizard, schema display
├── ui-health-check.js      # NEW: DOM audit, click simulation, health badge
└── styles.css              # EXTEND: chip styles, step picker styles
```

### Structure Rationale

- **connectors/notion.rs separate from integrations/notion.rs:** The integration module owns API auth and schema fetching (setup-time). The connector owns structured output parsing and page creation (runtime). These have different lifecycles and different failure modes.
- **pipeline-builder.js extracted from main.js:** main.js is already large. The builder is a complex sub-system that will grow significantly. Extract early before it becomes unmanageable.
- **Integration profiles as JSON files (not inside settings.json):** Schema can be large (many DB properties, many users). Keeping it out of the main settings file prevents settings.json bloat and allows per-integration sync without touching app settings.

---

## Architectural Patterns

### Pattern 1: Integration Profile (Schema Store)

**What:** A per-integration JSON file stored at `~/.nbp/config/integrations/{type}-{id}.json` containing the connection parameters, DB schema, people mappings, and defaults. Written at setup time, read at pipeline execution time.

**When to use:** Any "smart" connector (Notion, Linear, Jira) that needs schema-awareness. Not needed for "dumb" connectors (Slack, Save, Webhook).

**Trade-offs:** Snapshot of schema at setup time. Schema drift requires manual re-sync. Simpler than fetching schema on every run; avoids API calls during pipeline execution.

**Example:**
```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotionIntegrationProfile {
    pub id: String,
    pub name: String,
    pub database_id: String,
    pub database_name: String,
    pub properties: Vec<NotionPropertyDef>,  // snapshot of DB schema
    pub people_mappings: Vec<PeopleMapping>, // "СК" → notion_user_id
    pub defaults: serde_json::Value,         // { "status": "Todo" }
    pub synced_at: String,                   // ISO timestamp
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotionPropertyDef {
    pub name: String,
    pub property_type: String,   // "title", "people", "select", "date", etc.
    pub select_options: Vec<String>, // populated for select/multi-select only
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PeopleMapping {
    pub alias: String,           // "СК", "Sergey"
    pub notion_user_id: String,  // UUID
    pub display_name: String,    // "Sergey Kopanev"
}
```

### Pattern 2: Prompt Augmentation via Look-Ahead

**What:** The pipeline engine, before executing an LLM step, inspects the next step in the pipeline. If the next step is a structured connector (e.g., Notion), it loads the integration profile and appends a format specification to the base prompt.

**When to use:** Whenever an LLM step is immediately followed by a structured delivery step. The augmentation is invisible to the user but deterministic.

**Trade-offs:** Tight coupling between LLM step and the step that follows it. The look-ahead is simple (only N+1 step) — does not chain multiple future steps. Augmented prompt is longer but prevents hallucinated output formats.

**Example:**
```rust
// In pipeline_engine.rs execute_pipeline_internal(), before calling connectors::llm::execute()
fn build_augmented_prompt(
    base_prompt: &str,
    pipeline: &Pipeline,
    step_index: usize,
) -> String {
    // Look-ahead: what's the next step?
    let next_step = pipeline.steps.get(step_index + 1);

    let Some(next) = next_step else {
        return base_prompt.to_string();
    };

    match next.connector {
        ConnectorType::Notion => {
            if let Some(integration_id) = next.config.get("integration_id").and_then(|v| v.as_str()) {
                if let Ok(profile) = load_notion_profile(integration_id) {
                    let format_spec = build_notion_format_spec(&profile);
                    return format!("{}\n\n{}", base_prompt, format_spec);
                }
            }
            base_prompt.to_string()
        }
        _ => base_prompt.to_string(), // dumb connectors: no augmentation
    }
}

fn build_notion_format_spec(profile: &NotionIntegrationProfile) -> String {
    // Build: "Output as JSON array. Each item must have: title (string),
    //         assignee (one of [СК, СШ]), priority (one of [High, Medium, Low])"
    let mut lines = vec![
        "Output as a JSON array. Each item must be a JSON object with these fields:".to_string()
    ];
    for prop in &profile.properties {
        match prop.property_type.as_str() {
            "title" => lines.push(format!("- {}: string", prop.name)),
            "people" => {
                let aliases: Vec<&str> = profile.people_mappings.iter()
                    .map(|m| m.alias.as_str()).collect();
                lines.push(format!("- {}: one of {:?} or null", prop.name, aliases));
            }
            "select" => {
                lines.push(format!("- {}: one of {:?} or null", prop.name, prop.select_options));
            }
            "date" => lines.push(format!("- {}: ISO 8601 date string or null", prop.name)),
            _ => {}
        }
    }
    lines.join("\n")
}
```

### Pattern 3: Structured Output Parser in Connector

**What:** The Notion connector (and future structured connectors) does not assume the LLM output is perfect. It strips frontmatter, extracts the JSON array from the body (tolerating markdown code fences), and for each item maps aliases to Notion user IDs before calling the API.

**When to use:** Any structured connector. This is the runtime counterpart to Pattern 2.

**Trade-offs:** Requires JSON from the LLM. If the LLM returns free-form text (not JSON), the connector fails with a clear error. Retry logic (re-prompt with stricter instructions) can be added later; for v1, fail-and-report is acceptable.

**Example:**
```rust
// In connectors/notion.rs
fn extract_json_array(content: &str) -> Result<Vec<serde_json::Value>, String> {
    // Strip frontmatter first
    let body = crate::connectors::strip_frontmatter(content);

    // Try direct parse
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(body.trim()) {
        return Ok(arr);
    }

    // Try extracting from ```json ... ``` code fence
    if let Some(start) = body.find("```json") {
        if let Some(end) = body[start..].find("```\n").or_else(|| body[start..].find("```")) {
            let json_str = &body[start + 7..start + end];
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(json_str.trim()) {
                return Ok(arr);
            }
        }
    }

    Err(format!(
        "Could not parse structured output as JSON array. Raw output:\n{}",
        &body[..body.len().min(200)]
    ))
}
```

### Pattern 4: Pipeline Chips as Pre-Assignment UI

**What:** Pipeline chips in the app bar are rendered from `invoke("list_pipelines")`. Clicking a chip sets a `pendingPipelineId` in JS state. When recording stops, the engine immediately assigns and executes the pipeline. During recording, a chip click calls `invoke("assign_pipeline", { recording_id, pipeline_name })` which writes `waiting` status to metadata.

**When to use:** This is the primary recording entry point. The chip IS the workflow trigger — not just a label.

**Trade-offs:** Requires `list_pipelines` to be fast (it reads one file — fine). The chip list grows with the number of pipelines; cap visible chips at N (e.g., 5) with an overflow menu.

**Example (JS):**
```javascript
async function renderPipelineChips() {
    const pipelines = await invoke('list_pipelines');
    const container = document.getElementById('pipeline-chips');
    container.innerHTML = '';

    // Show up to 5 chips + overflow
    const visible = pipelines.slice(0, 5);
    visible.forEach(p => {
        const chip = document.createElement('button');
        chip.className = 'pipeline-chip';
        chip.dataset.pipeline = p.name;
        chip.textContent = p.name;
        chip.addEventListener('click', () => startRecordingWithPipeline(p.name));
        container.appendChild(chip);
    });
}

async function startRecordingWithPipeline(pipelineName) {
    pendingPipelineId = pipelineName;
    await invoke('start_recording');
    // highlight active chip
    document.querySelectorAll('.pipeline-chip').forEach(c => {
        c.classList.toggle('active', c.dataset.pipeline === pipelineName);
    });
}
```

---

## Data Flow

### Primary Flow: Schema → Prompt Augmentation → AI → Structured Output → API

```
[Notion Integration Setup]
         |
         | invoke("sync_notion_schema", { integration_id })
         ▼
[integrations/notion.rs]
  GET /v1/databases/{id}        ← Notion API
  GET /v1/users                 ← Notion API (list workspace users)
         |
         | Writes NotionIntegrationProfile
         ▼
[~/.nbp/config/integrations/notion-{id}.json]

─────────────────────────── PIPELINE EXECUTION ───────────────────────────

[Recording Stops → Auto-transcribe → Pipeline assigned → Execute]
         |
         ▼
[pipeline_engine::execute_pipeline_internal()]
         |
         | For each LLM step:
         |   build_augmented_prompt(base_prompt, pipeline, step_index)
         |     → Load integration profile (look-ahead to next step)
         |     → Append JSON format spec derived from DB schema
         ▼
[connectors/llm::execute(augmented_prompt)]
         |
         | Calls cloud AI provider (OpenAI / Anthropic / Google)
         ▼
[AI Response: JSON array of structured items]
         |
         | Written to ~/nbp-data/{id}/pipelines/{name}/{step}.md
         ▼
[connectors/notion::execute(step_output.md)]
         |
         | extract_json_array(content)
         | For each item:
         |   resolve_people_aliases(item, profile.people_mappings)
         |   resolve_select_options(item, profile.properties)
         |   POST /v1/pages { parent: db_id, properties: {...} }
         ▼
[Notion database: new pages created]
```

### Secondary Flow: Tags → Pipeline Labels Migration

```
[App Startup]
         |
         | storage::migrate_tags_to_pipeline_labels()
         ▼
[metadata.json: tags: ["hltm", "self"]]
         |
         | Convert each tag to a pipeline "label" (0-step pipeline reference)
         ▼
[metadata.json: pipelines: [{ name: "hltm", status: "label" }, ...]]
```

### Integration Profile Lifecycle

```
[User opens Settings > Integrations]
         ▼
[Notion Setup Wizard]
  Step 1: Enter API key → invoke("add_notion_integration", { id, name, token })
           → Validate token: GET /v1/users/me
           → Save token to macOS Keychain
  Step 2: invoke("list_notion_databases") → GET /v1/search (filter: database)
           → User picks database
  Step 3: invoke("sync_notion_schema", { integration_id, database_id })
           → GET /v1/databases/{id}  → read properties
           → GET /v1/users           → read workspace users
           → Store as NotionIntegrationProfile JSON
  Step 4: UI renders people mapping
           → User maps aliases → Notion users
           → invoke("update_notion_people_mappings", { integration_id, mappings })
           → Updates profile JSON
```

---

## Tauri Command Surface (New Commands to Register)

| Command | Module | Purpose |
|---------|--------|---------|
| `list_integrations_all` | `integrations/mod.rs` | Returns all configured integrations (Slack + Notion + Save paths) for pipeline builder |
| `add_notion_integration` | `integrations/notion.rs` | Validate API key, save token to Keychain, persist metadata |
| `list_notion_databases` | `integrations/notion.rs` | Call Notion API: list databases user has access to |
| `sync_notion_schema` | `integrations/notion.rs` | Fetch DB properties + workspace users → write integration profile |
| `update_notion_people_mappings` | `integrations/notion.rs` | Update alias → user_id mappings in profile |
| `test_notion_integration` | `integrations/notion.rs` | Verify token is still valid |
| `remove_notion_integration` | `integrations/notion.rs` | Delete token from Keychain, remove profile JSON |
| `get_integration_profile` | `integrations/mod.rs` | Return full profile (schema + mappings) for a given integration ID |
| `run_ui_health_check` | (JS-driven, no Rust needed) | JS-side DOM audit; returns health_report object |

---

## Integration Points

### Notion API (REST)

| Operation | Endpoint | Notes |
|-----------|----------|-------|
| Validate token | `GET /v1/users/me` | Returns bot user; fails with 401 if invalid |
| List databases | `POST /v1/search` with filter `database` | Returns paginated results; user must have shared DB with integration |
| Retrieve DB schema | `GET /v1/databases/{id}` | Returns `properties` map with type + config per column |
| List workspace users | `GET /v1/users` | Paginated; requires `user_capabilities` in integration settings |
| Create page | `POST /v1/pages` | Structured properties; people = array of user IDs |

**Authentication:** Internal integration API key (stored in macOS Keychain via `security_framework`). API key approach is the correct choice for a single-user desktop app — OAuth 2.0 is for multi-workspace distribution. (MEDIUM confidence, verified against official Notion docs.)

**Notion API versioning:** As of 2025-09, Notion introduced `2025-09-03` API version with `data_source_id` replacing `database_id` for multi-source databases. For v1, target the pre-`2025-09-03` API (`Notion-Version: 2022-06-28` header) since multi-source databases are an enterprise feature. (LOW confidence on exact version to pin — verify against Notion API changelog before implementation.)

### Internal Module Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `pipeline_engine` ↔ `connectors/*` | Direct function calls (same process) | `connectors::llm::execute()`, `connectors::notion::execute()` |
| `pipeline_engine` ↔ `integrations/*` | Direct function calls | Load profile JSON at execution time |
| `connectors/notion` ↔ `integrations/notion` | Read-only: connector loads profile; integration writes profile | No write from connector to profile |
| Frontend ↔ Backend | Tauri `invoke()` + `listen()` (IPC) | `pipeline-progress` event for step updates |
| `integrations/notion` ↔ Notion API | `reqwest` async HTTP client | Reuse existing `reqwest` dep (already used in Slack) |

---

## Anti-Patterns

### Anti-Pattern 1: Fetching Schema at Pipeline Execution Time

**What people do:** Call `GET /v1/databases/{id}` every time a pipeline runs to get fresh schema.
**Why it's wrong:** Adds network latency to every execution, breaks pipelines when offline, doubles API calls, and Notion rate limits integrations to 3 requests/second.
**Do this instead:** Snapshot schema at setup time into the integration profile. Provide a manual "Sync schema" button in settings. Schema changes in Notion are infrequent; the snapshot approach is reliable for 99% of cases.

### Anti-Pattern 2: Storing Integration Profile Inside AppSettings

**What people do:** Add `notion: HashMap<String, NotionIntegrationProfile>` to `AppSettings` in `config.rs`.
**Why it's wrong:** Schema profiles can be large (many properties, hundreds of users). Loading all profiles into memory at app startup is wasteful. The settings mutex becomes a bottleneck. Serializing large schemas into `settings.json` risks file corruption and slow writes.
**Do this instead:** Store each profile as a separate JSON file at `~/.nbp/config/integrations/{type}-{id}.json`. Load on demand when pipeline engine needs it.

### Anti-Pattern 3: Embedding Format Spec in the User-Facing Prompt Template

**What people do:** Ask users to write `"Output as JSON with fields: title, assignee..."` in their prompt template.
**Why it's wrong:** Users don't know the Notion schema. The schema can change. The format spec is implementation detail, not user intent. It breaks if the schema changes and the user forgets to update.
**Do this instead:** Keep the user prompt expressing intent only ("Extract action items"). The engine auto-appends the format spec from the integration profile transparently. The user never sees or manages it.

### Anti-Pattern 4: Making Pipeline Chips Redirect to Builder on Click

**What people do:** Make pipeline chips navigate to the pipeline detail view when clicked.
**Why it's wrong:** Destroys the primary UX goal: chip click = immediate record start. Navigation adds friction and breaks the "click → record" flow.
**Do this instead:** Chip click = `start_recording_with_pipeline(name)`. If the user wants to edit the pipeline, they go via Settings > Pipelines (separate path).

### Anti-Pattern 5: Running UI Health Check as Synchronous DOM Query Loop

**What people do:** Run `document.querySelectorAll(...)` checks synchronously on app startup in a loop, blocking UI render.
**Why it's wrong:** The health check runs after the DOM settles; synchronous execution during load blocks first paint and creates a bad startup experience.
**Do this instead:** Use `requestIdleCallback` or a short `setTimeout(healthCheck, 500)` after `DOMContentLoaded`. Health check should be a low-priority background task.

---

## Suggested Build Order (Phase Dependencies)

The new components have clear dependency layers:

```
Phase 1: Integration Profile Infrastructure (no UI dependencies)
  ├── integrations/notion.rs (API client, schema sync, profile storage)
  ├── config.rs: get_integrations_dir() helper
  └── New Tauri commands registered in lib.rs

Phase 2: Notion Connector (depends on Phase 1 profiles)
  ├── connectors/notion.rs (JSON parser, page creator)
  └── pipelines.rs: ConnectorType::Notion + validation

Phase 3: Prompt Augmentation (depends on Phase 1 + 2)
  ├── pipeline_engine.rs: build_augmented_prompt() look-ahead
  └── Tests: augmented prompt contains correct format spec

Phase 4: Integration Settings UI (depends on Phase 1 Tauri commands)
  ├── integrations-settings.js: Notion setup wizard
  ├── Schema display + people mapping UI
  └── Test/remove integration controls

Phase 5: Pipeline Builder Redesign (depends on Phase 4 integration data)
  ├── pipeline-builder.js: preset picker, step types
  ├── Delivery picker: shows only connected integrations
  └── Smart defaults (auto-name, auto-input chaining)

Phase 6: Pipeline Chips (depends on Phase 5 pipeline model)
  ├── main.js: chip rendering in app bar
  ├── startRecordingWithPipeline() flow
  └── Default pipeline setting

Phase 7: Tags → Pipeline Labels Migration
  ├── storage.rs: migrate_tags_to_pipeline_labels()
  └── RecordingMetadata: deprecate tags field

Phase 8: UI Health Check (depends on all UI components existing)
  └── ui-health-check.js: DOM audit, badge
```

**Rationale:** Backend infrastructure (Phases 1-3) before UI (Phases 4-6). The delivery picker in the builder needs real integration data to render — it can't be built without the integration backend. Prompt augmentation must be correct before the Notion connector is useful. Migration (Phase 7) is destructive — defer until pipeline model is stable.

---

## Scalability Considerations

This is a single-user desktop app. "Scale" means performance of local operations, not concurrent users.

| Concern | Current | With v2 |
|---------|---------|---------|
| Pipeline list loading | Fast (one JSON file) | Still fast; chip render is synchronous list |
| Integration profile load | N/A | File read per execution; negligible for 1-5 integrations |
| Notion API calls (setup) | N/A | 3-4 calls at setup time; not on hot path |
| Prompt length (augmented) | Base template | +50-200 chars for format spec; well within context limits |
| Schema drift | N/A | Manual re-sync button; acceptable for desktop app |

---

## Sources

- Notion REST API — Property Object: https://developers.notion.com/reference/property-object (HIGH confidence)
- Notion REST API — Authentication (Internal vs OAuth): https://developers.notion.com/docs/authorization (HIGH confidence)
- Notion REST API — List Users: https://developers.notion.com/reference/get-users (HIGH confidence)
- notionrs crate — lib.rs: https://lib.rs/crates/notionrs (MEDIUM confidence — active maintenance confirmed, specific API surface not verified)
- notion-client crate — crates.io: https://crates.io/crates/notion-client (MEDIUM confidence — updated November 2025)
- Tauri v2 Architecture: https://v2.tauri.app/concept/architecture/ (HIGH confidence)
- Existing codebase: `/workspace/src-tauri/src/` — pipeline_engine.rs, connectors/, integrations.rs, config.rs (HIGH confidence — direct analysis)

---
*Architecture research for: NBP Pipelines v2 — Schema-aware connectors, integration profiles, prompt augmentation*
*Researched: 2026-02-18*
