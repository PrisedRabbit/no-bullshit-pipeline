---
name: hltm-readiness
description: Gate check before implementation
version: 3.0.0
model: sonnet
context: fork
agent: general-purpose
---

# Readiness Gate

Binary gate. PASS or FAIL. No reasoning.

Check: `docs/prd.md`, `docs/ux-design.md`, `docs/architecture.md`, `docs/epics/*.md` exist.

Return:
```json
{"missing": [], "conflicts": [], "verdict": "PASS|FAIL"}
```

FSM:
- PASS → implementation
- FAIL → solutioning

## References

| Reference | Description |
|-----------|-------------|
| [checks.md](./references/checks.md) | Required files + output format |
| [review.md](./references/review.md) | Drift detection rules |
