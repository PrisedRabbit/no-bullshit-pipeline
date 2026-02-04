# Rules

## Architecture Rules

| Rule                        | Description                                              |
| --------------------------- | -------------------------------------------------------- |
| **Phase skills → fork**     | Every phase skill runs with `context: fork`              |
| **Orchestrators → no fork** | Autopilot — no fork (lives throughout session)           |
| **Validation error → STOP** | Any validation error = STOP, report to user. No auto-fix |

---

## Execution Rules

1. **GIT: NEVER commit unless explicitly requested by the user.**
2. Ask ONLY for missing brief requirements
3. NEVER ask during execution phases
4. ALWAYS use blueprint for tech decisions
5. ALWAYS run tests before declaring done
6. If stuck → decide yourself, document decision

---

## Model Assignment

| Type                              | Model  |
| --------------------------------- | ------ |
| Autopilot                         | sonnet |
| Planning (PRD, UX)                | opus   |
| Solutioning (architecture, epics) | opus   |
| Code review                       | opus   |
| impl-develop (code writing)       | opus   |
| Everything else                   | sonnet |

---

## Testing Mode

From blueprint:

```yaml
testing:
  mode: balanced # strict | balanced | minimal
```

| Mode       | ATDD          | Test Review  | When                    |
| ---------- | ------------- | ------------ | ----------------------- |
| `strict`   | ✅ Before DEV | ✅ After DEV | Production, critical    |
| `balanced` | ❌            | ✅ After DEV | MVP, standard (DEFAULT) |
| `minimal`  | ❌            | ❌           | Prototype, POC          |

---

## Status Flow

```
backlog → ready-for-dev → in-progress → review → done
          create-story    dev-story     dev-story  code-review
```

---

## Brownfield Pre-Check

```
Previous work exists (`{implementation_artifacts}/sprint-status.yaml` has in-progress; resolve via `_bmad/bmm/config.yaml`)?
  ├── NO  → Continue
  └── YES → ALL done?
            ├── YES → /hltm-retro, then continue
            └── NO  → ASK USER (resume / force retro / force delete)
```

---

## Output Locations

| Type                          | Location                                                          |
| ----------------------------- | ----------------------------------------------------------------- |
| BMAD planning artifacts       | Resolve from `_bmad/bmm/config.yaml` → `planning_artifacts`       |
| BMAD implementation artifacts | Resolve from `_bmad/bmm/config.yaml` → `implementation_artifacts` |
| Knowledge docs (if used)      | Resolve from `_bmad/bmm/config.yaml` → `project_knowledge`        |
| HLTM state                    | `.hltm/session.yaml`                                              |
| Code                          | `src/`                                                            |
| Tests                         | project-specific (often `tests/`)                                 |
