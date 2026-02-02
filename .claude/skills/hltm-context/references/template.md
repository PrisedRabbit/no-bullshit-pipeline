# Project Context Template

```markdown
# Project Context

> Rules for AI agents. NOT a feature list.

## Tech Stack

| Layer | Technology | Version |
|-------|------------|---------|
| Framework | Next.js | 14.x |
| UI | shadcn/ui | latest |
| Auth | Firebase Auth | 10.x |
| Database | Firestore | 10.x |
| Styling | Tailwind CSS | 3.x |

## Code Conventions

### File Naming
- Components: `PascalCase.tsx`
- Utilities: `camelCase.ts`
- Types: `types.ts` or `*.types.ts`

### Import Order
1. React/Next
2. External packages
3. Internal components (`@/components/`)
4. Internal utils (`@/lib/`)
5. Types

### Patterns
- Server Components by default
- `'use client'` only when needed
- Zod for all validation
- react-hook-form for forms

## Testing Rules

**Dev does NOT write tests.** QA (TEA) writes all tests.

Dev MUST:
- Write code only
- Document test invariants in story file (what MUST be true)
- Example: "user.email must be valid after save", "cart.total >= 0 always"

Dev MUST NOT:
- Write test files (`*.test.ts`, `*.spec.ts`)
- Run tests (except to verify existing don't break)

## Constraints (MUST NOT)

- [ ] Don't modify `lib/firebase.ts` without architecture review
- [ ] Don't create components outside `components/`
- [ ] Don't use `any` type
- [ ] Don't skip error handling
- [ ] Don't hardcode secrets
- [ ] Don't write tests (QA responsibility)

## Invariants (MUST ALWAYS)

- [ ] All API routes validate input with Zod
- [ ] All forms handle loading + error states
- [ ] All async operations have try/catch
- [ ] All user input is sanitized

## Key Files (Don't Touch)

| File | Why |
|------|-----|
| `lib/firebase.ts` | Core config |
| `app/layout.tsx` | Root layout |
| `components/ui/*` | shadcn primitives |
```
