---
phase: 05-pipeline-builder-redesign
verified: 2026-02-19T00:35:00Z
status: human_needed
score: 6/6 must-haves verified
human_verification:
  - test: "Build and run app; open Settings > Pipelines > New Pipeline; click '+ Add Step'"
    expected: "Picker appears with two sections — 'Processing' (5 presets) and 'Delivery' (connected integrations or empty message)"
    why_human: "Visual rendering of picker and its two-section layout cannot be verified without running the app"
  - test: "Click 'Meeting Notes' preset"
    expected: "Step appears immediately in the steps list with name 'meeting-notes', connector badge 'llm', input 'transcript' — no form to fill in"
    why_human: "Verifies zero-friction one-click add behavior in live UI"
  - test: "Click 'Custom Prompt' preset"
    expected: "Inline form appears below step list with: textarea, 'Save as reusable template' checkbox (hidden name input), Cancel and 'Add Step' buttons"
    why_human: "Visual rendering of the custom prompt form requires running the app"
  - test: "Type a prompt, leave checkbox unchecked, click 'Add Step'"
    expected: "Step added with connector=llm; opening step editor shows prompt in a textarea (not a template select)"
    why_human: "Verifies prompt_inline step creation path and step editor display"
  - test: "Drag a step handle to reorder two steps, then Save pipeline"
    expected: "Saved pipeline order matches the visual order after drag; fixStepInputs() ensures input chain is correct"
    why_human: "Drag-and-drop behavior in macOS WKWebView requires live testing with SortableJS"
  - test: "Check visual assembly preview below steps list"
    expected: "Preview shows: 'transcript → stepA (llm) → stepB (llm)'. Delivery steps (notion, save, slack) show in green; processing steps in default color"
    why_human: "Visual styling of delivery vs processing nodes requires running the app"
  - test: "Click a step to edit it; verify Provider and Model are NOT visible by default"
    expected: "Step editor shows prompt field + collapsed 'Advanced' disclosure; expanding 'Advanced' reveals Provider and Model selects"
    why_human: "HTML details element open/closed state is visual behavior"
  - test: "Run cargo check in src-tauri"
    expected: "No compilation errors — prompt_inline changes in pipelines.rs, connectors/llm.rs, pipeline_engine.rs compile cleanly"
    why_human: "Rust toolchain (cargo) is not available in the verification environment; must be run by developer"
---

# Phase 5: Pipeline Builder Redesign Verification Report

