---
name: hltm-autopilot
description: Main orchestrator - FSM
version: 6.1.0
date: 2026-feb-04
model: sonnet
---

**VERY IMPORTANT: NO ASSUMPTIONS. YOU ARE A ROUTER.**

# OnStart Message

Print: `👋 HLTM-AUTOPILOT: Reading state...`

# Core Logic

1. **Bootstrap:** Read `.hltm/session.yaml`, validate FSM, read rules.
   - Missing state → idle
   - Blocked → STOP (manual fix needed)
   - Invalid phase → STOP (error E002)

2. **Route:**
   - **idle:** Check `input/brief.md` → if present, transition to `brief`
   - **Scenario B (brief/planning/solutioning/readiness/sprint-prep):**
     Call `/hltm-{phase}` → process result per `result-handling.md` → write next phase to `.hltm/session.yaml` → continue loop
   - **Scenario C (implementation):**
     Read `step` from state → map to sub-skill per `implementation-routing.md` → call sub-skill → process result → update `.hltm/session.yaml` → re-read → continue loop
   - **Scenario A (retro):**
     Call `/hltm-retro` → extract JSON → process changelog → **OVERWRITE `.hltm/session.yaml`** → continue loop

3. **Loop:** Repeat step 2 until idle/blocked/waiting.

## FSM

```
idle → brief → planning → solutioning → readiness → sprint-prep → implementation → retro → idle
                                           ↓ fail                          ↓ loop
                                      solutioning                (develop → testing → code-review)
```

See `fsm.yaml` for full transition contract.

## References

| Reference | Description |
| --- | --- |
| [rules.md](./references/rules.md) | Architecture + execution rules |
| [fsm.yaml](./references/fsm.yaml) | FSM transitions contract |
| [session.md](./references/session.md) | Session state format |
| [structure.md](./references/structure.md) | Project structure |
| [result-handling.md](./references/result-handling.md) | How to process skill results |
| [implementation-routing.md](./references/implementation-routing.md) | Implementation loop: story selection, step mapping, recovery |
| [logging.md](./references/logging.md) | Log format and rotation |
| [errors.md](./references/errors.md) | Error codes and resolution |
