# Stage: learning

## Procedure

1. `git diff HEAD~5 --stat` — what changed.
2. `git log --oneline -10` — recent commits.
3. Read changed files.
4. Read `snapshot-guide.md` (same directory).
5. Update `_hltm/snapshot/` per guide:
   - `project.md` — stack, structure changes
   - `architecture.md` — components, data flow changes
   - `conventions.md` — new/changed patterns
   - `infra.md` — infrastructure changes
   - `decisions.md` — new decisions (top, never edit old)
   - `lessons.md` — review failures, bugs, mistakes
   - Additive only — never delete existing content.
6. `<loop:done>summary</loop:done>`
