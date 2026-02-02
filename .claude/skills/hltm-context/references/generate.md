# Generate Project Context

## Purpose

Create `docs/project-context.md` with rules agents must follow. NOT a feature list.

## Process

### 1. Load Blueprint

Read from `hltm-blueprints/{blueprint}/`:
- `tech-stack.md` → versions, dependencies
- `architecture.md` → patterns, structure
- Module files → conventions

### 2. Load Architecture (if exists)

Read `docs/architecture.md`:
- Project-specific decisions
- ADRs (Architecture Decision Records)
- Deviations from blueprint

### 3. Extract Rules

From blueprint + architecture, extract:

**Tech Stack**
- Exact versions (Next.js 14.x, NOT "latest")
- Required dependencies
- Forbidden dependencies

**Code Conventions**
- File naming: `PascalCase.tsx` for components
- Import order: React → external → internal → types
- Pattern usage: Server Components by default

**Constraints (MUST NOT)**
- Don't modify `lib/firebase.ts` without review
- Don't create duplicate components
- Don't use `any` type
- Don't skip TypeScript

**Invariants (MUST ALWAYS)**
- All API routes must validate input
- All forms must use Zod schemas
- All components must handle loading state

### 4. Write File

Output to `docs/project-context.md` using template.

## Updates

Context changes ONLY when:
- Retro identifies new rule (lesson learned → constraint)
- Architecture changes
- New invariant discovered

NOT when:
- New feature added
- Story completed
- Code refactored
