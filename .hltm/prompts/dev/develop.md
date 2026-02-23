# Stage: develop

## Procedure

1. `bd ready -n 1` — pick task.
2. `<loop:update>develop: <id></loop:update>`
3. `bd show <id>` — read description, dependencies, comments. Review failure comments = what to fix.
3. **Labels** — label signals human input needed (`needs-brainstorm`, `on-hold`, etc.) → `bd update <id> --status=blocked`, comment, `<loop:stage>develop</loop:stage>`.
4. `bd update <id> --status=in_progress`
5. `git checkout -b feat/<task-id>` (exists → `git checkout feat/<task-id>`)
6. `git diff` — changes already exist from crashed round? Match task → sanity check → skip to 13.
7. Read ALL relevant source files. Trace call chain. Check imports, types, patterns.
8. External APIs → **Context7** or **WebFetch**. Never guess signatures.
9. Implement. ONE task. ALL requirements. Handle edge cases.
10. **Self-review** — reviewer checks: correctness, completeness, patterns, scope, bugs, security, dead code, deps. Would you pass this? Fix now. Every fix round = wasted round.
11. `git add` + `git commit --author="hltm-loop <hltm-loop@local>" -m "<task-id>: <summary>"`
12. `<loop:stage>review:<id></loop:stage>`
13. Epic decomposition → create sub-tasks, no review needed.

## Rules

- UI/UX tasks → use `ui-ux-pro-max-skill`
- Scope lock — ONLY task-relevant files
- No new tasks unless decomposition required
- No refactoring unless task is about refactoring
- No dependency changes unless task requires it
- Never `bd close` — develop and check, not close
- Stuck → `bd update <id> --status=blocked`, comment, `<loop:stage>develop</loop:stage>`
- Infra broken → `<loop:failed>reason</loop:failed>`

## Anti-Patterns

- Implementing without reading existing code
- Guessing API shapes
- "Fixing" code outside the task
- Error handling for impossible scenarios
- Abstractions for one-time operations
- Comments on code you didn't write
- Reformatting untouched code
