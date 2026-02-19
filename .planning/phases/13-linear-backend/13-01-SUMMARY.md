---
phase: 13-linear-backend
plan: 01
status: completed
started: 2026-02-19
completed: 2026-02-19
---

## What Was Done

Created the Linear integration backend module following the established Notion connector pattern.

### Files Created/Modified

1. **`src-tauri/src/integrations/linear.rs`** (new) — Complete Linear integration module:
   - 6 types: `LinearIntegrationProfile`, `LinearWorkflowState`, `LinearLabel`, `LinearMember`, `LinearPriority`, `LinearTeamInfo`
   - Credential helpers delegating to shared `save_token`/`get_token`/`delete_token` with `linear:` prefix
   - Profile I/O: save/load/delete with 0o600 permissions at `~/.nbp/integrations/linear-{id}.json`
   - GraphQL client: direct reqwest POST to `https://api.linear.app/graphql` (no SDK dependency)
   - 6 Tauri commands: `add_linear_integration`, `test_linear_integration`, `remove_linear_integration`, `list_linear_teams`, `sync_linear_schema`, `list_linear_profiles`

2. **`src-tauri/src/integrations/mod.rs`** — Added `pub mod linear;`

3. **`src-tauri/src/lib.rs`** — Registered all 6 Linear commands in invoke_handler

### Key Decisions

- **No Linear SDK crate** — Used raw reqwest + GraphQL queries. Linear's API is simple enough and this avoids a new dependency.
- **3 separate GraphQL queries** for schema sync (states, labels, members) rather than one combined query — keeps each query simple with specific error messages.
- **Hardcoded priorities** (0-4: No priority, Urgent, High, Medium, Low) — Linear's priority levels are fixed.
- **Authorization header without "Bearer" prefix** — Linear API keys use raw token format.

### Verification

All plan requirements verified:
- All types derive Serialize, Deserialize, Clone, Debug
- Credential storage uses Keychain (release) / `.dev-credentials.json` (debug) via shared helpers
- Profile persisted with 0o600 permissions
- Idempotent removal (missing token/profile treated as success)
- No new crate dependencies added
