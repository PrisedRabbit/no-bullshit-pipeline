# Cleanup Gate

ALL must pass before deletion.

## Checks

| # | Check | What |
|---|-------|------|
| 1 | Architecture Sync | src/ matches preserved architecture in `project_knowledge` (resolved from `_bmad/bmm/config.yaml`) |
| 2 | Invariant Check | No orphan rules in stories only |
| 3 | ADR Check | Irreversible decisions documented |

---

## Gate Logic

```
All pass → CLEANUP ALLOWED
Any fail → STOP, fix first
```

---

## On Failure

```
CLEANUP GATE FAILED

Issues:
1. Architecture out of sync: [reason]
2. Orphan rules: [list]
3. Missing ADRs: [list]

Fix these before cleanup.
```

No auto-skip.
