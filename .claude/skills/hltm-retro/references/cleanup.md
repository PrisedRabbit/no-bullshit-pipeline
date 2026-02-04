# Cleanup

## Script

**No script. Manual cleanup only.**

## What Gets Deleted

| Path                          | Content                                                                      |
| ----------------------------- | ---------------------------------------------------------------------------- |
| `{planning_artifacts}/`       | BMAD planning artifacts. **DELETE:** epics, user stories, wireframes, intermediate diagrams. **KEEP:** PRD, Architecture, UX Guidelines (for Brownfield context). |
| `{implementation_artifacts}/` | BMAD implementation artifacts (stories, sprint-status, test summaries, etc.). **DELETE ALL** (except keeping logs if needed). |

## What Stays

| Path                             | Why                                          |
| -------------------------------- | -------------------------------------------- |
| `{planning_artifacts}/prd*.md` | Project Context |
| `{planning_artifacts}/arch*.md` | Project Context |
| `{planning_artifacts}/ux*.md`  | Project Context |
| `{project_knowledge}/`           | Long-term knowledge (if used by the project) |
| `{PROJECT_ROOT}/changelogs/changelog-*.md` | History                                |

## Git = Archive

Don't create `/archive` folders. Just delete.

```bash
# Recover deleted files from git:
git log --all --full-history -- <path>
git checkout <commit>^ -- <path>
```

## Post-Cleanup Invariant

Verify:

- `{project_knowledge}/project-context.md` exists (if used by the project)
- `{planning_artifacts}` contains ONLY PRD/Architecture/UX docs.
- Transient DONE artifacts removed (without destroying in-progress/backlog artifacts).
