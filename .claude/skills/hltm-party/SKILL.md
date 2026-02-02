---
name: hltm-party
description: Multi-agent discussion for complex features
version: 2.0.0
model: opus
context: fork
agent: general-purpose
---

# HLTM Party

Two modes: **Creative** and **Recovery**.

## Creative PARTY

Manual. Call for architecture decisions, UX trade-offs, complex features.

```
/hltm-party
```

→ [creative.md](./references/creative.md)

## Recovery PARTY

Auto-triggered when stuck (attempt ≥ 3, no progress).

**One per epic.** `recovery_used: true` → second PARTY forbidden.

→ [recovery.md](./references/recovery.md)

## Rules (both modes)

Does NOT:
- Change phase
- Skip FSM
- Edit project-context.md

## References

| Reference | Description |
|-----------|-------------|
| [creative.md](./references/creative.md) | Creative party mode |
| [recovery.md](./references/recovery.md) | Recovery party mode |
