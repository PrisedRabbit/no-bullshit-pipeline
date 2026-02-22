# Stage: review

## Procedure (follow exactly)

1. The task ID is in the stage name (e.g. `Current stage: review:nbp-14s` → task is `nbp-14s`). If no ID, run `bd list --status=in_progress -n 1`.
2. `bd show <id>` — read the full task description and any comments.
3. **Read the full branch diff**: find where the feature branch diverged and diff the whole thing: `git log --oneline main..HEAD` to see all commits, then `git diff main...HEAD` to see the total diff for this task. Do NOT use bare `git diff` or `HEAD~1` — review ALL changes on the branch.
4. If no commits for this task exist and `git diff` is also empty — the task was purely organizational (epic decomposition, task creation). PASS it and move on.
5. Verify against these criteria:

| Check            | Question                                                   |
| ---------------- | ---------------------------------------------------------- |
| **Correctness**  | Does the implementation actually do what the task asks?    |
| **Completeness** | Are all requirements from the task description met?        |
| **Patterns**     | Does it follow existing codebase conventions?              |
| **Scope**        | Were ONLY task-relevant files changed in the commit?       |
| **Bugs**         | Edge cases? Off-by-one? Null/undefined? Race conditions?   |
| **Security**     | Injection? XSS? Hardcoded secrets? OWASP top 10?           |
| **Dead code**    | Any TODOs, commented-out code, unused imports?             |
| **Dependencies** | Were any packages added/changed without task requiring it? |

## What is NOT a failure

- **Untracked files** (`??` in git status) — not your problem, not a bug.
- **Changes from other tasks** in the working tree — ignore them, review only the commit(s) for THIS task.
- **Missing `git add`/`git commit`** — that's workflow, not code quality.

## PASS — all checks green

Write the review comment to a temp file to avoid shell escaping issues:
1. Write your review summary to `/tmp/review_comment.txt`
2. `bd comments add <id> -f /tmp/review_comment.txt`
3. `bd close <id>`
4. If `bd ready` has more tasks → `<loop:stage>develop</loop:stage>`
5. If no tasks remain → `<loop:stage>learning</loop:stage>`

## FAIL — any check fails

**Max 3 review rounds.** Check `bd comments list <id>` — count prior "REVIEW FAIL" comments. If already 2:
1. Write final failure summary to `/tmp/review_comment.txt`
2. `bd comments add <id> -f /tmp/review_comment.txt`
3. `bd update <id> --status=blocked`
4. If `bd ready` has more tasks → `<loop:stage>develop</loop:stage>` (skip blocked task, move on)
5. If no tasks remain → `<loop:stage>learning</loop:stage>`

Write the failure details to a temp file to avoid shell escaping issues:
1. Write a **comprehensive** failure report to `/tmp/review_comment.txt` — list ALL issues found (every file:line, every bug, every problem). The fix agent will address everything in one shot. Do not drip-feed issues across rounds.
2. `bd comments add <id> -f /tmp/review_comment.txt`
3. `bd update <id> --status=open`
4. `<loop:stage>fix:<id></loop:stage>` — pass the task ID so fix knows what to work on.

## Anti-Patterns

- Passing code that "mostly works" — either it's correct or it's not
- Vague failure comments like "needs improvement" — say exactly what's wrong
- Failing code for style preferences that don't match codebase conventions
- Failing for untracked files, git status noise, or other tasks' changes
- Reviewing bare `git diff` instead of the task's commit diff
