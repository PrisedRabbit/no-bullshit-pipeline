# Stage: develop

## Procedure (follow exactly, no skipping)

1. `bd ready -n 1` — pick the task.
2. `bd show <id>` — read FULL description, dependencies, comments, labels. If there are review failure comments — read them carefully, they tell you exactly what to fix.
3. **Check labels** — if any label signals the task needs human input (e.g. `needs-brainstorm`, `needs-design`, `needs-decision`, `wontfix`, `on-hold`, or similar) — do NOT work on it. `bd update <id> --status=blocked`, add a comment citing the label, then `<loop:stage>develop</loop:stage>` to pick the next task. Use your judgement — if a label means "not ready for code", skip it.
4. `bd update <id> --status=in_progress` — claim the task BEFORE writing code.
5. **Branch**: create a feature branch if not already on one: `git checkout -b feat/<short-task-name>`. If a branch for this task already exists, switch to it.
6. `git diff` — check if code changes for this task already exist (previous round may have crashed after implementing but before review).
7. If changes already exist and match the task description — do a quick sanity check, then skip to step 13.
8. Read ALL relevant source files. Understand the context. Trace the call chain. Check imports, types, existing patterns.
9. For external libraries/APIs — always use **Context7** or **WebFetch** docs. NEVER guess function signatures, return types, config formats, or API shapes.
10. Implement the task. ONE task. ONE round. No "while I'm here" improvements.
11. After implementation — verify your own work: does it compile? does it match the task description? did you miss anything from the comments?
12. **Commit**: `git add` all files you created/changed for this task, then `git commit --author="hltm-loop <hltm-loop@local>" -m "<task-id>: <short summary>"`. This isolates your work from other tasks.
13. Emit `<loop:stage>review:<id></loop:stage>` — every dev job goes to review. No exceptions. Pass the task ID.
14. For epic decomposition — create sub-tasks, skip review. No code written = no review needed.

## Rules

- For UI and UX tasks, always use `ui-ux-pro-max-skill` skill
- **Scope lock** — touch ONLY files relevant to the task. Zero tolerance.
- **Git workflow** — you MUST `git add` and `git commit` your changes before sending to review. Work in a feature branch (`feat/<name>`). Never push. Never touch `main` or `dev`.
- **No new tasks** — unless the task explicitly requires decomposition.
- **No refactoring** — unless the task is specifically about refactoring.
- **No dependency changes** — unless the task requires it. Don't upgrade, add, or remove packages on your own.
- **Never call `bd close`** — your task is to develop and check your work.
- **If stuck on a task** — `bd update <id> --status=blocked`, add a comment explaining why, then `<loop:stage>develop</loop:stage>` to pick the next task. Do NOT guess your way through.
- **If infrastructure is broken** (bd/git unavailable, prompt files missing) — `<loop:failed>reason</loop:failed>`. This kills the entire loop. Use ONLY for fatal errors.

## Anti-Patterns (NEVER do these)

- Implementing a task without reading the existing code first
- Guessing an API shape instead of looking it up
- "Fixing" code that isn't part of the task
- Adding error handling "just in case" for impossible scenarios
- Creating abstractions for one-time operations
- Adding comments to code you didn't write
- Changing formatting/style of untouched code
