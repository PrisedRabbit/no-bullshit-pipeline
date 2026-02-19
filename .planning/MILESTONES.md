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

