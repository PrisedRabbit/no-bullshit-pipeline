---
name: hltm-implementation
description: Implementation phase - story loop
version: 4.0.0
model: opus
context: fork
agent: general-purpose
---

# Implementation

**VERY IMPORTANT: NO ASSUMPTIONS. READ ALL REFERENCES BEFORE ACTING.**

Loop per story:
1. `/bmad-bmm-create-story`
2. `/bmad-bmm-dev-story`
3. `/bmad-tea-testarch-automate`
4. `bun test` — fail → fix → retry
5. `/bmad-bmm-code-review` — fail → fix → retry

Gates:
- test fail → fix → retry
- code-review fail → fix → test → retry

Return: `{"status": "done", "phase": "implementation"}`
