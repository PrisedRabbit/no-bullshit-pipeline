# Brief Enhancement (Phase 0)

## Check

If `input/brief.md` has all sections (App, Problem, Users, MVP Features, Main Flow, Data):
→ **SKIP** → Continue to Phase 1

If brief is raw idea:
→ **Enhance** → Stop → Recommend restart

## Enhance

```
Agent: _bmad/bmm/agents/analyst.md
Prompt: |
  Enhance input/brief.md with smart questions (max 4).
  Save to input/brief.md.
  NEVER ask tech questions.
```

## After Enhancement

```
✅ Brief enhanced!

Saved: input/brief.md

⚠️ Restart chat for context efficiency.
Then: /hltm-autopilot
```

**STOP** — don't continue in same session.
