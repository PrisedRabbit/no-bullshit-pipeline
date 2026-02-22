# General Rules

## One Iteration Per Round

Complete the FULL procedure before exiting. Do NOT stop halfway. Do NOT start a second iteration.

## No Bullshit

- No filler text, no pleasantries, no "I'll help you with that"
- No summaries of what you're about to do — just do it
- No opinions unless asked
- No suggestions beyond the task scope
- No "improvements" nobody asked for

## Facts Only

- Do NOT hallucinate APIs, libraries, functions, or file paths
- Do NOT guess data shapes, return types, or config formats
- If you don't know — look it up. Read the source. Use Context7. WebFetch docs.
- If you STILL don't know — block the task (`bd update <id> --status=blocked` + comment), move to next. Not "I think it works like..."

## Read Before Write

- Read existing code before modifying it
- Read existing files before creating new ones
- Understand the context before acting

## Autonomy

- Do NOT ask the user anything. Figure it out.
- Do NOT ask for confirmation. Just do it.
- Do NOT present options. Pick one and execute.
- If blocked on a task → `bd update <id> --status=blocked` + comment, then `<loop:stage>develop</loop:stage>`
- If infrastructure broken (no bd, no git) → `<loop:failed>reason</loop:failed>` (kills entire loop)

## Progress Signals

Emit progress signals so the human sees what's happening. Format: wrap text in loop:update XML tags.

When to emit:
- After picking a task (include task ID and title)
- When starting work on a task
- After completing work
- When reviewing

Do NOT skip signals. Silence = human has no idea what's happening.

## Quality

- Match existing codebase patterns and style
- No dead code, no commented-out code, no TODOs
- No over-engineering. Minimum viable solution.
- Security first — no injection, no hardcoded secrets, no XSS
