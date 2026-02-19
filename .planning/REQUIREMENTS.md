# Requirements: NBP v1.2 Connector Expansion

**Defined:** 2026-02-19
**Core Value:** Zero post-recording work — select pipeline → record → stop → everything happens automatically.

## v1.2 Requirements

Requirements for v1.2 release. Each maps to roadmap phases.

### Linear Connector

- [ ] **LINEAR-01**: User can add a Linear integration by entering an API key in integration settings
- [x] **LINEAR-02**: User can select a Linear team and project during setup wizard
- [ ] **LINEAR-03**: System reads and stores Linear project schema (custom fields, labels, priorities, status options, team members)
- [x] **LINEAR-04**: User can map Linear team members to name aliases for participant resolution
- [x] **LINEAR-05**: User can add a Linear delivery step in pipeline builder (only shown when Linear integration exists)
- [x] **LINEAR-06**: When an LLM step precedes a Linear step, prompt is auto-augmented with Linear schema format specs
- [ ] **LINEAR-07**: Structured LLM output is parsed and mapped to Linear issue fields (title, description, priority, label, assignee, status)
- [x] **LINEAR-08**: User can re-sync Linear project schema from pipeline builder and integration settings, with staleness warnings
- [ ] **LINEAR-09**: Linear API key stored in macOS Keychain with dev-mode credential bypass
- [ ] **LINEAR-10**: JSON parse failures on Linear delivery trigger structured-output retry (same pattern as Notion)

### Telegram Connector

- [ ] **TELE-01**: User can add a Telegram integration by entering a bot token in integration settings
- [ ] **TELE-02**: User can select a target chat/group/channel during Telegram setup (via bot's getUpdates or manual chat ID entry)
- [ ] **TELE-03**: User can add a Telegram delivery step in pipeline builder (only shown when Telegram integration exists)
- [ ] **TELE-04**: Pipeline step output delivered as Telegram message to configured chat
- [ ] **TELE-05**: Telegram bot token stored in macOS Keychain with dev-mode credential bypass

## Future Requirements

Deferred to future releases. Tracked but not in current roadmap.

### Additional Connectors

- **CONN-01**: Jira connector with schema-aware structured output (follows Linear/Notion pattern)
- **CONN-02**: MCP connector for extensible tool integration
- **CONN-03**: Telegram topic/thread targeting within group chats

### Pipeline Features

- **PIPE-01**: Branching/conditional pipeline logic
- **PIPE-02**: Pipeline templates (export/import shareable presets)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Linear OAuth | API key sufficient for single-user app (same rationale as Notion) |
| Jira connector | Linear first, Jira follows same pattern later |
| MCP connector | Placeholder exists but no specification yet |
| Telegram inline keyboards/buttons | Simple message delivery sufficient for v1.2 |
| Branching/conditional pipeline logic | 90% of use cases are linear chains |
| Pipeline marketplace/sharing | Personal use focus |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| LINEAR-01 | Phase 13 | Pending |
| LINEAR-02 | Phase 16 | Complete |
| LINEAR-03 | Phase 13 | Pending |
| LINEAR-04 | Phase 16 | Complete |
| LINEAR-05 | Phase 16 | Complete |
| LINEAR-06 | Phase 15 | Complete |
| LINEAR-07 | Phase 14 | Pending |
| LINEAR-08 | Phase 16 | Complete |
| LINEAR-09 | Phase 13 | Pending |
| LINEAR-10 | Phase 14 | Pending |
| TELE-01 | Phase 17 | Pending |
| TELE-02 | Phase 17 | Pending |
| TELE-03 | Phase 17 | Pending |
| TELE-04 | Phase 17 | Pending |
| TELE-05 | Phase 17 | Pending |

**Coverage:**
- v1.2 requirements: 15 total
- Mapped to phases: 15
- Unmapped: 0

---
*Requirements defined: 2026-02-19*
*Last updated: 2026-02-19 — traceability filled in after roadmap creation*