**Phase Goal:** Users can build pipelines by picking from labeled presets in two categories, with automatic step chaining, drag-and-drop reordering, and a visual assembly preview
**Verified:** 2026-02-19T00:35:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | The step picker shows two sections — Processing (AI steps) and Delivery (send somewhere) — with built-in presets that add with no required fields | VERIFIED | `showPicker()` in `src/pipeline-builder.js:134` builds both sections; `PROCESSING_PRESETS[0..4]` are 5 entries; delivery section reads from `notionProfiles`, `savePathIntegrations`, `slackIntegrations` |
| 2 | Adding a Meeting Notes, Action Items, Summary, Structure, or Custom Prompt step creates a correctly configured step without the user filling in connector type, model, or input source | VERIFIED | `addPresetStep()` at line 223 deep-copies preset.step (which has `connector`, `input`, `config.prompt_template` pre-set); `fixStepInputs()` auto-chains input references |
| 3 | The Custom Prompt step shows one text area; an optional checkbox saves the prompt as a reusable template | VERIFIED | `showCustomPromptForm()` at line 232 renders textarea + checkbox; unchecked path sets `stepConfig = { prompt_inline: promptText }`; checked path calls `invoke('save_prompt_template', ...)` then sets `stepConfig = { prompt_template: templateName }` |
| 4 | Steps can be reordered by dragging; the saved pipeline order matches the visual order after drag | VERIFIED | SortableJS `onEnd` callback (line 460) does `pipelineEditorSteps.splice(evt.oldIndex, 1)` then `splice(evt.newIndex, 0, moved)`, then `fixStepInputs()`; `savePipelineDefBtn` saves `pipelineEditorSteps` directly; no `draggable="true"` attribute in step HTML template |
| 5 | A visual chain preview below the step list shows the full pipeline flow including automatic transcript-to-step-1 and step-N-to-next-step chaining | VERIFIED | `renderPipelinePreview()` at line 675 generates `transcript → step1 (connector) → step2 (connector)` chain; preview div `#pipeline-preview` appears after `#pipeline-steps-list` in `src/index.html:578-588`; delivery steps get `.preview-node.delivery` class (green); `fixStepInputs()` ensures automatic input chaining |
| 6 | Provider and model settings are hidden by default and available in a collapsed Advanced section per step | VERIFIED | `showStepEditor()` LLM branch at line 534 wraps Provider and Model in `<details class="step-editor-advanced">` — collapsed by default; CSS at `src/styles.css:2923` styles the Advanced section |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/pipeline-builder.js` | Extracted pipeline builder module with all pipeline state and functions | VERIFIED | 749 lines; contains `var allPipelineDefs`, `PROCESSING_PRESETS`, `renderPipelineSteps`, `renderPipelinePreview`, `fixStepInputs`, `showStepEditor`, `openPipelineEditor`, `closePipelineEditor`, `loadPipelineDefs`, `renderPipelineDefsList`, `showPicker`, `showCustomPromptForm`, `addPresetStep`, `buildDeliveryOptions`, all event listener wiring |
| `src/vendor/sortable.min.js` | SortableJS library for drag-and-drop | VERIFIED | 1 line (minified); `Sortable 1.15.6 - MIT` header confirmed |
| `src-tauri/src/prompt_templates.rs` | Built-in action-items, summary, structure templates merged into existing registry | VERIFIED | `get_builtin_templates()` returns 6 templates including `action-items`, `summary`, `structure`; `load_prompt_templates()` has builtin merge logic at lines 244-255 |
| `src-tauri/src/pipelines.rs` | Updated LLM validation accepting prompt_template OR prompt_inline | VERIFIED | `validate_step_config()` LLM arm at line 126: `has_template` OR `has_inline` check; test `test_llm_step_with_prompt_inline_passes` at line 511 present |
| `src-tauri/src/connectors/llm.rs` | LlmConfig and execute() support for prompt_inline path | VERIFIED | `LlmConfig` struct has `prompt_template: Option<String>` and `prompt_inline: Option<String>` at line 11; `execute()` standard path at line 179 uses `if let Some(ref template_name) = ... else if let Some(ref inline) = ...` |
| `src-tauri/src/pipeline_engine.rs` | build_augmented_prompt reads prompt_inline when prompt_template is absent | VERIFIED | `build_augmented_prompt()` at line 136: full if/else branching for `prompt_template` vs `prompt_inline`, each branch reads input file independently |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/pipeline-builder.js` | `src/main.js` | `var allPipelineDefs` window-global | WIRED | `var allPipelineDefs = []` at line 5; `updateSidebarCounts()` in main.js reads `allPipelineDefs.length` at main.js:1242; `loadPipelineDefs()` called in `init()` at main.js:1584 |
| `src/pipeline-builder.js` | `src/integrations-settings.js` | `typeof notionProfiles` typeof guards | WIRED | `typeof notionProfiles !== 'undefined'` at line 79 (buildDeliveryOptions); `typeof savePathIntegrations !== 'undefined'` at line 93; `typeof slackIntegrations !== 'undefined'` at line 107 |
| `src/index.html` | `src/pipeline-builder.js` | Script tag load order | WIRED | Lines 776-779: `sortable.min.js` → `main.js` → `integrations-settings.js` → `pipeline-builder.js` in correct dependency order |
| `src/pipeline-builder.js` | `src-tauri/src/prompt_templates.rs` | Preset `config.prompt_template` names match backend registry | WIRED | PROCESSING_PRESETS use names `meeting-notes`, `action-items`, `summary`, `structure` — all present in `get_builtin_templates()` return |
| `src-tauri/src/connectors/llm.rs` | `src-tauri/src/pipelines.rs` | `LlmConfig::from_value` reads `prompt_inline` as fallback | WIRED | `LlmConfig.prompt_inline: Option<String>` mirrors `validate_step_config()` `has_inline` check |
| `src-tauri/src/pipeline_engine.rs` | `src-tauri/src/connectors/llm.rs` | `build_augmented_prompt` reads `prompt_inline` when `prompt_template` absent | WIRED | Both files have matching if/else branches for `prompt_template` → `prompt_inline` pattern |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| BLDR-01 | 05-02 | Step picker shows two categories: Processing (AI) and Delivery (send somewhere) | SATISFIED | `showPicker()` renders `.step-picker-section-title` "Processing" and "Delivery" sections |
| BLDR-02 | 05-02 | Built-in processing presets available with one click: Meeting Notes, Action Items, Summary, Structure, Custom Prompt | SATISFIED | `PROCESSING_PRESETS` array has all 5 entries with step configs |
| BLDR-03 | 05-02 | Preset steps add with zero fields to fill (smart defaults for name, connector, input) | SATISFIED | `addPresetStep()` deep-copies full step config; `fixStepInputs()` auto-assigns input |
| BLDR-04 | 05-03 | Custom prompt step has one field (textarea) with optional "Save as reusable template" checkbox | SATISFIED | `showCustomPromptForm()` implements full form; checkbox toggles name input visibility |
| BLDR-05 | 05-01 | Step input chaining is automatic: step 1 = transcript, step N = previous step output, with toggle to override | SATISFIED | `fixStepInputs()` enforces step 1 = transcript, subsequent steps chain to previous step name; step editor provides Input select for override |
| BLDR-06 | 05-01 | Pipeline steps can be reordered via drag-and-drop | SATISFIED | SortableJS `onEnd` reorders `pipelineEditorSteps` array; `renderPipelineSteps()` re-initializes Sortable after innerHTML |
| BLDR-07 | 05-03 | Pipeline assembly preview shows visual chain of steps below the step list | SATISFIED | `renderPipelinePreview()` renders `transcript → step1 → stepN` chain; preview div positioned below steps in HTML |
| BLDR-08 | 05-03 | Provider/Model hidden by default, uses global settings; per-step override available in Advanced section | SATISFIED | `showStepEditor()` wraps Provider/Model in `<details class="step-editor-advanced">` collapsed by default |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/pipeline-builder.js` | 71 | `step: null // handled specially in Plan 05-03` | Info | Comment is stale — Plan 05-03 completed; the null is correct (handled by `showCustomPromptForm()` branch), but comment references old plan |

