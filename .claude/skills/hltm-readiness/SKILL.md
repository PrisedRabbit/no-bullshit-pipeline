---
name: hltm-readiness
description: Gate check before implementation
version: 6.1.0
model: sonnet
context: fork
agent: general-purpose
---

# Readiness Gate
**VERY IMPORTANT: NO ASSUMPTIONS. READ ALL REFERENCES BEFORE ACTING.**

**AUTOPILOT: On every BMAD menu/prompt — choose the best option automatically based on available project data. Ask user ONLY if a decision is truly impossible to make.**

1. `/bmad-bmm-check-implementation-readiness`
2. If FAIL → write `readiness-report.md` to `{planning_artifacts}/` with issues found.
   (Solutioning reads this on re-entry to fix reported issues.)

Return: `{"verdict": "PASS|FAIL", "missing": [...]}`
