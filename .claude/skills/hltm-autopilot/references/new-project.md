# New Project Flow

## Execution

**Use TodoWrite to track.**

| # | Task | Blocked By |
|---|------|------------|
| 1 | /hltm-planning | - |
| 2 | /hltm-solutioning | #1 |
| 3 | /hltm-readiness | #2 |
| 4 | /hltm-implementation | #3 |
| 5 | /hltm-retro | #4 (all epics done) |

---

## Source of Truth

```
sprint-status.yaml = PROJECT STATE
docs/prd.md = BACKLOG
docs/epics/ = WORK BREAKDOWN
```

---

## Phase 1: Planning

```
/hltm-planning
```

Output: `docs/prd.md`, `docs/ux-design.md`

---

## Phase 2: Solutioning

```
/hltm-solutioning
```

Output: `docs/architecture.md`, `docs/epics/`

---

## Phase 3: Implementation

```
/hltm-implementation
```

Runs story loop until all epics done.

---

## Phase 4: Retro

```
/hltm-retro
```

Then archive brief:

```bash
mkdir -p input/done && mv input/brief.md input/done/brief-$(date +%Y%m%d-%H%M).md
```