No blockers found. No empty implementations, no TODO stubs, no placeholder returns.

### Human Verification Required

#### 1. Step Picker Visual Rendering

**Test:** Open Settings > Pipelines > New Pipeline; click "+ Add Step"
**Expected:** Picker appears below the button with two clearly labeled sections: "PROCESSING" (5 presets with icons) and "DELIVERY" (either connected integration options or empty message)
**Why human:** Picker UI is dynamically created DOM — visual rendering and section layout require running the app

#### 2. One-Click Preset Add (Zero Fields)

**Test:** Click "Meeting Notes" preset in picker
**Expected:** Picker closes; a step card appears in the steps list showing name "meeting-notes", connector badge "llm", input "transcript" — no modal, no form, no fields to fill
**Why human:** Verifies the zero-friction preset add behavior and correct step display in live UI

#### 3. Custom Prompt Form

**Test:** Click "Custom Prompt" preset in picker
**Expected:** Picker closes; an inline form appears below the steps list with: a textarea ("Write your prompt here. Use {transcript}..."), "Save as reusable template" checkbox (unchecked), Cancel and "Add Step" buttons; checking the checkbox reveals a template name input
**Why human:** Visual form rendering and checkbox toggle behavior require running the app

#### 4. prompt_inline Step Creation and Editor Display

**Test:** In Custom Prompt form, type "Summarize this meeting: {transcript}", leave checkbox unchecked, click "Add Step"; then click the step to edit it
**Expected:** Step added; step editor shows a textarea (not a template select dropdown) containing "Summarize this meeting: {transcript}"; Provider and Model are hidden under collapsed "Advanced"
**Why human:** Step editor renders dynamically based on step.config — requires live verification that prompt_inline shows textarea instead of select

#### 5. Drag-and-Drop Reorder

**Test:** Add 3 steps; drag the middle step to the bottom by its grip handle; click Save
**Expected:** Steps reorder visually with ghost feedback; saved pipeline reflects the new order; input chaining auto-updates (step 1 still reads "transcript")
**Why human:** SortableJS drag behavior in macOS WKWebView requires physical drag-and-drop testing

#### 6. Assembly Preview Visual Distinction

**Test:** Add one LLM step (e.g. Meeting Notes) and one delivery step (e.g. a Notion integration); observe the preview below the steps list
**Expected:** Preview shows "transcript → meeting-notes (llm) → send-to-X (notion)" where the notion node has green-tinted styling
**Why human:** Color styling of delivery vs processing preview nodes requires visual inspection

#### 7. Rust Compilation

**Test:** Run `cargo check` in `/workspace/src-tauri`
**Expected:** Zero compilation errors; all three modified files (pipelines.rs, connectors/llm.rs, pipeline_engine.rs) compile with the new `prompt_inline` field
**Why human:** `cargo` binary not available in the verification environment — must be run by developer

---

## Gaps Summary

No gaps found. All 6 observable truths are verified by code evidence. All 8 BLDR requirements (BLDR-01 through BLDR-08) are covered across the three plans and implemented in the codebase.

The only items remaining are human verification items that require running the app (visual rendering, drag-and-drop behavior, Rust compilation). These are not gaps — they are standard "needs live testing" items that cannot be verified programmatically.

---

_Verified: 2026-02-19T00:35:00Z_
_Verifier: Claude (gsd-verifier)_
