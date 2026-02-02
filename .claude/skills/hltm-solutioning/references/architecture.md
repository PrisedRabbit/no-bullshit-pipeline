# Architecture Creation / Edit

## BMAD Workflow

```
/create-architecture
```

BMAD's `create-architecture` handles both create and edit modes internally.
It's a conversational workflow — asks clarifying questions.

## Input

- `docs/prd.md` — requirements
- `docs/ux-design.md` — UI needs (if exists)
- `hltm-blueprints/web-app/` — tech stack reference

## What BMAD Does

1. Reads PRD → extracts entities, features, integrations
2. Reads UX → extracts UI requirements
3. Uses blueprint as tech stack reference
4. Creates/updates `docs/architecture.md`:
   - Tech stack (from blueprint)
   - Project structure (from blueprint)
   - Data model (from PRD entities)
   - API routes (from PRD features)
   - Auth rules (from PRD)
   - Integrations (from PRD)
   - ADRs (if any deviations from blueprint)

## Alternative: Call Architect Agent

For review/update of existing architecture:

```
/bmad-agent-bmm-architect
```

Ask architect to review and update `docs/architecture.md`.

## Blueprint = Tech Stack Reference

| From Blueprint | From PRD |
|----------------|----------|
| Tech stack | Data model |
| Project structure | API routes |
| Patterns | Auth rules |
| Conventions | Integrations |

## ADR Example

If deviating from blueprint:

```markdown
## ADR: Use Supabase instead of Firebase

**Context:** Client requires PostgreSQL.
**Decision:** Replace Firebase with Supabase.
**Consequences:** Different auth API, SQL instead of NoSQL.
```

## Verify

- [ ] `docs/architecture.md` exists
- [ ] Tech stack from blueprint
- [ ] Data model covers PRD entities
- [ ] No TBD/TODO
