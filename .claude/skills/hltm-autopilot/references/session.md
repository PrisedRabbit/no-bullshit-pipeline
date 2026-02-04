# Session State

## File

```
.hltm/session.yaml
```

## Structure

```yaml
phase: implementation
blueprint: web-app
epic: epic-01-auth
story: 1-2-refresh-token
step: develop
attempt: 2
recovery_used: false
blocked: false
waiting_for_answers: false
```

No timestamps. No messages. No reasons.

## Fields

| Field               | Values                                                                                                    |
| ------------------- | --------------------------------------------------------------------------------------------------------- |
| phase               | idle, brief, planning, solutioning, readiness, sprint-prep, implementation, retro                      |
| blueprint           | web-app, desktop-app, custom (cached from brief)                                                          |
| epic                | epic file name                                                                                            |
| story               | story key                                                                                                 |
| step                | develop, testing, code-review                                                                             |
| attempt             | retry count (≥1)                                                                                          |
| recovery_used       | boolean (reset on epic change only)                                                                       |
| blocked             | boolean (true if critical error occurred). **User must remove manually after fixing issue.**              |
| waiting_for_answers | boolean (true if input/questions.md needs user attention). **User must remove manually after answering.** |

## Rules

1. **Overwrite entirely** — no append
2. **Read before EVERY action**
3. **Update after EVERY action**
4. **Corrupted → FAIL FAST**

## Write Pattern

```
Skill returns JSON → Autopilot validates → Autopilot writes YAML
```

Skills NEVER write `.hltm/session.yaml` directly.
