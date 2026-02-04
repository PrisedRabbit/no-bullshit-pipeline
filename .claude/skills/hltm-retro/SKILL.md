---
name: hltm-retro
description: Retro phase - lessons + cleanup
version: 6.1.0
model: sonnet
context: fork
agent: general-purpose
---

# Retro
**AUTOPILOT: On every BMAD menu/prompt — choose best option automatically.**

## Steps

1. **Analyze:** Compare `input/brief.md` vs `sprint-status.yaml` → identify completed/unfinished.

2. **Retrospective:** Call `/bmad-bmm-retrospective` → generate changelog JSON (1-3 bullets per epic).

3. **Cleanup:** See `references/scenarios.md`:
   - **ALL DONE:** Archive & delete brief → clean slate
   - **PARTIAL:** Archive original → rewrite brief with unfinished only

4. **Return:**
```json
{
  "phase": "idle",
  "changelog": {
    "epics": [
      {"title": "Epic Name", "items": ["..."]}
    ]
  }
}
```

## References

| Reference | Description |
| --- | --- |
| [scenarios.md](./references/scenarios.md) | ALL DONE vs PARTIAL cleanup |
| [cleanup.md](./references/cleanup.md) | What to keep/delete |
| [changelog.md](./references/changelog.md) | Changelog format |
