# Result Handling

## Process Skill Result

**Scenario A: Skill returns FULL State** (Retro)
- Validate JSON (must have `phase`).
- If contains `changelog` → process it (see Changelog section), then remove from result.
- If `error: critical` → Write `{ ..., blocked: true }` → STOP. Print: "⛔ Critical Error: {reason}"
- Else → **OVERWRITE** `.hltm/session.yaml` with result object (excluding `changelog`).

**Scenario B: Skill returns Status/Verdict** (Brief, Planning, Solutioning, Readiness, Create-Stories)
- `status: done` / `verdict: pass` → Lookup **Next Phase** in `fsm.yaml` → Write `{ phase: "NEXT_PHASE" }`.
- `verdict: fail` (Readiness) → Lookup fallback in `fsm.yaml` → Write `{ phase: "solutioning" }`.
- If result contains `blueprint` (from Brief) → cache it: `{ phase: "NEXT", blueprint: "..." }`.

**Scenario C: Implementation loop** (Develop, Testing, Code-Review)
Router calls sub-skill directly (see `implementation-routing.md`), processes result:
- `status: done` / `verdict: pass` → advance `step` per routing table → write state
- `status: fail` / `verdict: fail` → increment `attempt` → apply recovery logic
- All stories done → transition to `retro`

Each sub-skill call = ONE atomic step. Router MUST write state to `.hltm/session.yaml` before next call.

## Changelog

If result contains `changelog`:
- Read/Create `{PROJECT_ROOT}/changelogs/changelog-YYYY-MM.md`.
- Append formatted entries.
- Print: "📝 Changelog updated."

## Post-Flight: Questions

After **solutioning** phase:
- Check `input/questions.md`.
- If NOT empty:
  - Write `{ phase: "solutioning", waiting_for_answers: true }`.
  - STOP. Print: "⚠️ Questions in input/questions.md. Answer them, then remove 'waiting_for_answers: true' from session.yaml and run /hltm-autopilot again."

On re-run with `waiting_for_answers: true`:
- If user removed flag manually → clear it → re-run solutioning (update mode).
- If flag still present → STOP. Print: "Still waiting for answers. Remove flag when done."
