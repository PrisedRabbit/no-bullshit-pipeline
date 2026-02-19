# NBP Pipelines v2

## What This Is

NBP (No Bullshit Pipeline) is a Tauri desktop app (Rust + Vanilla JS) that captures Mac audio (mic + system), transcribes it, and runs multi-step pipelines for processing and delivery. Pipelines v2 shipped a complete redesign: unified pipeline-as-label model (replacing tags), pre-assignment UX with pipeline chips, a preset-based builder with Processing/Delivery categories, a three-layer integrations architecture with schema-aware Notion connector, automatic prompt augmentation, and a built-in UI health check system.

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

### Active

(See REQUIREMENTS.md for v1.1 scoped requirements)

## Current Milestone: v1.1 Resilience & Polish

**Goal:** Harden the v1 pipeline system — fix known bugs, add error recovery for structured outputs, improve UX polish for pipeline chips and prompt augmentation visibility.

**Target features:**
- Fix audit tech debt (MutationObserver selector mismatch, dual Slack state consolidation)
- Structured output error recovery (retry logic, fallback display when AI returns invalid JSON)
- Pipeline chip overflow UX (top N chips + overflow menu for large pipeline collections)
- Prompt augmentation visibility (show user what context was auto-injected into AI prompts)
- Token budget validation for prompt augmentation against real schema sizes
- Schema re-sync improvements (detect stale schemas, prompt user to re-sync)

### Out of Scope

- Linear/Jira connectors — Notion first, others follow same pattern later
- Telegram connector — marked "soon" in designs, not shipped
- MCP connector — placeholder exists but no specification
- OAuth for Notion — API key (internal integration) sufficient for single-user
- Automatic schema re-sync on each pipeline run — manual button shipped
- Real-time collaborative pipeline editing — single-user app
- Pipeline marketplace/sharing — personal use focus
- Branching/conditional pipeline logic — 90% of use cases are linear chains
- CSS/UI framework adoption — vanilla JS approach maintained

## Context

**Current codebase state (v1 shipped):**
- 5 connectors: LLM, Save, Webhook, Slack, Notion (in `src-tauri/src/connectors/`)
- Pipeline engine (`pipeline_engine.rs`) with N+1 look-ahead prompt augmentation
- Integrations architecture with profiles in `~/.nbp/integrations/`
- 6 built-in prompt templates (meeting-notes, action-items, summary, structure, brainstorm, journal)
- Pipeline builder extracted to `src/pipeline-builder.js` with SortableJS
- Integrations settings wizard in `src/integrations-settings.js`
- UI health check engine in `src/ui-health-check.js`
- ~10,800 LOC across key v2 files
- Config in `~/.nbp/`, recordings in `~/nbp-data/`

**Known issues (from v1 audit):**
- MutationObserver selector mismatch in integrations-settings.js:887 (`.settings-tabs-container` should be `.settings-tabs`) — integrations tab first-load broken, works after any user action
- Dual Slack state (main.js vs integrations-settings.js) — cosmetic staleness until page reload
- Prompt augmentation token budget (<500 tokens) unvalidated against large Notion databases

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

---
*Last updated: 2026-02-19 after v1.1 Resilience & Polish milestone start*
