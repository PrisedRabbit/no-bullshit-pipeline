# Project Structure

## Input

```
input/
├── brief.md        # Requirements (greenfield or brownfield)
├── questions.md    # Questions/Comments for the user (created by solutioning if needed)
└── done/           # Archive of processed inputs
```

## BMAD Output

```
_bmad/              # BMAD framework (via npx bmad-method)

_bmad-output/        # BMAD outputs (configured in _bmad/bmm/config.yaml)
├── planning-artifacts/      # PRD/UX/architecture/epics/etc (filenames per BMAD workflows)
└── implementation-artifacts/ # stories, sprint-status.yaml, tests, etc
```

## Knowledge (input for BMAD)

```
docs/               # project_knowledge (read as context; not BMAD output by default)
└── ...
```

## Changelogs

```
changelogs/
└── changelog-YYYY-MM.md  # Monthly changelog (user-facing)
```

## State

```
.hltm/
└── session.yaml    # FSM state (phase, epic, story, step, attempt)
```

## Generated App

```
src/                # Application code
├── [per BMAD architecture]
└── ...
```
