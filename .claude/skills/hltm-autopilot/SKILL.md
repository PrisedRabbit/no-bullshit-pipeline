---
name: hltm-autopilot
description: Main orchestrator - FSM
version: 3.0.0
model: opus
---

# Autopilot

Read `{PROJECT_ROOT}/.hltm/session.yaml` → call next phase → update session → loop.

## Triggers (idle)

Check:

- `{PROJECT_ROOT}/input/brief.md` exists → planning
- `{PROJECT_ROOT}/input/update.md` exists → planning
- neither → STOP

## FSM

```
idle → planning → solutioning → readiness → implementation → retro → idle
```

## Phases

- `/hltm-planning`
- `/hltm-solutioning`
- `/hltm-readiness`
- `/hltm-implementation`
- `/hltm-retro`
