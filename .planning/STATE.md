# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-19)

**Core value:** Zero post-recording work — select pipeline, record, stop, everything happens automatically.
**Current focus:** v1.2 Connector Expansion

## Current Position

Phase: 16 — Linear Frontend (IN PROGRESS)
Plan: 1/2 plans executed (16-01 complete, 16-02 wave 2 pending)
Status: Phase 16 in progress — 16-01 complete (Linear wizard UI + member alias backend), 16-02 pending
Last activity: 2026-02-19 — 16-01 complete (MemberAlias backend, 4-step wizard, connected cards, available integration entry)

Progress: [█████░░░░░] 60% (3/5 phases)

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

### Pending Todos

None.

### Blockers/Concerns

- [Carried from v1]: cargo check not available in execution environment — all code verified structurally, compilation deferred to first cargo tauri dev run

## Session Continuity

Last session: 2026-02-19
Stopped at: Completed Phase 16 Plan 01 — Linear wizard UI and member alias backend
Resume file: None

## Next Step

**Action:** Execute Phase 16 Plan 02 — Pipeline Builder Linear Step + Re-sync UI
**Context:** 16-01 complete. 16-02 (wave 2) covers pipeline builder Linear step and re-sync UI (LINEAR-05, LINEAR-08).
