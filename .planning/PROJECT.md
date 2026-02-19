# NBP Pipelines v2

## What This Is

NBP (No Bullshit Pipeline) is a Tauri desktop app (Rust + Vanilla JS) that captures Mac audio (mic + system), transcribes it, and runs multi-step pipelines for processing and delivery. Pipelines v2 shipped a complete redesign: unified pipeline-as-label model (replacing tags), pre-assignment UX with pipeline chips, a preset-based builder with Processing/Delivery categories, a three-layer integrations architecture with schema-aware Notion connector, automatic prompt augmentation, and a built-in UI health check system. v1.1 hardened the system with bug fixes, structured output error recovery, UX polish, and schema management safeguards.

## Core Value

Zero post-recording work: select pipeline → record → stop → everything happens automatically (transcribe → process → deliver).

## Requirements

### Validated

- ✓ Audio capture (mic + system via Core Audio Process Taps) — existing
- ✓ Real-time mixing with EBU R128 normalization — existing
- ✓ OGG Vorbis encoding — existing
- ✓ In-app playback via rodio — existing
- ✓ Local Whisper transcription (Metal GPU) — existing
- ✓ Cloud transcription (OpenAI, Google, Anthropic) — existing
- ✓ Pipeline engine with sequential step execution — existing
- ✓ LLM connector (OpenAI, Google, Anthropic) — existing
- ✓ Save connector (filesystem with path variables) — existing
- ✓ Webhook connector (POST/PUT/PATCH) — existing
- ✓ Slack connector (channels, DMs, OAuth) — existing
- ✓ Prompt templates (built-in + custom, variable substitution) — existing
- ✓ Waveform visualization — existing
- ✓ Mic selection — existing
- ✓ Auto-discard short recordings — existing
- ✓ Tags replaced by pipelines (zero-step pipeline = label) — v1
- ✓ Pipeline chips in app bar for pre-assignment before recording — v1
- ✓ One-click pipeline chip starts recording with pipeline pre-assigned — v1
- ✓ Default pipeline setting for new recordings — v1
- ✓ Multiple pipelines per recording (each writes to own directory) — v1
- ✓ Pipeline builder redesign: preset picker instead of forms — v1
- ✓ Step categories: Processing and Delivery (not "connectors") — v1
- ✓ Built-in processing presets (Meeting Notes, Action Items, Summary, Structure, Custom) — v1
- ✓ Smart defaults: zero fields for preset steps — v1
- ✓ Input chaining: auto-link to previous step output, toggle for transcript — v1
- ✓ Prompt templates inline + reusable, with built-in presets — v1
- ✓ Provider/Model: global default, per-step override in Advanced — v1
- ✓ Integrations settings page (three-layer architecture) — v1
- ✓ Save paths as named integrations — v1
- ✓ Delivery step picker shows only connected integrations — v1
- ✓ Notion connector with structured property mapping — v1
- ✓ Integration profiles (schema, people mappings, defaults) — v1
- ✓ Prompt augmentation: auto-inject format specs from downstream schema — v1
- ✓ Structured output parser and validation for schema-aware connectors — v1
- ✓ UI health check: automated element audit on app start — v1
- ✓ UI health check: interactive walkthrough on first launch — v1
- ✓ Lazy tag-to-pipeline-label migration — v1
- ✓ Pipeline run status visibility (Waiting/Running/Done/Failed) — v1
- ✓ Auto-transcribe + auto-execute pipeline on recording stop — v1
- ✓ SortableJS drag-and-drop step reordering — v1
- ✓ Integrations tab renders correctly on first load (MutationObserver fix) — v1.1
- ✓ Slack state consistent across all UI views (single source of truth) — v1.1
- ✓ JSON retry with stricter prompt on structured output parse failure — v1.1
- ✓ Raw AI output preserved and shown on structured output failure — v1.1
- ✓ Partial-success pipeline execution (delivery failures don't halt independent steps) — v1.1
- ✓ Pipeline chip overflow menu for large collections with ARIA accessibility — v1.1
- ✓ Augmented prompt expandable section in pipeline run output — v1.1
- ✓ Per-step wall-clock timing in pipeline run output — v1.1
- ✓ Token budget validation before augmented LLM execution — v1.1
- ✓ Schema staleness warning (7-day threshold) in integration profiles — v1.1
- ✓ In-builder schema re-sync button for Notion steps — v1.1
- ✓ Linear connector with schema-aware structured output — v1.2 Phases 13-15
- ✓ Linear integration setup wizard (API key → team → schema → member alias mapping) — v1.2 Phase 16
- ✓ Pipeline builder shows Linear as delivery step option — v1.2 Phase 16
- ✓ Integration settings Linear setup with re-sync and staleness warnings — v1.2 Phase 16

### Active

- [ ] Telegram connector for simple message delivery (Bot API)
- [ ] Telegram integration setup wizard (bot token → chat selection)
- [ ] Pipeline builder shows Telegram as delivery step option
- [ ] Integrations settings page supports Telegram setup

### Out of Scope

- Jira connector — Linear first, Jira follows same pattern later
- Linear OAuth — API key sufficient for single-user (same rationale as Notion)
- MCP connector — placeholder exists but no specification
- OAuth for Notion — API key (internal integration) sufficient for single-user
- Automatic schema re-sync on each pipeline run — manual button shipped
- Real-time collaborative pipeline editing — single-user app
- Pipeline marketplace/sharing — personal use focus
- Branching/conditional pipeline logic — 90% of use cases are linear chains
- CSS/UI framework adoption — vanilla JS approach maintained

## Current Milestone: v1.2 Connector Expansion

**Goal:** Extend the pipeline delivery ecosystem with Linear (schema-aware structured output) and Telegram (simple message delivery) connectors, leveraging the architecture established in v1.

**Target features:**
- Linear connector — schema-aware setup wizard, structured property mapping, prompt augmentation integration
- Telegram connector — bot token auth, chat/group selection, simple message delivery
- Integration settings and pipeline builder updated for both new connectors

## Context

**Current codebase state (v1.2 Linear complete):**
- 6 connectors: LLM, Save, Webhook, Slack, Notion, Linear (in `src-tauri/src/connectors/`)
- Pipeline engine (`pipeline_engine.rs`) with N+1 look-ahead prompt augmentation (Notion + Linear), token budget validation, partial-success execution, and JSON retry
- Integrations architecture with profiles in `~/.nbp/integrations/` (Notion, Linear with member aliases)
- 6 built-in prompt templates (meeting-notes, action-items, summary, structure, brainstorm, journal)
- Pipeline builder extracted to `src/pipeline-builder.js` with SortableJS and in-builder schema re-sync
- Integrations settings wizard in `src/integrations-settings.js` with schema staleness warnings
- UI health check engine in `src/ui-health-check.js`
- Config in `~/.nbp/`, recordings in `~/nbp-data/`

**Known issues:** None — all v1 audit items resolved in v1.1.

**Design inputs:**
- Brainstorming session 2026-02-03: Pipeline data model, file structure, connectors
- Brainstorming session 2026-02-18: UX redesign — mental model, assembly, pre-assignment, schema-aware connectors, UI health check

## Constraints

- **Tech Stack**: Tauri (Rust backend + Vanilla JS frontend, no framework, no bundler)
- **Package Manager**: Bun only (no npm/npx/yarn)
- **Storage**: Recordings in `~/nbp-data/`, config in `~/.nbp/`
- **Security**: API keys and tokens in macOS Keychain
- **Platform**: macOS 13.0+ only
- **Privacy**: Local-first, no telemetry, no network calls unless user configures cloud APIs
- **Encoding**: OGG Vorbis, 48kHz stereo

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Tags replaced by pipelines | One concept instead of two. Pipeline with 0 steps = label. Simpler mental model. | ✓ Good — v1 Phase 7 |
| Pre-assignment via chips in app bar | Most valuable moment is BEFORE recording. Eliminates post-recording work. | ✓ Good — v1 Phase 6 |
| Preset picker, not forms | One click for known step types. Zero fields for presets. User-friendly. | ✓ Good — v1 Phase 5 |
| Processing + Delivery categories | Maps to user mental model, not implementation details. | ✓ Good — v1 Phase 5 |
| Global default provider/model | 95% of users use one provider. Per-step override hidden in Advanced. | ✓ Good — v1 Phase 5 |
| Auto input chaining | 90% of pipelines are linear chains. Previous step output by default. | ✓ Good — v1 Phase 5 |
| Save paths as integrations | Pre-configured named locations, consistent with other integrations. | ✓ Good — v1 Phase 4 |
| Schema-aware setup wizard for Notion | Reads DB schema → stores profile → augments AI prompts automatically. | ✓ Good — v1 Phases 1+4 |
| API key for Notion v1 | Simpler than OAuth. Internal integration is sufficient for single-user app. | ✓ Good — v1 Phase 1 |
| Notion connector: profile-driven property mapping | Iterate profile.properties (not JSON keys) to build typed PageProperty map. | ✓ Good — v1 Phase 2 |
| SortableJS as local vendor file | Native HTML5 DnD unreliable in macOS WKWebView. | ✓ Good — v1 Phase 5 |
| Integration profiles as separate JSON files | Not inside settings.json. Per-integration file isolation. | ✓ Good — v1 Phase 1 |
| Dev-mode credential bypass | .dev-credentials.json avoids Keychain dialogs during development. | ✓ Good — v1 Phase 1 |
| UI health check as lightweight internal module | Not Selenium/Playwright. querySelectorAll + state checks. <2 seconds. | ✓ Good — v1 Phase 8 |
| Prompt augmentation hard-fail on missing profile | Prevents expensive LLM call with guaranteed non-JSON output. | ✓ Good — v1 Phase 3 |
| Delegate Slack state to main.js single source | Eliminates duplicate Tauri invokes and stale state between views. | ✓ Good — v1.1 Phase 9 |
| NotionErrorKind enum for retry dispatch | Typed errors (JsonParse vs Other) enable retry-aware dispatch without string matching. | ✓ Good — v1.1 Phase 10 |
| Max 1 retry for structured output | No loop — prevents retry storms on consistently bad models. | ✓ Good — v1.1 Phase 10 |
| ConnectorType::is_delivery() classification | Delivery vs processing determines partial-success continuation. Exhaustive match. | ✓ Good — v1.1 Phase 10 |
| Sidecar .augmented-prompt.txt files | Avoids modifying connector frontmatter format across all connectors. | ✓ Good — v1.1 Phase 11 |
| Token budget pre-flight validation | Check budget before API call, not inside connector. Prevents costly over-limit calls. | ✓ Good — v1.1 Phase 12 |
| Manual schema re-sync with staleness warning | 7-day threshold + in-builder button. Simpler and safer than automatic re-sync. | ✓ Good — v1.1 Phase 12 |
| Raw reqwest GraphQL for Linear (no SDK) | Simpler, no new dependency. 3 queries for states/labels/members. | ✓ Good — v1.2 Phase 13 |
| Linear single JSON object (not array) | One issue per delivery step. Consistent with one-issue-per-step model. | ✓ Good — v1.2 Phase 14 |
| MemberAlias follows PeopleMapping pattern | Consistent alias resolution across Notion and Linear integrations. | ✓ Good — v1.2 Phase 16 |
| 4-step Linear wizard (no share step) | Linear API key grants direct team access — no DB-sharing step needed like Notion. | ✓ Good — v1.2 Phase 16 |

---
*Last updated: 2026-02-19 after Phase 16 (Linear Frontend)*
