# Architecture & UX Review

Before implementation, verify docs are still valid.

## Why

Between solutioning and implementation:
- Requirements may have clarified
- Tech constraints discovered
- Better patterns found
- UX feedback received

Dev agents read these docs. If outdated → bad code.

## Architecture Review

### Check Against

- `docs/prd.md` — requirements still match?
- `docs/epics/` — stories achievable with this architecture?
- Blueprint — still aligned?

### Red Flags

- Tech stack changed but not updated
- New requirements not covered
- Patterns in stories contradict architecture
- Missing ADRs for major decisions

### If Drift Found

Call architect agent:
```
/bmad-agent-bmm-architect
```

Ask: "Review and update docs/architecture.md based on current PRD and epics."

## UX Review

### Check Against

- `docs/prd.md` — user flows match requirements?
- `docs/epics/` — UI stories implementable?

### Red Flags

- Screens missing for requirements
- Flows don't match PRD user journeys
- Component patterns inconsistent
- Mobile/responsive not addressed

### If Drift Found

Call UX designer agent:
```
/bmad-agent-bmm-ux-designer
```

Ask: "Review and update docs/ux-design.md based on current PRD."

## Generate Project Context

**ONLY if `docs/project-context.md` does NOT exist.**

```
/hltm-context
```

Creates once. Never updates here.

**Updates ONLY via `/hltm-retro`** — if found invariant rule.

## Output

After review:
- `docs/architecture.md` — updated if needed
- `docs/ux-design.md` — updated if needed
- `docs/project-context.md` — created (if didn't exist)

Ready for implementation.
