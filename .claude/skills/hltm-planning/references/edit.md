# Edit PRD

Edit existing PRD based on feedback or new requirements.

## Agent + Workflow

```
Agent: _bmad/bmm/agents/pm.md
Workflow: _bmad/bmm/workflows/2-plan-workflows/create-prd/workflow.md
Mode: Edit (steps-e/)
```

## Prompt

```
CONTEXT:
- Existing PRD: docs/prd.md
- Changes requested: {user input or input/update.md}

TASK:
Load PM agent and run PRD workflow in EDIT mode.
Apply requested changes while maintaining PRD consistency.

RULES:
- NEVER ask user questions — infer from context
- Keep existing structure, modify only what's needed
- Validate changes don't break other sections
```

## After Edit

Run validation:
```
Workflow: _bmad/bmm/workflows/2-plan-workflows/create-prd/workflow.md
Mode: Validate (steps-v/)
```
