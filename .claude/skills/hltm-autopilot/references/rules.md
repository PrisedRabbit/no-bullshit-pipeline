# Rules

## Architecture Rules (v3.0)

| Rule | Description |
|------|-------------|
| **Thinking/writing → _atomic + fork** | Any step that thinks or writes code = `hltm-autopilot/_atomic/*` with `context: fork` |
| **Routing → dispatcher, no fork** | Any routing/orchestration = dispatcher skill without fork |
| **Validation error → STOP** | Any validation error = STOP, report to user. No auto-fix. |

---

## Execution Rules

1. Ask ONLY for missing brief requirements
2. NEVER ask during execution phases
3. ALWAYS use blueprint for tech decisions
4. ALWAYS run tests before declaring done
5. If stuck → decide yourself, document decision
6. If tests fail → fix & retry (max 3x, then report)

---

## Model Assignment

| Type | Model |
|------|-------|
| Autopilot | opus |
| Dispatchers | sonnet |
| Atomic (hltm-autopilot/_atomic/*) | opus |

---

## Testing Mode

From blueprint:

```yaml
testing:
  mode: balanced  # strict | balanced | minimal
```

| Mode | ATDD | Test Review | When |
|------|------|-------------|------|
| `strict` | ✅ Before DEV | ✅ After DEV | Production, critical |
| `balanced` | ❌ | ✅ After DEV | MVP, standard (DEFAULT) |
| `minimal` | ❌ | ❌ | Prototype, POC |

---

## Status Flow

```
backlog → ready-for-dev → in-progress → review → done
          create-story    dev-story     dev-story  code-review
```

---

## Output Locations

| Type | Location |
|------|----------|
| Docs | `docs/` |
| State | `.hltm/session.yaml`, `sprint-status.yaml` |
| Code | `src/` |
| Tests | `tests/` |
