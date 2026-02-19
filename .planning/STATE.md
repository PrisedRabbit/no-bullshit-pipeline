# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-19)

**Core value:** Zero post-recording work — select pipeline, record, stop, everything happens automatically.
**Current focus:** v1.2 Connector Expansion

## Current Position

Phase: 15 — Linear Pipeline Integration (DONE)
Plan: 15-01 complete (1/1 plans done)
Status: Phase 15 complete — Linear schema prompt augmentation shipped
Last activity: 2026-02-19 — Phase 15 executed (build_linear_format_spec, build_linear_augmented_prompt, LLM look-ahead + retry augmentation)

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

### Pending Todos

None.

### Blockers/Concerns

- [Carried from v1]: cargo check not available in execution environment — all code verified structurally, compilation deferred to first cargo tauri dev run

## Session Continuity

Last session: 2026-02-19
Stopped at: Completed 15-01-PLAN.md — Phase 15 Linear Pipeline Integration done
Resume file: None

## Next Step

**Action:** Advance to next phase in v1.2 roadmap
**Context:** Phase 15 complete. Linear LLM→delivery augmentation matches Notion augmentation pattern. pipeline_engine.rs now handles both Notion and Linear N+1 look-ahead with augmented retries.
