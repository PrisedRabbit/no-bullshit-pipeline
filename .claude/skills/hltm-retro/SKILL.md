---
name: hltm-retro
description: Retro phase - lessons + cleanup
version: 4.0.0
model: opus
context: fork
agent: general-purpose
---

# Retro

**CRITICAL: `.hltm/session.yaml` → NEVER DELETE, only reset to `phase: idle`**

1. `/bmad-bmm-retrospective`
2. Run `./scripts/cleanup.sh`
3. Update `.hltm/session.yaml` → `phase: idle`

Return: `{"status": "done", "phase": "retro"}`
