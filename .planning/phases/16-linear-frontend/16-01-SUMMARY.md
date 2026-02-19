---
phase: 16-linear-frontend
plan: 01
subsystem: integrations-ui
tags: [linear, wizard, frontend, member-aliases, tauri]
dependency_graph:
  requires: [13-01-SUMMARY.md]
  provides: [Linear wizard UI, MemberAlias backend, connected Linear cards]
  affects: [src/integrations-settings.js, src/index.html, src-tauri/src/integrations/linear.rs]
tech_stack:
  added: []
  patterns: [Notion wizard state machine pattern, clone-replace button pattern]
key_files:
  created: []
  modified:
    - src-tauri/src/integrations/linear.rs
    - src-tauri/src/lib.rs
    - src/index.html
    - src/integrations-settings.js
decisions:
  - MemberAlias struct follows Notion PeopleMapping pattern exactly — consistent API for alias resolution
  - 4-step wizard (API key, team picker, schema, alias mapping) vs Notion's 5-step (extra share-instruction step omitted since Linear has no sharing step)
  - Linear card detail shows team_name (not database_name) — matches Linear's data model
metrics:
  duration_seconds: 175
  completed_date: 2026-02-19
  tasks_completed: 2
  tasks_total: 2
  files_modified: 4
---

# Phase 16 Plan 01: Linear Frontend Wizard Summary

**One-liner:** 4-step Linear wizard with MemberAlias backend — API key validation, team picker with schema sync, schema display (states/labels/members/priorities), and member alias mapping persisted via new Tauri command.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add member alias support to Linear backend | 964f40a | src-tauri/src/integrations/linear.rs, src-tauri/src/lib.rs |
| 2 | Linear wizard UI, connected cards, and available integration entry | b01298c | src/index.html, src/integrations-settings.js |

## What Was Built

### Backend (Task 1)
- `MemberAlias` struct added to `linear.rs` with `alias`, `member_id`, `display_name` fields
- `member_aliases: Vec<MemberAlias>` field added to `LinearIntegrationProfile` with `#[serde(default)]` for backward compatibility with existing profiles
- `sync_linear_schema` updated to preserve existing `member_aliases` on re-sync (combined with existing name preservation into single load)
- `add_linear_integration` updated to initialize `member_aliases: Vec::new()`
- `update_linear_member_aliases` Tauri command added — validates member IDs against profile's members list before saving
- Command registered in `lib.rs` invoke_handler

### Frontend (Task 2)
- `linear-wizard-modal` HTML added to `index.html` with 4-step wizard structure
- `linearProfiles` global and `loadLinearProfiles()` function added
- `loadAllIntegrations()` updated to load Linear profiles in parallel
- Linear connected cards added to `renderConnectedIntegrations()` with staleness warning (>7 days)
- Test and Remove button handlers for Linear cards
- Linear entry added to available integrations grid (`+ Add` card)
- Full 4-step wizard implementation:
  - **Step 0** — API key entry: calls `add_linear_integration`, stores returned ID
  - **Step 1** — Team picker: calls `list_linear_teams`, renders clickable team list, calls `sync_linear_schema` on Next
  - **Step 2** — Schema display: 4 mini-sections (workflow states, labels, members, priorities) with Re-sync button
  - **Step 3** — Member alias mapping: dynamic rows with alias input + member dropdown, calls `update_linear_member_aliases` on Finish
- Cancel button cleans up partial integrations via `remove_linear_integration`
- `replaceLinearNextBtn()` clone-replace pattern prevents listener stacking

## Deviations from Plan

None — plan executed exactly as written.

## Key Decisions Made

1. **Combined name + aliases load in sync_linear_schema** — single `load_linear_profile` call extracts both `existing_name` and `existing_aliases`, replacing the previous name-only load. Cleaner and avoids a second disk read.

2. **4-step wizard (vs Notion's 5-step)** — Omitted the "share instruction" step that Notion needs (sharing database with integration). Linear has no equivalent sharing step — API key grants direct team access.

3. **Schema display as 4 mini-sections** — Used separate tables per schema category (states/labels/members/priorities) rather than one flat table, since each category has a different column structure.

## Self-Check

### Files Exist
- [x] src-tauri/src/integrations/linear.rs — modified
- [x] src-tauri/src/lib.rs — modified
- [x] src/index.html — modified
- [x] src/integrations-settings.js — modified

### Commits Exist
- [x] 964f40a — feat(16-01): add MemberAlias struct and update_linear_member_aliases command
- [x] b01298c — feat(16-01): add Linear wizard UI, connected cards, and available integration entry

## Self-Check: PASSED
