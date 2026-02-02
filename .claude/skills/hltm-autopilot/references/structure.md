# Project Structure

## Input

```
input/
├── brief.md        # New project requirements
├── update.md       # Update request (when updating)
└── done/           # Archive of processed inputs
```

## BMAD Output

```
_bmad/              # BMAD framework (via npx bmad-method)

docs/
├── prd.md          # BACKLOG: pending requirements
├── ux-design.md
├── architecture.md
└── epics/
    ├── epic-01-*.md
    └── ...

sprint-status.yaml  # PROJECT STATE
```

## Generated App

```
src/                # Application code
├── [per BMAD architecture]
└── ...
```
