# Requirements: NBP v1.1 Resilience & Polish

**Defined:** 2026-02-19
**Core Value:** Zero post-recording work — select pipeline, record, stop, everything happens automatically.

## v1.1 Requirements

Requirements for v1.1 release. Each maps to roadmap phases.

### Bug Fixes

- [x] **BUG-01**: Integrations tab renders correctly on first load without requiring user interaction (MutationObserver selector fix)
- [x] **BUG-02**: Slack connection state is consistent across all UI views without requiring page reload (dual state consolidation)

### Error Recovery

- [x] **ERR-01**: System retries with a stricter prompt when AI returns invalid JSON for structured delivery steps (Notion, etc.)
- [x] **ERR-02**: User sees the raw AI output with an actionable error message when structured output parsing fails after retry
- [x] **ERR-03**: Pipeline execution continues to subsequent independent steps when one delivery step fails (partial success)

### UX Polish

- [ ] **UX-01**: Pipeline chips in app bar show top N pipelines with an overflow menu when more than N exist
- [ ] **UX-02**: User can see what context was auto-injected into AI prompts via an expandable "Augmented prompt" section in pipeline run output
- [ ] **UX-03**: Pipeline run output shows per-step timing (duration) for transparency

### Schema Management

- [ ] **SCHM-01**: System validates that prompt augmentation token budget fits within model context limits before executing AI step
- [ ] **SCHM-02**: Integration profile shows "last synced" timestamp and warns when schema is older than 7 days
- [ ] **SCHM-03**: User can trigger schema re-sync from within the pipeline builder (not just settings)

## v2 Requirements

Deferred to future milestones. Tracked but not in current roadmap.

### New Connectors

- **CONN-01**: Linear connector with schema-aware setup wizard
- **CONN-02**: Telegram connector for message delivery
- **CONN-03**: MCP connector for extensible tool use

### Advanced Pipelines

- **PIPE-01**: Branching/conditional logic in pipeline steps
- **PIPE-02**: Pipeline templates marketplace/sharing
- **PIPE-03**: Parallel step execution (fan-out/fan-in)

### Authentication

- **AUTH-01**: Notion OAuth flow (replacing API key for better UX)

## Out of Scope

| Feature | Reason |
|---------|--------|
| New connector types (Linear, Telegram, MCP) | Stabilize existing connectors first, add new ones in v2 |
| Notion OAuth | API key (internal integration) sufficient for single-user; OAuth adds complexity |
| Pipeline branching/conditional logic | 90% of use cases are linear chains; defer to v2 |
| CSS/UI framework adoption | Vanilla JS approach maintained per project constraints |
| Real-time collaborative editing | Single-user desktop app |
| Automatic schema re-sync on every pipeline run | Manual re-sync with staleness warning is sufficient for v1.1 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| BUG-01 | Phase 9 | Complete |
| BUG-02 | Phase 9 | Complete |
| ERR-01 | Phase 10 | Complete |
| ERR-02 | Phase 10 | Complete |
| ERR-03 | Phase 10 | Complete |
| UX-01 | Phase 11 | Pending |
| UX-02 | Phase 11 | Pending |
| UX-03 | Phase 11 | Pending |
| SCHM-01 | Phase 12 | Pending |
| SCHM-02 | Phase 12 | Pending |
| SCHM-03 | Phase 12 | Pending |

**Coverage:**
- v1.1 requirements: 11 total
- Mapped to phases: 11
- Unmapped: 0 ✓

---
*Requirements defined: 2026-02-19*
*Last updated: 2026-02-19 — traceability filled after roadmap creation*
