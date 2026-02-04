# Implementation Routing

Router handles `phase: implementation` directly — no intermediary skill.

## Step Mapping

| State | Action |
|-------|--------|
| No `story` in state | Select next story (see Story Selection below) |
| `step: develop` | Call `/hltm-impl-develop` |
| `step: testing` | Call `/hltm-impl-testing` |
| `step: code-review` | Call `/hltm-impl-code-review` |

## Step Transitions

| Result | Current Step | Next State |
|--------|-------------|------------|
| **pass** | develop | `step: testing` |
| **pass** | testing | `step: code-review` |
| **pass** | code-review | Mark story DONE in `sprint-status.yaml` → select next story |
| **fail** | any | Increment `attempt` → apply Recovery |

## Story Selection

1. Read `sprint-status.yaml`
2. Stories already ordered by BMAD (dependencies + business value)
3. Pick first story NOT `done`
4. Write state: `{ story: "KEY", step: "develop", attempt: 1 }`

**Epic change:** When selected story has different epic than `session.epic` → reset `recovery_used: false`.

**All stories done** → transition to `retro`.

## Recovery

| Condition | Action |
|-----------|--------|
| `attempt < 5` | Retry same step (`attempt: N+1`) |
| `attempt >= 5` AND `!recovery_used` | Call `/hltm-party` → `/bmad-agent-bmm-pm` → reset `attempt: 1`, `recovery_used: true`, `step: develop` |
| `attempt >= 5` AND `recovery_used` | Set `blocked: true` → STOP (error E004) |
