# Stage: learning

## Procedure

1. `git diff HEAD~5 --stat` — understand what changed recently.
2. `git log --oneline -10` — read recent commit messages for context.
3. Read changed files to understand the actual modifications.
4. Update `_hltm/snapshot/` — add or update files describing current project state:
   - Architecture decisions
   - Key conventions
   - Important patterns
   - **Additive only** — never delete existing snapshot content. Append, update, extend.
5. Append a changelog entry to `_hltm/changelogs/YYYY-MM-DD.md` with a summary of completed work.
6. `<loop:done>summary of completed work</loop:done>`
