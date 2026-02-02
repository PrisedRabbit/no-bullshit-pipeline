# Update Flow

## Pre-Check

```
Previous work exists?
  │
  ├── NO  → Continue
  │
  └── YES → ALL done?
            ├── YES → /hltm-retro, then continue
            └── NO  → ASK USER (resume / force retro / force delete)
```

---

## Task List (if retro needed first)

**Use TodoWrite to track.**

| # | Task | Blocked By |
|---|------|------------|
| 1 | /hltm-retro | - |
| 2 | Update PRD | #1 |
| 3 | /hltm-solutioning | #2 |
| 4 | /hltm-implementation | #3 |

---

## Docs First

**FORBIDDEN:** grep `src/` to understand architecture
**REQUIRED:** Read docs first:

1. docs/architecture.md
2. docs/project-context.md
3. docs/epics/*.md

---

## Classify Update

| Type | Criteria |
|------|----------|
| FIX | Bug, <50 lines, single file |
| FEATURE | New page/component |
| MAJOR | Data model, architecture change |

---

## FIX Flow

```
/hltm-implementation (single story)
```

---

## FEATURE / MAJOR Flow

```
1. Update docs/prd.md
2. /hltm-solutioning (MAJOR: also architecture)
3. /hltm-implementation
```

---

## Archive

```bash
mkdir -p input/done && mv input/update.md input/done/update-$(date +%Y%m%d-%H%M).md
```
