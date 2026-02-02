# Creative PARTY

Discussion mode for complex decisions.

## When to Use

- Architecture decisions
- UX trade-offs
- Complex feature design
- Strategic choices
- "Should we X or Y?"

## Command

```
/hltm-party
```

Or invoke via BMAD:

```
/bmad-party-mode
```

## What It Does

1. Gathers relevant agents (architect, ux, pm, etc.)
2. Presents the problem
3. Each agent gives perspective
4. Synthesizes decision
5. Documents in appropriate place

## Rules

Creative PARTY does **NOT**:
- Change current phase
- Skip FSM transitions
- Edit project-context.md (only retro can)
- Auto-implement decisions

## Output

Decision goes to:

| Decision Type | Destination |
|--------------|-------------|
| Architecture | ADR in docs/architecture.md |
| UX | Update docs/ux-design.md |
| Requirements | Update docs/prd.md |
| Implementation | Story notes |

## Example

```
User: /hltm-party
Context: Deciding between REST vs GraphQL for API

Participants:
- Architect: "REST simpler, fits our needs"
- PM: "GraphQL overkill for MVP"
- Dev: "REST easier to test"

Decision: REST
→ Added to docs/architecture.md as ADR
```

## No Auto-Trigger

Creative PARTY is **always manual**.

User calls it when needed. System never auto-calls Creative PARTY.
