# Retrospective Process

## Steps

1. Read docs (architecture, project-context, epics)
2. Verify all epics done
3. Read all story files
4. Classify lessons
5. Update project-context.md
6. Update CHANGELOG
7. Run cleanup.sh

**Use TodoWrite to track.**

---

## Docs First

Read before analyzing:
1. docs/architecture.md
2. docs/project-context.md
3. docs/epics/*.md

---

## Classify Lessons

| If... | Then... |
|-------|---------|
| Agent will fuck up without this | → project-context.md |
| One-time / nice to know | → trash |

---

## CHANGELOG

1-3 lines per epic. Max.

```markdown
## [Epic 1] Auth
- User auth (email + Google)
- Profile page
```

---

## Cleanup

```bash
bash ./scripts/cleanup.sh
```

Git = archive.
