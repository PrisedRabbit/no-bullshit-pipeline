# Stage: review

## Environment

- **Task tracker**: `bd`. Commands: `bd show <id>`, `bd comments <id>`, `bd comments add <id> "..."`, `bd close <id>`, `bd update <id> --status=...`. No `bd comments list`.
- **Branch**: `feat/<task-id>`. Checkout: `git checkout feat/<task-id>`.
- **Diff**: `git diff <base>...HEAD` or `git show HEAD`. Base branch is at the top of the prompt.

## Procedure

1. Task ID is in stage name (`review:nbp-14s` → `nbp-14s`). No ID → `bd list --status=in_progress -n 1`.
2. `<loop:update>review: <id></loop:update>`
3. `bd show <id>` — read task description and comments.
3. `git checkout feat/<task-id>`
4. Read diff: `git diff <base>...HEAD`
5. No changes → PASS (organizational task).
6. **Build** — run check command from `_hltm/snapshot/project.md` Toolchain table. Doesn't compile = FAIL.
7. **Lint** — run linter from snapshot. New lint errors = FAIL.
8. Verify:

| Check            | Question                                                |
| ---------------- | ------------------------------------------------------- |
| **Correctness**  | Does it do what the task asks?                          |
| **Completeness** | All requirements met?                                   |
| **Patterns**     | Follows codebase conventions?                           |
| **Scope**        | ONLY task-relevant files changed?                       |
| **Bugs**         | Edge cases? Off-by-one? Null? Race conditions?          |
| **Security**     | Injection? XSS? Hardcoded secrets?                      |
| **Dead code**    | TODOs, commented-out code, unused imports?              |
| **Dependencies** | Packages added/changed without task requiring it?       |

## Not a failure

- Untracked files (`??` in git status)
- Missing `git add`/`git commit`

## PASS

1. `bd comments add <id> "REVIEW PASS: <summary>"`
2. `bd close <id>`
3. `git checkout <base> && git merge feat/<task-id> --no-ff && git branch -d feat/<task-id>`
4. `bd ready` has tasks → `<loop:stage>develop</loop:stage>`
5. No tasks → `<loop:stage>learning</loop:stage>`

## FAIL

**Max 3 rounds.** Check `bd comments <id>` — count prior "REVIEW FAIL". Already 2:
1. `bd comments add <id> "BLOCKED after 3 failures. Branch: feat/<task-id>. Issues: <summary>"`
2. `bd update <id> --status=blocked`
3. Tasks left → `<loop:stage>develop</loop:stage>`, none → `<loop:stage>learning</loop:stage>`

Rounds 1-2:
1. `bd comments add <id> "REVIEW FAIL: <ALL issues, every file:line, every bug>"` — comprehensive, one shot.
2. `bd update <id> --status=open`
3. `<loop:stage>fix:<id></loop:stage>`

## Anti-Patterns

- Passing code that "mostly works"
- Vague comments like "needs improvement" — say exactly what's wrong
- Failing for style preferences that don't match codebase conventions
- Failing for untracked files or other tasks' changes
