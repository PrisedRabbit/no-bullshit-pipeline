# Development Methodology

## Stage → File

| Stage         | Read                            |
| ------------- | ------------------------------- |
| `develop`     | `.hltm/prompts/dev/develop.md`  |
| `review:<id>` | `.hltm/prompts/dev/review.md`   |
| `fix:<id>`    | `.hltm/prompts/dev/fix.md`      |
| `learning`    | `.hltm/prompts/dev/learning.md` |

Default (no stage or unknown) → `develop.md`

Read the file. Follow it exactly.

## Project Context

Read `_hltm/snapshot/` if it exists:
- `project.md` — stack, build commands, structure
- `architecture.md` — components, data flow
- `conventions.md` — naming, patterns
- `infra.md` — environments, CI/CD, config

## Git

- Loop resets to base branch before every round.
- Base branch name is at the top of the prompt.
- Feature branches: `git checkout -b feat/<task-id>`
- Never modify files on base branch.

## Task Tracker (`bd`)

- `bd ready` — available work
- `bd show <id>` — task details
- `bd update <id> --status=in_progress` — claim
- `bd update <id> --status=blocked` — stuck, add comment
- `bd comments add <id> "..."` — comment
- `bd close <id>` — done
- **Never take a task with open `blockedBy`** — close blockers first
- Blocked → block it, `<loop:stage>develop</loop:stage>`
- No bd, no git → `<loop:failed>reason</loop:failed>`

## Hard Rules

| Rule               | Violation                                            |
| ------------------ | ---------------------------------------------------- |
| One task per round | Never 2+ tasks                                       |
| Read before write  | Never modify unread files                            |
| No API guessing    | Context7 / WebFetch for ANY external API             |
| Scope lock         | Don't touch unrelated files                          |
| No dead code       | No TODOs, no commented-out code, no unused imports   |
| Match patterns     | Follow existing style exactly                        |
| Fail loud          | Stuck → block task, move on. Never produce broken code |
| No opinions        | Don't suggest. Don't add features. Execute.          |
