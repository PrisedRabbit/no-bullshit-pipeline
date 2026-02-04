---
name: hltm-solutioning
description: Solutioning phase - Architecture + Epics
version: 6.1.0
model: opus
context: fork
agent: general-purpose
---

# Solutioning
**Goal:** Architecture + Epics. **Strict scope** (see `../hltm-autopilot/references/scope.md`).

**Note:** If `readiness-report.md` exists, READ IT first and fix issues.

## Steps

1. `/bmad-bmm-create-architecture`
2. `/bmad-bmm-create-epics-and-stories`
3. `/hltm-party` — review
4. `/bmad-agent-bmm-pm` — apply changes

Return: `{"status": "done"}`
