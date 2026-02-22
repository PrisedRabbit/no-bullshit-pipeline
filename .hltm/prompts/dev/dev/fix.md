# Stage: fix

Review failed. Fix everything the reviewer reported. No excuses.

## Procedure

1. The task ID is in the stage name (e.g. `Current stage: fix:nbp-14s` → task is `nbp-14s`). If no ID → `<loop:stage>develop</loop:stage>`. Do NOT guess.
2. `bd show <id>` — read the task description.
3. `bd comments <id>` — read ALL review comments, understand the full history. The latest "REVIEW FAIL" comment tells you what's wrong NOW, but older comments give context on what was already attempted.
4. Fix ALL issues listed in the latest review comment. Every single one. Zero issues left behind. No extras, no "while I'm here".
5. `bd update <id> --status=in_progress`
6. **Commit the fix**: `git add` changed files, then `git commit --author="hltm-loop <hltm-loop@local>" -m "<task-id>: fix review issues"`.
7. Emit `<loop:stage>review:<id></loop:stage>` — always pass the task ID.

## Rules

- Fix ONLY what the reviewer flagged. Do not touch anything else.
- If the reviewer said "file:line — issue" — go to that exact file and line.
- Do NOT re-implement the whole task. Surgical fixes only.
- Do NOT argue with the review. Fix it.
- Do NOT explain why something is hard. Do NOT describe what you can't do. Fix it.
- If the review comment is unclear or unfixable — `bd update <id> --status=blocked`, add a comment explaining what's unclear, then `<loop:stage>develop</loop:stage>`.
