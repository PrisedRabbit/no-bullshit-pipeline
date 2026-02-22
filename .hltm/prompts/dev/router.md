# Development Methodology

## BEFORE YOU START: READ THE INSTRUCTIONS

Based on the `current stage`, read the corresponding instructions file.

| Stage         | File to read                    |
| ------------- | ------------------------------- |
| `develop`     | `.hltm/prompts/dev/develop.md`  |
| `review:<id>` | `.hltm/prompts/dev/review.md`   |
| `fix:<id>`    | `.hltm/prompts/dev/fix.md`      |
| `learning`    | `.hltm/prompts/dev/learning.md` |

Any other stage then use `.hltm/prompts/dev/develop.md`

**Do NOT improvise. Do NOT guess the procedure. Read the file. Follow it exactly.**

## Hard Rules

| #   | Rule               | Violation = immediate stop                                        |
| --- | ------------------ | ----------------------------------------------------------------- |
| 1   | One task per round | Never work on 2+ tasks simultaneously                             |
| 2   | Read before write  | Never modify a file you haven't read this round                   |
| 3   | No API guessing    | Context7 / WebFetch for ANY external API. No exceptions.          |
| 4   | Scope lock         | Don't touch files unrelated to the current task                   |
| 5   | No dead code       | No TODOs, no commented-out code, no unused imports                |
| 6   | Match patterns     | Follow existing codebase style exactly                            |
| 7   | Fail loud          | If stuck → block the task, move on. Never silently produce broken code. |
| 8   | No opinions        | Don't suggest improvements. Don't add features. Execute the task. |
