---
name: hltm-context
description: Generate project-context.md - rules, constraints, invariants for agents
version: 1.0.0
model: opus
context: fork
agent: general-purpose
---

# Project Context

Creates `docs/project-context.md` — rules that agents MUST follow.

## What It Is

- Tech stack + exact versions
- Code conventions (naming, imports, patterns)
- Constraints (what NOT to do)
- Invariants (what must ALWAYS be true)

## What It Is NOT

- Feature list
- Implementation history
- "What we did"

## When to Run

**ONCE** — before first implementation (in `/hltm-readiness`).

## Who Can Update

**ONLY `/hltm-retro`** — and only if found invariant rule that breaks project without it.

NOT dev. NOT planner. NOT architect during work.

## Command

```
/hltm-context
```

## Input

- Blueprint (`hltm-blueprints/`)
- `docs/architecture.md` (if exists)

## Output

`docs/project-context.md`

## References

| Reference | Description |
|-----------|-------------|
| [generate.md](./references/generate.md) | Generation process |
| [template.md](./references/template.md) | File template |
