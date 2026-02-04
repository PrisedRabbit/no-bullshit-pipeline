# Retrospective Process

## Steps

1. Resolve artifact locations from `_bmad/bmm/config.yaml`
2. Read artifacts (architecture, project-context, epics)
3. Verify all epics done
4. Read all story files
5. Classify lessons
6. Update project-context.md
7. Update CHANGELOG
8. Cleanup:
   - Default: manual selective cleanup of DONE items only

**Use TodoWrite to track.**

---

## Docs First

Read before analyzing:
1. `{project_knowledge}/architecture.md` (if preserved) OR `{planning_artifacts}/*architecture*.md`
2. `{project_knowledge}/project-context.md` (if used)
3. `{planning_artifacts}/*epic*.md` (typically `epics.md`)

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

Git = archive.
