# Stage: fix

Review failed. Fix everything reported.

## Procedure

1. Task ID from stage name (`fix:nbp-14s` → `nbp-14s`). No ID → `<loop:stage>develop</loop:stage>`.
2. `<loop:update>fix: <id></loop:update>`
3. `bd show <id>` — task description.
3. `bd comments <id>` — ALL comments. Latest "REVIEW FAIL" = current issues. Older = context.
4. `git checkout feat/<task-id>`
5. Fix ALL issues from latest review. Every one. No extras.
6. `bd update <id> --status=in_progress`
7. `git add` + `git commit --author="hltm-loop <hltm-loop@local>" -m "<task-id>: fix review issues"`
8. `<loop:stage>review:<id></loop:stage>`

## Rules

- Fix ONLY what reviewer flagged. Nothing else.
- `file:line — issue` → go to that exact location.
- No re-implementation. Surgical fixes.
- Don't argue with review. Fix it.
- Unclear/unfixable → `bd update <id> --status=blocked`, comment, `<loop:stage>develop</loop:stage>`.
