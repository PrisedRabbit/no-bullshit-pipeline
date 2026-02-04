---
name: hltm-brief
description: Enhance raw idea into complete HLTM-ready brief
version: 6.1.0
model: sonnet
context: fork
agent: general-purpose
---

# Brief
**Goal:** Enhance raw brief → complete HLTM-ready brief with blueprint.

## Logic

1. **Detect:** Brownfield vs Greenfield (see `references/detection.md`)

2. **Execute:**
   - **GREENFIELD:** Enhance brief → select blueprint
   - **BROWNFIELD:** Read PRD/Arch → summarize stack → match blueprint

3. **Output:**
   - Write `input/brief-enhanced.md` (NEVER modify `input/brief.md`)
   - Return: `{"status": "done", "blueprint": "web-app|desktop-app|custom"}`

## References

| Reference | Description |
| --- | --- |
| [detection.md](./references/detection.md) | Brownfield detection logic |
| [rules.md](../hltm-autopilot/references/rules.md) | Core rules |
| [structure.md](../hltm-autopilot/references/structure.md) | Project structure |
