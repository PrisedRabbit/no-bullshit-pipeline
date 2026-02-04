---
name: hltm-impl-code-review
description: Code review for ONE story
version: 6.1.0
model: opus
context: fork
agent: general-purpose
---

# Code Review
**VERY IMPORTANT: NO ASSUMPTIONS. READ ALL REFERENCES BEFORE ACTING.**

**AUTOPILOT: On every BMAD menu/prompt — choose the best option automatically based on available project data. Ask user ONLY if a decision is truly impossible to make.**

1. `/bmad-bmm-code-review`

Find 3-10 issues. NEVER say "looks good".

**Logic:**
- Review thoroughly against `rules.md` and Blueprint patterns.
- **Always find issues.** No code is perfect.
- **If minor issues only:** Verdict `pass` (with warnings). Still list them.
- **If blocking issues exist:** Verdict `fail`. List them clearly.

Return: `{"verdict": "pass|fail", "issues": [...]}`
