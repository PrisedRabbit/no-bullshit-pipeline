# Requirements: NBP Pipelines v2

**Defined:** 2026-02-18
**Core Value:** Zero post-recording work — select pipeline, record, stop, everything happens automatically.

## v1 Requirements

Requirements for the Pipelines v2 milestone. Each maps to roadmap phases.

### Pipeline Model

- [ ] **PIPE-01**: Pipeline with zero steps functions as a label (replaces tags concept)
- [ ] **PIPE-02**: User can create, edit, and delete pipelines with Processing and Delivery steps
- [ ] **PIPE-03**: Recording metadata stores pipeline references instead of tags
- [ ] **PIPE-04**: Existing tag data migrates to pipeline labels automatically on access (lazy migration)
- [ ] **PIPE-05**: Multiple pipelines can be assigned to a single recording, each writing to its own output directory

### Pre-Assignment UX

- [ ] **ASGN-01**: Pipeline chips appear in the app bar next to the record button
- [ ] **ASGN-02**: Clicking a pipeline chip starts recording immediately with that pipeline pre-assigned
- [ ] **ASGN-03**: Pipeline chips remain active during recording for mid-recording assignment
- [ ] **ASGN-04**: User can assign/change pipeline after recording in the detail view
- [ ] **ASGN-05**: Default pipeline setting in Settings applies to all new recordings unless overridden
- [ ] **ASGN-06**: Last-used pipeline is remembered and highlighted on next app launch
- [ ] **ASGN-07**: Chip bar shows top N pipelines with overflow menu for additional pipelines

### Pipeline Builder

- [ ] **BLDR-01**: Step picker shows two categories: Processing (AI) and Delivery (send somewhere)
- [ ] **BLDR-02**: Built-in processing presets available with one click: Meeting Notes, Action Items, Summary, Structure, Custom Prompt
- [ ] **BLDR-03**: Preset steps add with zero fields to fill (smart defaults for name, connector, input)
- [ ] **BLDR-04**: Custom prompt step has one field (textarea) with optional "Save as reusable template" checkbox
- [ ] **BLDR-05**: Step input chaining is automatic: step 1 = transcript, step N = previous step output, with toggle to override
- [ ] **BLDR-06**: Pipeline steps can be reordered via drag-and-drop
- [ ] **BLDR-07**: Pipeline assembly preview shows visual chain of steps below the step list
- [ ] **BLDR-08**: Provider/Model hidden by default, uses global settings; per-step override available in Advanced section

### Integrations

- [ ] **INTG-01**: Integrations settings page shows Connected and Available sections
- [ ] **INTG-02**: Each connected integration shows Test and Remove actions inline
- [ ] **INTG-03**: Save paths are first-class integrations with named locations (e.g., "Notes folder" → ~/Documents/notes/)
- [ ] **INTG-04**: Delivery step picker in pipeline builder shows only connected integrations
- [ ] **INTG-05**: Integration profiles stored as separate JSON files per integration (not in settings.json)

### Notion Connector

- [ ] **NOTN-01**: User can add Notion integration via API key (internal integration token)
- [ ] **NOTN-02**: Notion API key stored securely in macOS Keychain
- [ ] **NOTN-03**: Setup wizard: user picks database from list fetched via Notion API
- [ ] **NOTN-04**: Setup wizard: app reads database schema (properties, select options, people) automatically
- [ ] **NOTN-05**: Setup wizard: user maps conversation aliases to Notion workspace users (people mapping)
- [ ] **NOTN-06**: Schema and people mappings stored as Integration Profile
- [ ] **NOTN-07**: Schema re-sync available via manual button in integration settings
- [ ] **NOTN-08**: Notion connector creates pages in the selected database with structured property values

### Prompt Augmentation

- [ ] **AUGM-01**: Pipeline engine auto-detects when an LLM step is followed by a structured delivery step (Notion)
- [ ] **AUGM-02**: Format instructions derived from the destination schema are auto-injected into the LLM prompt
- [ ] **AUGM-03**: User never writes format specs manually — schema-to-prompt is automatic
- [ ] **AUGM-04**: AI structured JSON output is validated against the integration profile schema before delivery
- [ ] **AUGM-05**: If AI output is not valid JSON, step fails with clear error message and raw output shown

### Pipeline Execution

- [ ] **EXEC-01**: After recording stops, auto-transcribe followed by auto-pipeline execution with no user action
- [ ] **EXEC-02**: Pipeline run status per recording visible in recording detail view (Waiting/Running/Done/Failed)
- [ ] **EXEC-03**: Failed pipeline step shows inline error with the specific step that failed and why
- [ ] **EXEC-04**: Notion connector normalizes select values (case-insensitive match) and resolves people aliases to user IDs

### UI Health Check

