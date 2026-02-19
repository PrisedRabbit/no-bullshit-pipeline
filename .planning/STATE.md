# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-19)

**Core value:** Zero post-recording work — select pipeline, record, stop, everything happens automatically.
**Current focus:** v1.2 Connector Expansion

## Current Position

Phase: 14 — Linear Delivery (COMPLETE)
Plan: 14-01 done (1 plan, 1 wave, 2 tasks — commit d413ca0)
Status: Phase 14 complete, Phase 15 needs planning
Last activity: 2026-02-19 — Phase 14 executed (Linear connector + pipeline engine integration)

Progress: [████░░░░░░] 40% (2/5 phases)

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

### Pending Todos

None.

### Blockers/Concerns

- [Carried from v1]: cargo check not available in execution environment — all code verified structurally, compilation deferred to first cargo tauri dev run

## Session Continuity

Last session: 2026-02-19
Stopped at: Phase 14 complete
Resume file: None

## Next Step

**Action:** Plan Phase 15 — Linear Pipeline Integration
**Context:** Phase 15 adds prompt augmentation for Linear schema format specs. When an LLM step immediately precedes a Linear step, the prompt should be augmented with Linear field format instructions (priority values, label names, status options, member names). Extends the existing `build_augmented_prompt` function pattern used for Notion. See ROADMAP.md Phase 15 details.
