# Update Analysis

## Read First

1. `input/update.md` — the request
2. `docs/prd.md` — backlog (pending requirements)
3. `docs/architecture.md` — tech decisions
4. `sprint-status.yaml` — current project state

---

## Determine Type

| Type | Criteria |
|------|----------|
| FIX | Bug, typo, <50 lines, single file |
| FEATURE | New page/component, extends existing |
| MAJOR | Data model, new integration, architecture change |

---

## FIX

```
1. Add fix to sprint-status.yaml
2. Agent: dev.md → implement
3. Agent: dev.md → code-review
4. Mark completed in sprint-status.yaml
5. Archive update.md
```

---

## FEATURE / MAJOR

```
Agent: pm.md         → add to PRD backlog
Agent: architect.md  → review/update architecture
Agent: pm.md         → create stories (new only)
Agent: sm.md         → update sprint-status.yaml
Agent: sm.md + dev.md → implement loop with review
Agent: analyst.md    → update project-context.md
CLEANUP              → remove completed from PRD, epics, sprint-status
```

---

## Key Principles

1. **PRD = backlog** — only pending requirements
2. **sprint-status.yaml = state** — tracks all work
3. **Every change tracked** — even FIX goes through sprint-status
4. **Cleanup after epic** — keep docs lean