- [ ] **HLTH-01**: Automated DOM element audit runs on app startup (silent, badge in status bar)
- [ ] **HLTH-02**: Health check verifies all expected interactive elements exist and respond to events
- [ ] **HLTH-03**: Health report shows specific failures with suggested fixes
- [ ] **HLTH-04**: Interactive walkthrough available on first launch and on demand from Settings

## v2 Requirements

Deferred to future iterations. Tracked but not in current roadmap.

### Enhanced Automation

- **AUTO-01**: Prompt augmentation visibility toggle — show user what format instructions were auto-injected
- **AUTO-02**: Branching/conditional logic in pipelines (if transcript contains X, route to Y)
- **AUTO-03**: AI-suggested pipeline creation ("build me a pipeline for X")

### Additional Connectors

- **CONN-01**: Telegram connector for message delivery
- **CONN-02**: Linear connector with schema-aware setup
- **CONN-03**: Jira connector with schema-aware setup
- **CONN-04**: Google Sheets connector

### Advanced Features

- **ADVN-01**: Notion OAuth 2.0 support (for multi-workspace distribution)
- **ADVN-02**: Automatic schema re-sync on each pipeline run
- **ADVN-03**: Shared/team pipelines synced across devices
- **ADVN-04**: Workflow execution history dashboard

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Branching/conditional pipeline logic | 90% of use cases are linear chains; DAG execution engine rewrite deferred to v3 |
| OAuth for Notion | API key (internal integration) sufficient for single-user desktop app; OAuth adds redirect URI complexity, security review, token refresh |
| Real-time schema sync on every run | Adds latency, API rate limit risk, breaks offline use; manual sync button is correct |
| Telegram/Linear/Jira connectors | Ship Notion first to prove schema-aware pattern; additional connectors fragment v2 scope |
| MCP connector | Placeholder exists but no specification; defer until protocol stabilizes |
| Shared/team pipeline sync | Requires cloud infrastructure; out of scope for local desktop app |
| Workflow execution history dashboard | Enterprise feature; per-recording pipeline status is sufficient for personal use |
| CSS/UI framework adoption | Existing app uses vanilla JS; adding a framework mid-project creates dual-paradigm complexity |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PIPE-01 | Phase 7 | Pending |
| PIPE-02 | Phase 7 | Pending |
| PIPE-03 | Phase 7 | Pending |
| PIPE-04 | Phase 7 | Pending |
| PIPE-05 | Phase 7 | Pending |
| ASGN-01 | Phase 6 | Pending |
| ASGN-02 | Phase 6 | Pending |
| ASGN-03 | Phase 6 | Pending |
| ASGN-04 | Phase 6 | Pending |
| ASGN-05 | Phase 6 | Pending |
| ASGN-06 | Phase 6 | Pending |
| ASGN-07 | Phase 6 | Pending |
| BLDR-01 | Phase 5 | Pending |
| BLDR-02 | Phase 5 | Pending |
| BLDR-03 | Phase 5 | Pending |
| BLDR-04 | Phase 5 | Pending |
| BLDR-05 | Phase 5 | Pending |
| BLDR-06 | Phase 5 | Pending |
| BLDR-07 | Phase 5 | Pending |
| BLDR-08 | Phase 5 | Pending |
| INTG-01 | Phase 4 | Pending |
| INTG-02 | Phase 4 | Pending |
| INTG-03 | Phase 4 | Pending |
| INTG-04 | Phase 4 | Pending |
| INTG-05 | Phase 1 | Pending |
| NOTN-01 | Phase 1 | Pending |
| NOTN-02 | Phase 1 | Pending |
| NOTN-03 | Phase 2 | Pending |
| NOTN-04 | Phase 2 | Pending |
| NOTN-05 | Phase 2 | Pending |
| NOTN-06 | Phase 1 | Pending |
| NOTN-07 | Phase 1 | Pending |
| NOTN-08 | Phase 2 | Pending |
| AUGM-01 | Phase 3 | Pending |
| AUGM-02 | Phase 3 | Pending |
| AUGM-03 | Phase 3 | Pending |
| AUGM-04 | Phase 3 | Pending |
| AUGM-05 | Phase 3 | Pending |
| EXEC-01 | Phase 6 | Pending |
| EXEC-02 | Phase 6 | Pending |
| EXEC-03 | Phase 6 | Pending |
| EXEC-04 | Phase 2 | Pending |
| HLTH-01 | Phase 8 | Pending |
| HLTH-02 | Phase 8 | Pending |
| HLTH-03 | Phase 8 | Pending |
| HLTH-04 | Phase 8 | Pending |

**Coverage:**
- v1 requirements: 46 total
- Mapped to phases: 46
- Unmapped: 0

---
*Requirements defined: 2026-02-18*
*Last updated: 2026-02-18 — traceability populated after roadmap creation*
