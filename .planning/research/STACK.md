# Stack Research

**Domain:** Tauri desktop app — Pipelines v2 milestone (NBP)
**Researched:** 2026-02-18
**Confidence:** MEDIUM — Rust crate landscape verified via crates.io and GitHub; JS drag-and-drop verified via official/community sources; macOS testing gap confirmed via Tauri official docs.

---

## Scope

This file covers **only new dependencies** needed for Pipelines v2. The existing stack (Tauri 2.10, Rust, Vanilla JS, reqwest, serde_json, security-framework, tokio, whisper-rs, rodio, cpal) is locked and not re-listed.

---

## Recommended Stack

### New Rust Dependencies

| Crate | Version | Purpose | Why Recommended |
|-------|---------|---------|-----------------|
| `notion-client` | `1.0.11` | Notion API client | Most actively maintained Rust Notion crate as of 2026-02. Supports databases, pages, blocks, users, search. MIT license. Uses reqwest (already in Cargo.toml). Builder pattern API. |
| `jsonschema` | `0.29` (latest stable) | Validate AI JSON output against Notion schema | High-performance, actively maintained, serde_json integration. Used to verify AI-produced structured JSON matches Notion DB property schema before creating pages. |
| `schemars` | `0.8` | Generate JSON Schema from Rust structs | Derive macro generates schema from IntegrationProfile structs. Needed if we expose schema to AI prompt augmentation layer. |

### New JS Dependencies (Frontend)

| Library | Version | Purpose | Why Recommended |
|---------|---------|---------|-----------------|
| `sortablejs` | `1.15.x` (latest) | Drag-and-drop step reordering in pipeline builder | No framework required. Works with vanilla JS and plain DOM elements. Built-in touch support. Actively maintained (GitHub: SortableJS/Sortable). No jQuery. Single `new Sortable(el, options)` call — integrates cleanly into existing no-bundler setup. |

### No New Dev Tool Dependencies

The existing UI test approach (Playwright with webkit, `file://` protocol, mocked Tauri IPC) is continued for the UI health check feature. No new testing frameworks needed — the health check is an **in-app runtime module** (`ui_health_check.js`), not a test framework.

---

## Installation

```bash
# Rust — add to src-tauri/Cargo.toml [dependencies]
# notion-client = "1.0.11"
# jsonschema = "0.29"
# schemars = { version = "0.8", features = ["derive"] }

# JS — load via CDN in index.html (no bundler, consistent with existing approach)
# <script src="https://cdn.jsdelivr.net/npm/sortablejs@1.15.6/Sortable.min.js"></script>
# Or: bun add sortablejs  (for local copy, reference from node_modules)
```

Note: The project uses no bundler and serves static files directly. For SortableJS, either the CDN approach or copying the minified file to `src/vendor/sortable.min.js` keeps it consistent with the existing zero-bundler constraint.

---

## Technology Decisions — Detailed Rationale

### Notion Integration: Raw reqwest vs notion-client

**Decision: Use `notion-client` crate, not raw reqwest calls.**

The existing Slack integration uses raw reqwest calls directly. For Notion this is not the right call because:

1. Notion's data model is complex: databases have typed properties (Select, MultiSelect, People, Date, Relation, etc.), each with different serialization shapes. Modeling this by hand in serde structs is a substantial effort.
2. `notion-client` already has Rust types for all Notion property types. The setup wizard reads `database.properties` — that's a heterogeneous map of typed property definitions — and these types are pre-built in the crate.
3. The crate uses `reqwest` (already in Cargo.toml), so it adds no new HTTP runtime.

**Notion API version to target: `2022-06-28` initially, with upgrade path to `2025-09-03` flagged.**

