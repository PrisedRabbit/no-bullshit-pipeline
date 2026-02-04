# Cleanup Scenarios

## Scenario A: ALL DONE

All items from brief completed.

**Actions:**

- Archive `input/brief.md` → `input/done/brief-{timestamp}.md`
- Archive `input/brief-enhanced.md` → `input/done/brief-enhanced-{timestamp}.md`
- Archive `input/questions.md` → `input/done/questions-{timestamp}.md` (if exists)
- Delete all 3 files from `input/`

**Result:** Project clean. Ready for new brief.

---

## Scenario B: PARTIAL

Some items NOT completed.

**Actions:**

- Archive original `input/brief.md` → `input/done/brief-{timestamp}-partial.md`
- Archive `input/questions.md` → `input/done/questions-{timestamp}-partial.md` (if exists)
- **REWRITE** `input/brief.md` with:
  - Only UNFINISHED requirements
  - Section `## Status & Reasons` (why not completed)
- Delete `input/brief-enhanced.md`

**Result:** `brief.md` remains. Next autopilot run picks it up.

---

## Transient Cleanup (Both Scenarios)

**Delete:**

- `{implementation_artifacts}/sprint-status.yaml`
- Stories with `status: done` in `{implementation_artifacts}/stories/`

**Keep:**

- `src/` (codebase)
- `{project_knowledge}/` (docs)
- `{planning_artifacts}/` (PRD/Arch for brownfield)
