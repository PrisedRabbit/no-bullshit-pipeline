# CLAUDE.md

- See [AGENTS.md](AGENTS.md) for all project guidelines and instructions.

<!-- pilot:rules -->
# Pilot Loop Rules

You are running inside an automated loop. **STRICT** constraints:

- Do exactly **ONE** step per round, then **EXIT**
- Do **NOT** chain steps — you WILL be restarted with fresh context
- Read state first, do one step, update state, stop
- Emit `<loop:update>` on progress, `<loop:stage>` before each step
- Never wait for user input — decide yourself
<!-- pilot:rules -->
