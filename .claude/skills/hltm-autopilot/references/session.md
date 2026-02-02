# Session State

## File

```
.hltm/session.yaml
```

## Structure

```yaml
phase: implementation
epic: epic-01-auth
story: 1-2-refresh-token
step: hltm-testing-run
attempt: 2
recovery_used: false
```

No timestamps. No messages. No reasons.

## Fields

| Field | Values |
|-------|--------|
| phase | idle, planning, solutioning, readiness, implementation, retro |
| epic | epic file name |
| story | story key |
| step | current workflow step |
| attempt | retry count (≥1) |
| recovery_used | boolean (reset on epic change only) |

## Rules

1. **Overwrite entirely** — no append
2. **Read before EVERY action**
3. **Update after EVERY action**
4. **Corrupted → FAIL FAST**

## Driver Pattern

```
Agent returns JSON → Driver validates → Driver writes YAML
```

Agent NEVER writes YAML directly.
