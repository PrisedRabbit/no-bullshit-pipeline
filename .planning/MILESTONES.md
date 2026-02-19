# Milestones

## v1 Pipelines v2 (Shipped: 2026-02-19)

**Phases completed:** 8 phases, 20 plans, 0 tasks

**Key accomplishments:**
- Schema-aware Notion connector with Keychain credential management, API-driven database schema reading, people alias resolution, and structured property formatting
- Automatic prompt augmentation — pipeline engine auto-injects schema-derived format specs into LLM prompts when followed by a structured delivery step
- Integrations settings wizard for Notion (API key → share instruction → DB picker → schema → people mapping) and named save path integrations
- Preset-based pipeline builder with categorized step picker (Processing/Delivery), one-click presets, SortableJS drag-and-drop, and visual assembly preview
- Zero-friction pre-assignment UX — pipeline chips in app bar for one-click recording, last-used persistence, auto-transcribe + auto-execute on stop
- Unified pipeline-as-label model replacing tags (zero-step pipeline = label) with lazy migration and built-in UI health check system

**Stats:** 83 commits, 76 files, +18,545/-695 lines, ~10,800 LOC in key v2 files
**Git range:** feat(01-01) → feat(08-02)
**Timeline:** 2026-02-18 → 2026-02-19

---


## v1.1 Resilience & Polish (Shipped: 2026-02-19)

**Phases completed:** 4 phases (9-12), 7 plans, 12 tasks

**Key accomplishments:**
- Fixed MutationObserver selector mismatch and consolidated dual Slack state to single source of truth (BUG-01, BUG-02)
- JSON retry-on-failure with NotionErrorKind enum for structured output error categorization and raw output preservation (ERR-01, ERR-02)
- Partial-success pipeline execution — delivery step failures continue to independent steps with per-step visual status indicators (ERR-03)
- Pipeline chip bar hardened with display:none for empty state, ARIA accessibility roles, and scrollable overflow for large collections (UX-01)
- Augmented prompt transparency — expandable section showing auto-injected context, with per-step wall-clock timing (UX-02, UX-03)
- Token budget validation gate prevents over-limit LLM calls, schema staleness warnings, and in-builder re-sync button (SCHM-01, SCHM-02, SCHM-03)

**Stats:** 13 feat/fix commits, 27 files changed, +3,565/-281 lines
**Git range:** fix(09-01) → feat(12-02)
**Timeline:** 2026-02-19
**Requirements:** 11/11 complete

---

