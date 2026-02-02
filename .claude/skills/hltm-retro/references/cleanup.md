# Cleanup

## Script

**Use the script. Don't delete manually.**

```bash
bash ./scripts/cleanup.sh
```

Script will:
1. Confirm with user
2. Delete: epics, stories, prd, sprint-status
3. Reset .hltm/session.yaml to idle
4. Verify post-cleanup invariants

## What Gets Deleted

| Path | Content |
|------|---------|
| `docs/epics/` | All epic files |
| `docs/stories/` | All story files (if exists) |
| `docs/prd.md` | Requirements doc |
| `sprint-status.yaml` | Sprint tracking |

## What Stays

| Path | Why |
|------|-----|
| `docs/architecture.md` | Long-term reference |
| `docs/ux-design.md` | Long-term reference |
| `docs/project-context.md` | Rules for agents |
| `changelog-*.md` | History |
| `.hltm/session.yaml` | Reset to idle |

## Git = Archive

Don't create `/archive` folders. Just delete.

```bash
# Recover deleted files from git:
git log --all --full-history -- docs/epics/
git checkout <commit>^ -- docs/epics/
```

## Post-Cleanup Invariant

Script verifies:
- `docs/project-context.md` exists
- `docs/architecture.md` exists
- `docs/prd.md` deleted
- `sprint-status.yaml` deleted
- `docs/epics/` empty or gone
- `.hltm/session.yaml` phase = idle
