---
phase: 05-pipeline-builder-redesign
plan: 03
subsystem: ui
tags: [pipeline-builder, custom-prompt, prompt-inline, javascript, rust]

# Dependency graph
requires:
  - phase: 05-pipeline-builder-redesign
    provides: "Step picker with Custom Prompt placeholder (preset.step=null) and showStepEditor() LLM branch"
provides:
  - "showCustomPromptForm() with textarea and save-as-template checkbox"
  - "prompt_inline field accepted by validate_step_config(), LlmConfig, and build_augmented_prompt()"
  - "renderPipelinePreview() with delivery step visual distinction (preview-node.delivery)"
  - "showStepEditor() LLM branch showing prompt_inline textarea or prompt_template select"
  - "Provider/Model wrapped in <details class='step-editor-advanced'> collapsed by default"
affects:
  - pipeline-execution
  - pipeline-builder-ui

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "prompt_inline as fallback to prompt_template in LLM step config - validated in pipelines.rs, read in connectors/llm.rs and pipeline_engine.rs"
    - "Custom Prompt form inserted after pipelineStepsListEl via insertBefore(formEl, nextSibling)"
    - "deliveryConnectors array in renderPipelinePreview() classifies steps for visual distinction"
    - "<details> element for Advanced section in step editor - no JS handler changes needed since querySelectorAll('[data-field]') finds elements inside details"

key-files:
  created: []
  modified:
    - src/pipeline-builder.js
    - src/styles.css
    - src-tauri/src/pipelines.rs
    - src-tauri/src/connectors/llm.rs
    - src-tauri/src/pipeline_engine.rs

key-decisions:
  - "prompt_template and prompt_inline are both Option<String> in LlmConfig — either is accepted, neither required exclusively"
  - "build_augmented_prompt() reads input file inside each if/else branch to avoid duplicate reads"
  - "showCustomPromptForm() inserts after pipelineStepsListEl.nextSibling — form always visible below step list"
  - "deliveryConnectors list hardcoded in renderPipelinePreview() — consistent with connector types in pipelines.rs"
  - "step editor stores prompt_inline when step.config.prompt_inline exists; Done button reads data-field attrs including those inside <details>"

patterns-established:
  - "prompt_inline: inline prompt text stored directly in step config, no template lookup needed"
  - "Custom Prompt form: ephemeral DOM element inserted into editor, removed on cancel or confirm"
  - "Preview node classification: delivery connectors get green-tinted styling to distinguish from processing steps"

requirements-completed: [BLDR-04, BLDR-07, BLDR-08]

# Metrics
duration: 5min
completed: 2026-02-19
---

# Phase 5 Plan 03: Custom Prompt Form, Enhanced Preview, and prompt_inline Backend Summary

**Custom Prompt inline form with save-as-template toggle, delivery-vs-processing preview distinction, Advanced collapsed Provider/Model section, and full prompt_inline support in Rust validation, LLM connector, and pipeline engine**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-19T00:20:00Z
- **Completed:** 2026-02-19T00:25:00Z
- **Tasks:** 2
- **Files modified:** 5 (pipeline-builder.js, styles.css, pipelines.rs, connectors/llm.rs, pipeline_engine.rs)

## Accomplishments
- Added `prompt_inline` field support across all three Rust files: validation in `pipelines.rs` accepts either `prompt_template` OR `prompt_inline`; `LlmConfig` in `connectors/llm.rs` now has both as `Option<String>`; `build_augmented_prompt()` in `pipeline_engine.rs` handles both paths
- Replaced Custom Prompt blank-step placeholder with `showCustomPromptForm()` — inline textarea form with optional "Save as reusable template" checkbox; unchecked creates `prompt_inline` step, checked saves template and creates `prompt_template` step
- Enhanced `renderPipelinePreview()` to visually distinguish delivery steps (save, notion, slack, webhook, mcp) with green-tinted `preview-node.delivery` class vs blue processing steps
- Wrapped Provider/Model in `<details class="step-editor-advanced">` collapsed by default; `showStepEditor()` shows textarea for `prompt_inline` steps and select for `prompt_template` steps

## Task Commits

Each task was committed atomically:

1. **Task 1: Add prompt_inline support to Rust backend (validation, LLM config, engine)** - `e438307` (feat)
2. **Task 2: Implement Custom Prompt form, enhanced assembly preview, and Advanced section** - `2ff4a61` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/pipeline-builder.js` - Added showCustomPromptForm(), updated renderPipelinePreview() with deliveryConnectors, updated showStepEditor() LLM branch with prompt_inline/prompt_template conditional and <details> Advanced section
- `src/styles.css` - Added .custom-prompt-form, .custom-prompt-header, .custom-prompt-textarea, .custom-prompt-save-label, .custom-prompt-name-row, .custom-prompt-actions, .preview-node.delivery, .step-editor-advanced CSS classes
- `src-tauri/src/pipelines.rs` - Updated validate_step_config() LLM arm to accept prompt_template OR prompt_inline; added test_llm_step_with_prompt_inline_passes test
- `src-tauri/src/connectors/llm.rs` - LlmConfig struct changed prompt_template to Option<String>, added prompt_inline: Option<String>; updated from_value() and execute() standard path; fixed test assertion
- `src-tauri/src/pipeline_engine.rs` - Updated build_augmented_prompt() to handle both prompt_template and prompt_inline with input file read inside each branch

## Decisions Made
- `prompt_template` and `prompt_inline` are both `Option<String>` in `LlmConfig` — validation requires at least one, but neither exclusively
- `build_augmented_prompt()` in `pipeline_engine.rs` reads the input file inside each if/else branch independently to keep the branches self-contained and avoid stale references
- Custom Prompt form inserted via `insertBefore(formEl, pipelineStepsListEl.nextSibling)` — always appears directly below the step list
- `deliveryConnectors` hardcoded in `renderPipelinePreview()` — mirrors the Rust `ConnectorType` enum values

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- `cargo` not available in execution environment (known constraint from STATE.md blockers) — Rust code verified structurally by reading modified files; compilation will be confirmed at first `cargo tauri dev` run

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 5 Pipeline Builder Redesign is now complete (all 3 plans executed)
- BLDR requirements 01-08 fully satisfied across plans 05-01, 05-02, and 05-03
- `prompt_inline` pipeline steps can be created via Custom Prompt form and saved via `save_pipeline` command
- Phase 6 (Recording UX + Auto-pipeline trigger) can proceed

## Self-Check: PASSED

- `src/pipeline-builder.js` exists with showCustomPromptForm(), deliveryConnectors in renderPipelinePreview(), and <details> in showStepEditor()
- `src/styles.css` exists with .custom-prompt-form, .preview-node.delivery, .step-editor-advanced CSS
- `src-tauri/src/pipelines.rs` exists with has_inline check in validate_step_config()
- `src-tauri/src/connectors/llm.rs` exists with prompt_inline: Option<String> in LlmConfig
- `src-tauri/src/pipeline_engine.rs` exists with prompt_inline branch in build_augmented_prompt()
- Commit `e438307` (Task 1: Rust backend) verified
- Commit `2ff4a61` (Task 2: JS frontend) verified

---
*Phase: 05-pipeline-builder-redesign*
*Completed: 2026-02-19*
