# Recovery PARTY

Emergency mode. Strictly by trigger.

## Trigger

```
attempt >= 3 AND same_step_repeat >= 2 AND no_progress
```

## Tech-Repair First

```
Syntax/Import/Type/Build error → tech-repair (2 attempts)
Logic/design issue → Recovery PARTY
```

**PARTY is expensive.** Don't waste on missing imports.

## Rules

Recovery PARTY does **NOT**:
- Change phase
- Skip FSM
- Edit project-context
- Mark story done

## One Per Epic

```
recovery_used: true → second PARTY FORBIDDEN
```

Reset ONLY on epic change.

## Escalation

Still failing after recovery → **STOP. Report to user.**
