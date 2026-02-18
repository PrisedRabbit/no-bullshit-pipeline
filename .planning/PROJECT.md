# NBP Pipelines v2

## What This Is

NBP (No Bullshit Pipeline) is a Tauri desktop app (Rust + Vanilla JS) that captures Mac audio (mic + system), transcribes it, and runs multi-step pipelines for processing and delivery. Pipelines v2 is a complete redesign of how users think about, build, assign, and run pipelines — merging the tag and pipeline concepts, adding pre-assignment UX, a preset-based builder, an integrations architecture, schema-aware connectors, and a UI health check system.

## Core Value

Zero post-recording work: select pipeline → record → stop → everything happens automatically (transcribe → process → deliver).

## Requirements

### Validated

<!-- Existing capabilities confirmed in codebase -->

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
- ✓ Pipeline builder UI (settings tab, step management) — existing
- ✓ Pipeline assignment to recordings — existing
- ✓ Pipeline progress tracking with events — existing
- ✓ Waveform visualization — existing
- ✓ Mic selection — existing
- ✓ Auto-discard short recordings — existing
- ✓ Recording metadata with tags — existing

### Active

<!-- Pipelines v2 scope — building toward these -->

- [ ] Tags replaced by pipelines (zero-step pipeline = label)
- [ ] Pipeline chips in app bar for pre-assignment before recording
- [ ] One-click pipeline chip starts recording with pipeline pre-assigned
- [ ] Default pipeline setting for new recordings
- [ ] Multiple pipelines per recording (each writes to own directory)
- [ ] Pipeline builder redesign: preset picker instead of forms
- [ ] Step categories: Processing and Delivery (not "connectors")
- [ ] Built-in processing presets (Meeting Notes, Action Items, Summary, Structure, Custom)
- [ ] Smart defaults: zero fields for preset steps
- [ ] Input chaining: auto-link to previous step output, toggle for transcript
- [ ] Prompt templates inline + reusable, with built-in presets
- [ ] Provider/Model: global default, per-step override in Advanced
- [ ] Integrations settings page (three-layer architecture)
- [ ] Save paths as named integrations
- [ ] Delivery step picker shows only connected integrations
- [x] Notion connector with structured property mapping — Phase 2
- [x] Integration profiles (schema, people mappings, defaults) — Phase 1
- [ ] Prompt augmentation: auto-inject format specs from downstream schema
- [ ] Structured output parser for schema-aware connectors
- [ ] UI health check: automated element audit on app start
- [ ] UI health check: interactive walkthrough on first launch

### Out of Scope

<!-- Explicit boundaries for this milestone -->

- Linear/Jira connectors — Notion first, others follow same pattern later
- Telegram connector — marked "soon" in designs, not v2 scope
- MCP connector — placeholder exists but not in v2
- OAuth for Notion — API key (internal integration) for v1, OAuth later
- Automatic schema re-sync on each pipeline run — manual button for v2
- Real-time collaborative pipeline editing — single-user app
- Pipeline marketplace/sharing — personal use focus

## Context

**Existing codebase state:**
- Pipeline engine (`pipeline_engine.rs`) executes steps sequentially with progress events
- 4 connectors implemented: LLM, Save, Webhook, Slack (each in `src-tauri/src/connectors/`)
- Prompt templates stored in `~/.nbp/config/prompt-templates.json`
- Tags are `Vec<String>` in `metadata.json` — will be replaced by pipeline references
- Pipeline builder in settings tab of `src/main.js` — developer-oriented, needs redesign
- Slack already has OAuth + Keychain token storage
- Config in `~/.nbp/config/`, recordings in `~/nbp-data/`

**Design inputs:**
- Brainstorming session 2026-02-03: Pipeline data model, file structure, connectors
- Brainstorming session 2026-02-18: UX redesign — mental model, assembly, pre-assignment, schema-aware connectors, UI health check
- Both sessions produced detailed designs with specific UI mockups and architecture decisions

**Open questions resolved with defaults:**
1. Pipeline chips in app bar → show top N + overflow (N=5)
2. Notion auth → API key (internal integration) for simplicity
3. Schema re-sync → manual button in integration settings
4. Prompt augmentation visibility → show "Auto-formatted for Notion" indicator, expandable
5. Error recovery for structured output → retry once with stricter prompt, then show raw output with error message

## Constraints

- **Tech Stack**: Tauri (Rust backend + Vanilla JS frontend, no framework, no bundler)
- **Package Manager**: Bun only (no npm/npx/yarn)
- **Storage**: Recordings in `~/nbp-data/`, config in `~/.nbp/config/`
- **Security**: API keys and tokens in macOS Keychain
- **Platform**: macOS 13.0+ only
- **Privacy**: Local-first, no telemetry, no network calls unless user configures cloud APIs
- **Encoding**: OGG Vorbis, 48kHz stereo

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Tags replaced by pipelines | One concept instead of two. Pipeline with 0 steps = label. Simpler mental model. | — Pending |
| Pre-assignment via chips in app bar | Most valuable moment is BEFORE recording. Eliminates post-recording work. | — Pending |
| Preset picker, not forms | One click for known step types. Zero fields for presets. User-friendly. | — Pending |
| Processing + Delivery categories | Maps to user mental model, not implementation details. | — Pending |
| Global default provider/model | 95% of users use one provider. Per-step override hidden in Advanced. | — Pending |
| Auto input chaining | 90% of pipelines are linear chains. Previous step output by default. | — Pending |
| Save paths as integrations | Pre-configured named locations, consistent with other integrations. | — Pending |
| Schema-aware setup wizard for Notion | Reads DB schema → stores profile → augments AI prompts automatically. | Phase 1 backend (schema read, people mapping), Phase 4 UI |
| API key for Notion v1 | Simpler than OAuth. Internal integration is sufficient for single-user app. | Phase 1 — Keychain storage |
| Notion connector: profile-driven property mapping | Iterate profile.properties (not JSON keys) to build typed PageProperty map. Avoids Notion 400s for unknown fields. | Phase 2 — connectors/notion.rs |
| Notion connector: explicit User construction | notion-client User may not impl Default; list all 4 fields explicitly (id, name, avator_url, user_type). | Phase 2 — connectors/notion.rs |
| NOTN-03/04/05 reclassified Phase 2→4 | These are setup wizard UI requirements, not connector concerns. Backend built in Phase 1, UI in Phase 4. | Phase 2 verification |
| UI health check built-in | Verifies all interactive elements work. Lightweight, not a test framework. | — Pending |

---
*Last updated: 2026-02-18 after Phase 2*
