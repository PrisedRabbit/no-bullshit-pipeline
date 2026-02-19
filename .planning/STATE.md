# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-19)

**Core value:** Zero post-recording work — select pipeline, record, stop, everything happens automatically.
**Current focus:** v1.2 Connector Expansion — Telegram Connector

## Current Position

Phase: 17 — Telegram Connector (READY TO PLAN)
Plan: 0/2 plans — plans need to be created
Status: Phase 16 complete, Phase 17 ready to plan
Last activity: 2026-02-19 — Phase 16 complete (Linear frontend: wizard, pipeline builder, re-sync)

Progress: [████████░░] 80% (4/5 phases)

## Performance Metrics

**Velocity (v1):** 8 phases, 20 plans — shipped 2026-02-18 to 2026-02-19
**Velocity (v1.1):** 4 phases, 7 plans, 12 tasks, 13 feat/fix commits — shipped 2026-02-19
**Velocity (v1.2):** Phase 13 done in 1 plan, 1 commit. Phase 14 done in 1 plan, 1 commit.

See milestones/v1-ROADMAP.md and milestones/v1.1-ROADMAP.md for full details.

## Accumulated Context

### Decisions

All decisions archived in PROJECT.md Key Decisions table (15 v1 + 7 v1.1 decisions).

**v1.2 decisions:**
- Linear uses raw reqwest GraphQL (no SDK crate) — simpler, no new dependency
- Linear auth header: raw token, no "Bearer" prefix
- 3 separate GraphQL queries for schema sync (states, labels, members) — clear error messages
- Priorities hardcoded (0-4) — Linear's priority levels are fixed
- Linear connector extracts single JSON object (not array like Notion) — one issue per delivery step
- Field resolution is case-insensitive against profile data — tolerant of LLM output casing
- graphql_request made pub(crate) so both integrations and connectors modules can share it
- Linear format spec outputs singular "JSON object" (not array) — consistent with one-issue-per-step model
- Hard fail on workflow_states.is_empty() && team_id.is_empty() — both empty = unsynced profile
- display_name preferred over name for member assignee — matches what users see in Linear UI
- [Phase 16]: MemberAlias struct follows Notion PeopleMapping pattern for consistent alias resolution across integrations
- [Phase 16]: 4-step Linear wizard omits Notion share-instruction step — Linear API key grants direct team access
- [Phase 16-02]: typeof linearProfiles guard in pipeline-builder.js — allows independent loading before integrations-settings.js globals are set
- [Phase 16-02]: Linear added to deliveryConnectors array — ensures delivery styling in pipeline preview

### Pending Todos

None.

### Blockers/Concerns

- [Carried from v1]: cargo check not available in execution environment — all code verified structurally, compilation deferred to first cargo tauri dev run

## Session Continuity

Last session: 2026-02-19
Stopped at: Phase 16 complete, transitioning to Phase 17 planning
Resume file: None

## Next Step

**Action:** Plan Phase 17 — Telegram Connector
**Context:** Linear connector is fully shipped (Phases 13-16). Phase 17 is the last phase in v1.2 — Telegram connector for simple message delivery. ROADMAP has 2 plan slots (17-01 backend, 17-02 frontend) but plans need to be created via /gsd:plan-phase 17.
