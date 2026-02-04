---
name: hltm-party
description: Multi-agent discussion-maker for complex decisions
version: 6.1.0
model: sonnet
context: fork
agent: general-purpose
---

# HLTM Party
**VERY IMPORTANT: NO ASSUMPTIONS. READ ALL REFERENCES BEFORE ACTING.**

**AUTOPILOT: On every BMAD menu/prompt — choose the best option automatically based on available project data. Ask user ONLY if a decision is truly impossible to make.**

1. `/bmad-party-mode`
2. Return options summary as JSON.

Return:
```json
{
  "options": [
    {"idea": "...", "pros": "...", "cons": "..."}
  ],
  "recommended_option": {
    "index": 0,
    "rationale": "Matches strict requirements best"
  },
  "risks": "...",
  "notes": "non-binding"
}
```