Version `2022-06-28` is stable and still works for all single-source databases (which is 100% of NBP's use case). The `2025-09-03` version introduced multi-source databases — a breaking change only relevant if a user adds a second data source to a Notion DB. For v1 of the Notion connector, pin to `2022-06-28`. The `notion-client` crate targets this version.

The upgrade to `2025-09-03` is a future concern (tracked in PITFALLS.md), not a day-one requirement.

### Drag-and-Drop: SortableJS vs Native HTML5 API vs html5sortable

**Decision: SortableJS.**

The pipeline builder needs step reordering (vertical list, drag to reorder). Three options evaluated:

- **Native HTML5 DnD API**: Achievable with ~50 lines of vanilla JS using `dragstart`/`dragover`/`drop` events. No dependencies. However: no visual drag ghost on macOS WebView (WKWebView does not replicate the element during drag), no smooth animation, inconsistent cursor behavior across platforms. The Tauri WKWebView is not a full browser — native DnD quirks are worse here than in Chrome.
- **html5sortable (2KB)**: Lightweight but the maintainer is actively seeking a co-maintainer (as of 2025). Last release is stale. Not recommended for a new feature.
- **SortableJS (28KB min)**: Actively maintained, no framework required, one-line initialization, built-in animation, touch support, works reliably in WebView contexts. The size overhead is acceptable for a desktop app with no bandwidth constraints.

SortableJS is the correct choice because of WebView DnD reliability. Load it as a local vendor file to avoid CDN dependency at runtime.

### UI Health Check: In-App Module vs External Test Framework

**Decision: Custom in-app `ui_health_check.js` module — no external test framework.**

The brainstorming session specified this clearly: "NOT Selenium/Playwright. It's a small internal module." This is the right call because:

- Tauri on macOS has **no WebDriver support** (confirmed via official Tauri docs: "only Windows and Linux are supported due to macOS not having a WKWebView driver tool available")
- The existing `test-ui.mjs` uses Playwright with mocked Tauri IPC — appropriate for CI on Linux, not for in-app runtime health checks
- The health check needs to run inside the real app at runtime, detecting real DOM state, not in a separate test process

The health check module uses `document.querySelector`, `dispatchEvent`, and `window.__TAURI__.core.invoke` directly — zero dependencies.

### JSON Schema Handling: jsonschema crate

**Decision: Use `jsonschema` crate for AI output validation.**

When the Notion connector receives AI output (structured JSON), it must validate the shape before making API calls — otherwise a malformed response causes a confusing Notion API error. The `jsonschema` crate validates a `serde_json::Value` against a JSON Schema derived from the integration profile. This is a lightweight one-shot validation step per pipeline execution.

`valico` was considered but is less actively maintained. `serde_valid` is derive-based (compile-time), which doesn't fit dynamic schemas loaded at runtime from integration profiles.

### Keychain for Notion API Keys

**Decision: Continue using `security-framework` (already in Cargo.toml).**

The Slack integration already uses `security-framework` for Keychain storage. The Notion API key follows the same pattern: `set_generic_password(KEYCHAIN_SERVICE, "notion:{integration_id}", token)`. No new dependency needed — just extend the existing keychain module in `integrations.rs`.

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `notion-client` crate | Raw `reqwest` calls | Only if notion-client becomes unmaintained or has breaking bugs. Raw reqwest is viable but requires hand-writing serde structs for all Notion property types — significant scope increase. |
| `notion-client` crate | `notionrs` crate | `notionrs` is explicitly alpha/not production-ready as of 2026-02. Revisit if notion-client is abandoned. |
| SortableJS (28KB) | Native HTML5 DnD | Use native DnD only if SortableJS causes WKWebView issues on specific macOS versions. A plain DnD fallback should be kept in mind. |
| SortableJS (28KB) | html5sortable (2KB) | html5sortable is maintenance-risk. Only prefer if bundle size becomes a real concern, which it won't in a desktop app. |
| `jsonschema` crate | Manual serde validation | Manual validation is fine for known-shape schemas. Use manual validation for simple 2-3 field cases; `jsonschema` for complex Notion schemas with many property types. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `tauri-driver` / WebdriverIO for UI health check | macOS is not supported by Tauri WebDriver. Health checks in the real app on macOS will not work. | Custom in-app `ui_health_check.js` module using DOM APIs |
| `notionrs` crate | Explicitly alpha, API stability not guaranteed | `notion-client` 1.0.x |
| `jakeswenson/notion` crate | Unmaintained (last release 2022), does not track current Notion API | `notion-client` 1.0.x |
| `rusticnotion` crate | Fork of unmaintained crate, not yet on crates.io | `notion-client` 1.0.x |
| Notion API version `2025-09-03` for v1 | Breaking change for multi-source databases; `notion-client` targets `2022-06-28`; no NBP use case for multi-source yet | Pin to `2022-06-28` for v1, upgrade path documented |
| OAuth 2.0 for Notion | Requires registering a public integration with Notion, callback URL handling, token refresh. Significant complexity vs API key. | Internal integration (API key) — user creates a Notion integration in their own workspace, pastes the secret token. Same pattern as Slack bot token. |
| Any CSS/UI framework | Existing app uses pure Vanilla JS + CSS. Adding a framework mid-project creates dual-paradigm complexity. | Vanilla JS + existing CSS patterns |

---

## Stack Patterns by Variant

**Notion connector setup wizard (schema reading):**
- Use `notion-client` to: `list_databases()` → pick database → `get_database(id)` to read `.properties` map → extract property types, People members, Select options
- Store as `NotionIntegrationProfile` struct in `~/.nbp/integrations.json`
- API key stored in Keychain via existing `security-framework` pattern

**Prompt augmentation (AI step before Notion delivery):**
- No new crate needed. Pure Rust: read `NotionIntegrationProfile`, generate format instruction string, append to LLM prompt in `connectors/llm.rs` before the API call
- Implement as `augment_prompt_for_next_step(prompt: &str, next_step: &PipelineStep) -> String`

**AI output validation (before Notion page creation):**
- Use `jsonschema` to validate AI-produced JSON against a schema derived from `NotionIntegrationProfile`
- If invalid: return structured error to pipeline engine → mark step as `failed` → emit event with recovery suggestion

**Pipeline step reordering (UI):**
- Load SortableJS from `src/vendor/sortable.min.js`
- `new Sortable(stepsContainer, { animation: 150, onEnd: (evt) => reorderStep(evt.oldIndex, evt.newIndex) })`
- `reorderStep()` calls `invoke('reorder_pipeline_step', {...})` Tauri command

**UI health check (runtime audit):**
- `ui_health_check.js` module with `runAudit()` → returns `{ passed: number, failed: number, issues: Issue[] }`
- Runs on `DOMContentLoaded` after app init, updates status bar badge
- No external dependencies

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `notion-client 1.0.11` | `reqwest 0.13`, `tokio 1.x`, `serde 1.x` | All already in Cargo.toml. No conflicts expected. |
| `jsonschema 0.29` | `serde_json 1.x` | Already in Cargo.toml. jsonschema requires Rust 1.83+; project is on Rust 2024 edition, fully compatible. |
| `schemars 0.8` | `serde 1.x` | Already in Cargo.toml. Derive-based, no runtime conflicts. |
| `sortablejs 1.15.x` | Vanilla JS, no bundler | Works as a plain `<script>` tag. No module system required. |

---

## Sources

- `notion-client` crate: [crates.io/crates/notion-client](https://crates.io/crates/notion-client) — version 1.0.11, active maintenance confirmed
- `notion-client` GitHub: [github.com/takassh/notion-client](https://github.com/takassh/notion-client) — endpoints: databases, pages, blocks, users, search, comments
- Notion API versioning: [developers.notion.com/docs/upgrade-guide-2025-09-03](https://developers.notion.com/docs/upgrade-guide-2025-09-03) — `2022-06-28` still works for single-source DBs (HIGH confidence)
- Notion API backward compat FAQs: [developers.notion.com/docs/upgrade-faqs-2025-09-03](https://developers.notion.com/docs/upgrade-faqs-2025-09-03) — no deprecation timeline for `2022-06-28` announced
- `jsonschema` crate: [crates.io/crates/jsonschema](https://crates.io/crates/jsonschema) — HIGH confidence, most maintained Rust JSON Schema validator
- SortableJS GitHub: [github.com/SortableJS/Sortable](https://github.com/SortableJS/Sortable) — no framework required, actively maintained (MEDIUM confidence — WebView behavior not independently verified)
- Tauri WebDriver docs: [v2.tauri.app/develop/tests/webdriver/](https://v2.tauri.app/develop/tests/webdriver/) — macOS not supported, confirmed (HIGH confidence)
- `security-framework` already in `Cargo.toml` at `3.5` — no change needed for Notion key storage (HIGH confidence)
- html5sortable maintenance concern: [github.com/lukasoppermann/html5sortable](https://github.com/lukasoppermann/html5sortable) — seeking co-maintainer, not recommended (MEDIUM confidence)

---

*Stack research for: NBP Pipelines v2 — new capability dependencies only*
*Researched: 2026-02-18*
