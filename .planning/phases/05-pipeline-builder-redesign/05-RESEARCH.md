# Phase 5: Pipeline Builder Redesign - Research

**Researched:** 2026-02-18
**Domain:** Vanilla JS frontend — pipeline builder extraction, preset step picker, SortableJS drag-and-drop, state-first re-render, assembly preview
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| BLDR-01 | Step picker shows two categories: Processing (AI) and Delivery (send somewhere) | No picker UI exists today — only a raw `+ Add Step` button that pushes a blank step into `pipelineEditorSteps`. Phase adds a modal/dropdown picker rendered as two sections. Connected integrations (loaded in Phase 4 via `notionProfiles`, `savePathIntegrations`, `slackIntegrations`) drive the Delivery section. |
| BLDR-02 | Built-in processing presets available with one click: Meeting Notes, Action Items, Summary, Structure, Custom Prompt | Five named presets, each mapping to a hard-coded `PipelineStep` with connector=`llm` and a specific `prompt_template` name from the built-in template registry. The Rust backend already ships these built-in templates (`meeting-notes`, `brainstorm`, `journal`, etc.) — Phase 5 adds new templates for Action Items, Summary, Structure to match the preset names. |
| BLDR-03 | Preset steps add with zero fields to fill (smart defaults for name, connector, input) | Each preset's JS object has `name`, `connector: 'llm'`, `input: 'transcript'` (or previous step name), and `config.prompt_template` pre-set. No dialog shown — step is added directly and state re-renders. |
| BLDR-04 | Custom prompt step has one field (textarea) with optional "Save as reusable template" checkbox | Custom Prompt preset opens a mini inline form (just textarea + checkbox) instead of the full step-editor. On confirm: if checkbox checked, calls `invoke('save_prompt_template', {...})` then sets `config.prompt_template` to the saved name. If unchecked, saves prompt text inline (new `config.prompt_inline` field). Needs Rust validation change or an `inline` virtual prompt template approach. |
| BLDR-05 | Step input chaining is automatic: step 1 = transcript, step N = previous step output, with toggle to override | `fixStepInputs()` already exists in `main.js` and does exactly this. State-first re-render pattern calls it after every mutation. Override is a dropdown in the step's Advanced section (pre-existing behavior preserved). |
| BLDR-06 | Pipeline steps can be reordered via drag-and-drop | Current code uses native HTML5 DnD (`draggable="true"` + `dragstart`/`dragover`/`drop` handlers). Decision is to replace with SortableJS — prior research confirmed native DnD is unreliable in macOS WKWebView. SortableJS must be added as a local vendor file (`src/vendor/sortable.min.js`). |
| BLDR-07 | Pipeline assembly preview shows visual chain of steps below the step list | `renderPipelinePreview()` already exists and renders a `transcript → step1 → step2` chain using `<span class="preview-node">` elements. Phase 5 moves it below the step list (HTML reorder) and enhances it to show connector type and delivery target label. |
| BLDR-08 | Provider/Model hidden by default, uses global settings; per-step override available in Advanced section | Current `showStepEditor()` always shows Provider/Model fields for `llm` connector. Phase 5 wraps these in a `<details>` element (collapsed by default). The step's `config.provider` and `config.model` fields remain unchanged — just the UI collapses them. |
</phase_requirements>

---

## Summary

Phase 5 is a pure frontend refactor and enhancement. No new Tauri commands are needed. The phase has three concrete deliverables: (1) extract `pipeline-builder.js` from `main.js` as a new `<script>` module with all pipeline-related state and functions migrated out; (2) replace the `+ Add Step` button with a categorized picker (two sections: Processing presets + Delivery integrations), with each preset adding a correctly-configured step with zero user input; (3) replace native HTML5 DnD with SortableJS, collapse Provider/Model into an Advanced `<details>` section, surface the Custom Prompt textarea + save-as-template checkbox, and move the assembly preview to below the step list.

The existing pipeline builder in `main.js` (lines 1566–1991) is already 425 lines and has all the foundational infrastructure: `pipelineEditorSteps` state array, `renderPipelineSteps()` full re-render, `fixStepInputs()` automatic chaining, and `renderPipelinePreview()` visual chain. The state-first re-render pattern is already in use. Phase 5 extracts, cleans, and enhances this — it does not redesign from scratch.

The one gap that requires backend attention is the Custom Prompt inline prompt: the current `PipelineStep.config` for LLM connector requires a `prompt_template` name, and `validate_step_config()` in `pipelines.rs` enforces this. An inline prompt (no saved template) needs either (a) a new optional `prompt_inline` field in `PipelineStep.config` that the engine substitutes directly, or (b) a silent auto-save as an anonymous template. Option (a) is cleaner and requires a small validation change in `pipelines.rs` and `pipeline_engine.rs`.

**Primary recommendation:** Extract `pipeline-builder.js` first (state-first pattern), add SortableJS vendor file, then build the picker and Advanced section. Tackle Custom Prompt inline storage last since it is the only change that touches Rust.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Vanilla JS DOM | — | All UI rendering | Project constraint: no framework, no bundler |
| SortableJS | `1.15.x` | Drag-and-drop step reordering | Prior research decision. One-line initialization, reliable in WKWebView, actively maintained. Native HTML5 DnD is broken in Tauri's WKWebView on macOS. Load as `src/vendor/sortable.min.js`. |
| `escapeHtml()` | built-in | XSS prevention | Every user-supplied string rendered into innerHTML |
| CSS `<details>`/`<summary>` | native HTML | Collapsed Advanced section | Zero JS needed, matches existing inline-editor styling |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `invoke` | Tauri 2.x | Call Rust for `save_prompt_template` | Only for Custom Prompt "Save as template" checkbox path |
| `window.__TAURI__.core.invoke` | Tauri 2.x | All other Rust calls (already used) | Existing pattern; no change |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| SortableJS vendor file | Native HTML5 DnD | Native DnD is unreliable in macOS WKWebView — do not use |
| SortableJS vendor file | CDN (jsdelivr) | App requires no internet at runtime; vendor file is the correct approach for a desktop app |
| `<details>` for Advanced | JS accordion with click toggle | `<details>` is semantic, zero JS, already styled by existing CSS resets; no downside |
| New `prompt_inline` field | Auto-save anonymous template | Auto-save pollutes template list with unnamed entries; `prompt_inline` is cleaner and keeps templates registry meaningful |

**Download SortableJS vendor file:**
```bash
# Download to src/vendor/sortable.min.js (using curl, no bundler)
curl -L "https://cdn.jsdelivr.net/npm/sortablejs@1.15.6/Sortable.min.js" -o src/vendor/sortable.min.js
# Then add to index.html before main.js:
# <script src="vendor/sortable.min.js"></script>
```

---

## Architecture Patterns

### Recommended File Structure
```
src/
├── main.js                    # Keep: recording, settings, Slack — NOT pipeline builder
├── pipeline-builder.js        # NEW: extracted pipeline builder module
├── integrations-settings.js   # Phase 4: already exists
├── vendor/
│   └── sortable.min.js        # NEW: SortableJS local vendor file
└── index.html                 # Add <script src="vendor/sortable.min.js"> before main.js
```

### Pattern 1: Module Extraction — What Moves to `pipeline-builder.js`

**What:** All pipeline-related state variables, DOM references, and functions currently in `main.js` (lines 1566–1991) move to `pipeline-builder.js`. The module loads after `main.js` via `<script src="pipeline-builder.js">`.

**State to extract:**
```javascript
// Currently in main.js — move to pipeline-builder.js
let allPipelineDefs = [];
let editingPipelineDef = null;
let pipelineEditorSteps = [];

// DOM refs:
const pipelineDefsListEl = document.getElementById('pipeline-defs-list');
// ... all pipeline-related getElementById calls
```

**Functions to extract:**
- `loadPipelineDefs()`
- `renderPipelineDefsList()`
- `openPipelineEditor(name)`
- `closePipelineEditor()`
- `renderPipelineSteps()` — will be the primary re-render function
- `fixStepInputs()`
- `getInputOptions(stepIndex)`
- `showStepEditor(index)` — will be heavily modified for Phase 5
- `renderPipelinePreview()`
- Event listeners for all pipeline editor buttons

**What stays in `main.js`:** `loadPipelineDefs()` call in `init()`, `updateSidebarCounts()` reference to `allPipelineDefs.length`.

**Load order in `index.html`:**
```html
<script src="vendor/sortable.min.js"></script>
<script src="main.js"></script>
<script src="integrations-settings.js"></script>
<script src="pipeline-builder.js"></script>
```

**Critical:** `pipeline-builder.js` loads last. It uses `escapeHtml`, `invoke`, `notionProfiles`, `savePathIntegrations`, `slackIntegrations` — all set up by earlier scripts. `allPipelineDefs` must be declared with `var` (not `let`) if `main.js` `init()` needs to read `allPipelineDefs.length` for sidebar counts. Alternatively, expose a `getPipelineCount()` function.

### Pattern 2: State-First Re-Render

**What:** `pipelineEditorSteps` is the single authoritative source of truth. Every mutation (add, remove, reorder, edit) updates the array first, then calls `renderPipelineSteps()` which destroys and rebuilds the DOM from scratch.

**Current state:** Already implemented. `renderPipelineSteps()` uses `innerHTML` assignment. `fixStepInputs()` is called before re-render on reorder and remove. This pattern must be preserved as-is when extracting to `pipeline-builder.js`.

**Anti-pattern to avoid:** Reading step data back from DOM (e.g., `querySelectorAll('[data-index]')`) to determine order. Always read from `pipelineEditorSteps`.

### Pattern 3: SortableJS Integration

**What:** Replace native HTML5 DnD handlers in `renderPipelineSteps()` with a single SortableJS initialization.

**When to use:** After `renderPipelineSteps()` builds the new DOM. SortableJS is initialized on `pipelineStepsListEl` after each full re-render (or once on module load if using `ghostClass` + `onEnd` callback only).

**Recommended approach:** Initialize SortableJS once, not on every re-render. SortableJS tracks DOM mutations. After `renderPipelineSteps()` replaces `innerHTML`, SortableJS on the container still works because it listens on the container, not individual items.

**Code pattern (from project research STACK.md):**
```javascript
// Initialize once after pipeline editor is shown, or on module load:
let sortableInstance = null;

function initSortable() {
  if (sortableInstance) sortableInstance.destroy();
  sortableInstance = new Sortable(pipelineStepsListEl, {
    animation: 150,
    handle: '.step-drag-handle',  // existing class
    ghostClass: 'pipeline-step-item--ghost',  // new CSS class needed
    onEnd: (evt) => {
      const { oldIndex, newIndex } = evt;
      if (oldIndex === newIndex) return;
      // Move in state array
      const [moved] = pipelineEditorSteps.splice(oldIndex, 1);
      pipelineEditorSteps.splice(newIndex, 0, moved);
      fixStepInputs();
      renderPipelineSteps();   // full re-render from state
      renderPipelinePreview();
    }
  });
}
```

**Important:** After `renderPipelineSteps()` clears `innerHTML`, existing SortableJS instance on the container continues to work — Sortable listens on the container, not child nodes. Re-initialization via `destroy()` + `new Sortable()` is only needed if the container itself changes. Initialize once when the pipeline editor opens.

**CSS class needed:**
```css
.pipeline-step-item--ghost {
  opacity: 0.4;
  background: var(--accent-soft);
  border: 1px dashed var(--accent);
}
```

### Pattern 4: Categorized Step Picker

**What:** Replace the single `+ Add Step` button with a picker that shows two sections: Processing (fixed presets) and Delivery (filtered to connected integrations).

**UI approach:** An inline dropdown/popover that appears below the `+ Add Step` button. Not a modal — it should be lightweight. Toggle visibility on button click, dismiss on outside click (add `document.addEventListener('click', dismissPicker)` with stopPropagation on the picker itself).

**Processing presets (hard-coded in JS, no Rust call needed):**
```javascript
const PROCESSING_PRESETS = [
  {
    label: 'Meeting Notes',
    step: {
      name: 'meeting-notes',
      connector: 'llm',
      input: 'transcript',  // fixStepInputs() will update
      config: { prompt_template: 'meeting-notes' },
      description: 'Extract attendees, decisions, action items'
    }
  },
  {
    label: 'Action Items',
    step: {
      name: 'action-items',
      connector: 'llm',
      input: 'transcript',
      config: { prompt_template: 'action-items' },  // new template needed in Rust
      description: 'Extract tasks and owners'
    }
  },
  {
    label: 'Summary',
    step: {
      name: 'summary',
      connector: 'llm',
      input: 'transcript',
      config: { prompt_template: 'summary' },  // new template needed in Rust
      description: 'Concise summary of key points'
    }
  },
  {
    label: 'Structure',
    step: {
      name: 'structure',
      connector: 'llm',
      input: 'transcript',
      config: { prompt_template: 'structure' },  // new template needed in Rust
      description: 'Organize content into sections'
    }
  },
  {
    label: 'Custom Prompt',
    step: null  // opens inline textarea form instead
  }
];
```

**Delivery section:** Generated dynamically from connected integrations. Show each Notion profile, save path, and Slack workspace as a delivery option. Read from `notionProfiles`, `savePathIntegrations`, `slackIntegrations` (accessible as globals from prior scripts).

**Adding a preset:** On click, deep-copy the preset step, call `fixStepInputs()` to set input correctly (step 0 gets `transcript`, others get previous step name), push to `pipelineEditorSteps`, call `renderPipelineSteps()` and `renderPipelinePreview()`. Dismiss picker.

### Pattern 5: Custom Prompt Step

**What:** Clicking "Custom Prompt" in the picker shows a small inline form: one textarea (the prompt text) and one checkbox ("Save as reusable template" with optional name input). No connector, model, or input dropdowns shown — these are Advanced and hidden by default.

**Form flow:**
1. User types prompt in textarea.
2. If "Save as reusable template" checked: show name input → on confirm, call `invoke('save_prompt_template', {...})` → set `step.config.prompt_template = savedName` → reload `allPromptTemplates`.
3. If not checked: set `step.config.prompt_inline = promptText` (new field — see Rust gap below).

**Rust gap — `prompt_inline` support:** The current validation in `pipelines.rs` (`validate_step_config`) requires `prompt_template` for LLM connector. The `pipeline_engine.rs` fetches the template by name. Two changes needed:
- `pipelines.rs`: Change LLM validation to accept EITHER `prompt_template` OR `prompt_inline` (not both required simultaneously).
- `pipeline_engine.rs`: In the LLM connector execution, if `config.prompt_inline` is set, use it directly instead of loading from template registry.

This is the only backend change in Phase 5.

### Pattern 6: Advanced Section (Provider/Model)

**What:** Collapse Provider and Model fields inside a `<details>` element within the step editor. Default state: closed.

**Current code:** `showStepEditor()` generates `configFields` inline HTML with Provider/Model for `llm` connector. Wrap those rows in:
```javascript
// In configFields for llm connector:
configFields = `
  <div class="step-editor-row"><label>Prompt</label><select data-field="prompt_template">...</select></div>
  <details class="step-editor-advanced">
    <summary>Advanced</summary>
    <div class="step-editor-row"><label>Provider</label><select data-field="provider">...</select></div>
    <div class="step-editor-row"><label>Model</label><input data-field="model" .../></div>
  </details>
`;
```

**CSS needed:**
```css
.step-editor-advanced {
  border-top: 1px solid var(--border);
  padding-top: 8px;
  margin-top: 4px;
}

.step-editor-advanced summary {
  font-size: 0.75rem;
  color: var(--text-secondary);
  cursor: pointer;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}
```

**Read-back:** The `Done` button handler uses `querySelectorAll('[data-field]')` to read all fields — this works inside `<details>` without change.

### Pattern 7: Assembly Preview Position and Enhancement

**What:** Move `#pipeline-preview` to below `#pipeline-steps-list` in the HTML (swap their order in `index.html`). Enhance `renderPipelinePreview()` to show delivery connector type.

**Current HTML order:** preview is above the steps list (inside `.editor-field` before the steps field). Requirements say preview should be below.

**Enhanced preview render:**
```javascript
function renderPipelinePreview() {
  if (!pipelinePreviewEl) return;
  if (pipelineEditorSteps.length === 0) {
    pipelinePreviewEl.innerHTML = '<span class="pipeline-preview-empty">Add steps to see preview</span>';
    return;
  }

  let html = '<span class="preview-node source">Transcript</span>';
  for (const step of pipelineEditorSteps) {
    const isDelivery = ['save', 'notion', 'slack', 'webhook', 'mcp'].includes(step.connector);
    const nodeClass = isDelivery ? 'preview-node delivery' : 'preview-node step';
    html += '<span class="preview-arrow">&rarr;</span>';
    html += `<span class="${nodeClass}">${escapeHtml(step.name || '?')}</span>`;
  }
  pipelinePreviewEl.innerHTML = html;
}
```

**CSS addition for delivery nodes:**
```css
.preview-node.delivery {
  background: rgba(16, 185, 129, 0.15);
  border: 1px solid rgba(16, 185, 129, 0.4);
  color: #10b981;
}
```

### Anti-Patterns to Avoid

- **Reading DOM for step order after SortableJS drag:** Always read from `pipelineEditorSteps`, never from `querySelectorAll('.pipeline-step-item')` positions. SortableJS's `onEnd` provides `oldIndex`/`newIndex` — use these to mutate the array, then re-render.
- **Re-initializing SortableJS on every `renderPipelineSteps()` call:** This causes memory leaks (each destroy/recreate cycle may leak event listeners). Initialize once when the editor opens; SortableJS on the container handles new child nodes.
- **`innerHTML` injection of preset data without `escapeHtml`:** Preset names are hard-coded constants (safe), but user-defined integration names (Notion workspace name, save path name) rendered in the Delivery picker must be escaped.
- **Setting `step.input` in presets to the previous step name before pushing:** `fixStepInputs()` handles this. Presets only need to set `input: 'transcript'` as a placeholder; `fixStepInputs()` corrects it.
- **Calling `invoke('save_prompt_template')` without reloading `allPromptTemplates`:** After saving an inline prompt as a template, call `await loadPromptTemplates()` so the new template appears in the template selector for subsequent steps.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Drag-and-drop reordering in WKWebView | Custom HTML5 DnD handlers | SortableJS | Native DnD events are swallowed or behave inconsistently in Tauri's WKWebView. Prior research confirmed this. |
| Collapsed "Advanced" accordion | JS click toggle + CSS animation | `<details>/<summary>` | Native HTML element, zero JS, keyboard accessible, no animation bugs |
| Inline picker popover positioning | JS getBoundingClientRect + absolute positioning | Simple `position: relative` container + `position: absolute` child | The builder is in a fixed-width panel; simple CSS absolute positioning is sufficient |

---

## Common Pitfalls

### Pitfall 1: SortableJS + `innerHTML` = indices desync

**What goes wrong:** After `renderPipelineSteps()` replaces `innerHTML`, if SortableJS's internal item list is out of sync, the `oldIndex`/`newIndex` values in `onEnd` can be stale or wrong.

**Why it happens:** SortableJS caches DOM nodes. If the container's children are replaced (not mutated) via `innerHTML`, SortableJS may still reference old nodes.

**How to avoid:** After every `renderPipelineSteps()` call that replaces `innerHTML`, call `sortableInstance.destroy()` and re-initialize SortableJS on the (now fresh) container. OR: build step items incrementally (appendChild) instead of `innerHTML` so SortableJS DOM references stay valid. The destroy/re-init approach is simpler given the existing `innerHTML` pattern.

**Correct flow:**
```javascript
function renderPipelineSteps() {
  // ... build innerHTML ...
  pipelineStepsListEl.innerHTML = stepsHtml;

  // Re-init Sortable after DOM replacement
  if (sortableInstance) sortableInstance.destroy();
  sortableInstance = new Sortable(pipelineStepsListEl, { ...sortableOptions });
}
```

**Warning signs:** Drag appears to work visually but wrong step gets moved in the array.

### Pitfall 2: `prompt_template` validation rejects `prompt_inline` steps

**What goes wrong:** User creates a Custom Prompt step without saving as a template. `invoke('save_pipeline')` fails with "LLM connector requires 'prompt_template' in config".

**Why it happens:** `validate_step_config()` in `pipelines.rs` enforces `prompt_template` presence for LLM connector without a fallback for inline prompts.

**How to avoid:** Update validation to accept either field:
```rust
ConnectorType::Llm => {
    let has_template = step.config.get("prompt_template").and_then(|v| v.as_str()).is_some();
    let has_inline = step.config.get("prompt_inline").and_then(|v| v.as_str()).is_some();
    if !has_template && !has_inline {
        return Err(format!(
            "Step '{}': LLM connector requires either 'prompt_template' or 'prompt_inline' in config",
            step.name
        ));
    }
}
```

**Warning signs:** Save pipeline fails for Custom Prompt steps; error message mentions `prompt_template`.

### Pitfall 3: Built-in preset templates not in the template registry

**What goes wrong:** User adds a "Meeting Notes" preset. The step has `config.prompt_template: 'meeting-notes'`. But `allPromptTemplates` (loaded from `list_prompt_templates`) does not include `meeting-notes` by its exact key — because the template key in the backend is `"meeting-notes"` but the display lookup in `showStepEditor()` searches `t.name`.

**Why it happens:** The backend stores templates in a `HashMap<String, PromptTemplate>` and `list_prompt_templates` returns them as a `Vec`. The order is non-deterministic. The step editor dropdown looks up by `t.name` — this must exactly match the `config.prompt_template` value.

**How to avoid:** Verify that the preset `config.prompt_template` values (`'meeting-notes'`, `'action-items'`, `'summary'`, `'structure'`) match exactly the `name` field of templates in the Rust registry. The existing `meeting-notes` template uses `name: "meeting-notes"` — confirmed match. The new templates (Action Items, Summary, Structure) must be added as built-in templates in `prompt_templates.rs` with the exact same names.

**Warning signs:** Step editor shows "Select template..." with no matching option selected even after adding a preset; pipeline execution fails with "Template 'action-items' not found".

### Pitfall 4: Global variable access order in multi-script setup

**What goes wrong:** `pipeline-builder.js` references `notionProfiles` or `savePathIntegrations` at module parse time (top-level code). These are declared with `var` in `integrations-settings.js`, which loads after `pipeline-builder.js` per the current load order.

**Why it happens:** Script load order in `index.html` determines when globals are available.

**How to avoid:** `pipeline-builder.js` must load LAST in `index.html`:
```html
<script src="vendor/sortable.min.js"></script>
<script src="main.js"></script>
<script src="integrations-settings.js"></script>
<script src="pipeline-builder.js"></script>
```

Access to `notionProfiles`, `savePathIntegrations`, `slackIntegrations` must only happen inside functions (not at top-level code), using `typeof` guards where needed (following the Phase 4 precedent in `main.js`):
```javascript
const profiles = (typeof notionProfiles !== 'undefined') ? notionProfiles : [];
```

**Warning signs:** `ReferenceError: notionProfiles is not defined` on page load; Delivery picker shows no integrations.

### Pitfall 5: Step picker popover dismissal interferes with picker clicks

**What goes wrong:** Clicking a preset inside the picker triggers the outside-click dismiss handler before the preset click handler runs. Result: picker closes but no step is added.

**Why it happens:** Event bubbling — `document.click` fires before the picker item's handler if not properly `stopPropagation`'d.

**How to avoid:** In the outside-click dismiss handler, check if the click target is inside the picker:
```javascript
document.addEventListener('click', (e) => {
  if (!pickerEl.contains(e.target) && e.target !== addPipelineStepBtn) {
    closePicker();
  }
});
```
And use `e.stopPropagation()` on the picker container click to prevent double-triggering.

**Warning signs:** Clicking a preset closes the picker but does not add a step.

---

## Code Examples

### SortableJS Initialization (from STACK.md, verified pattern)
```javascript
// Source: SortableJS GitHub + project STACK.md research
let sortableInstance = null;

function renderPipelineSteps() {
  // ... build stepsHtml string ...
  pipelineStepsListEl.innerHTML = stepsHtml;

  // Re-init after innerHTML replacement
  if (sortableInstance) sortableInstance.destroy();
  sortableInstance = new Sortable(pipelineStepsListEl, {
    animation: 150,
    handle: '.step-drag-handle',
    ghostClass: 'pipeline-step-item--ghost',
    onEnd(evt) {
      if (evt.oldIndex === evt.newIndex) return;
      const [moved] = pipelineEditorSteps.splice(evt.oldIndex, 1);
      pipelineEditorSteps.splice(evt.newIndex, 0, moved);
      fixStepInputs();
      renderPipelineSteps();
      renderPipelinePreview();
    }
  });
}
```

### Preset Step Add (zero-fields pattern)
```javascript
// Source: derived from existing pipelineEditorSteps.push pattern in main.js
function addPresetStep(preset) {
  const step = JSON.parse(JSON.stringify(preset.step)); // deep copy
  pipelineEditorSteps.push(step);
  fixStepInputs(); // sets input to previous step or 'transcript'
  renderPipelineSteps();
  renderPipelinePreview();
  closePicker();
}
```

### Custom Prompt Inline Form
```javascript
// Rendered as a transient inline form (not a full step-editor replacement):
function showCustomPromptForm() {
  const formEl = document.createElement('div');
  formEl.className = 'custom-prompt-form';
  formEl.innerHTML = `
    <textarea class="custom-prompt-textarea" rows="4"
      placeholder="Write your prompt here. Use {transcript} for the transcript text."></textarea>
    <label class="custom-prompt-save-label">
      <input type="checkbox" class="custom-prompt-save-checkbox" />
      Save as reusable template
    </label>
    <div class="custom-prompt-name-row" style="display:none;">
      <input type="text" class="custom-prompt-name-input" placeholder="Template name" />
    </div>
    <div class="custom-prompt-actions">
      <button class="mini-action-btn custom-prompt-cancel">Cancel</button>
      <button class="mini-action-btn primary custom-prompt-confirm">Add Step</button>
    </div>
  `;
  // ... attach event listeners, show checkbox toggle, confirm handler
  pipelineStepsListEl.appendChild(formEl);
}
```

### Built-in Template Addition (Rust — `prompt_templates.rs`)
```rust
// Add to get_builtin_templates() in src-tauri/src/prompt_templates.rs:
templates.insert(
    "action-items".to_string(),
    PromptTemplate {
        name: "action-items".to_string(),
        description: "Extract action items, tasks, and owners from the conversation".to_string(),
        prompt: r#"Extract all action items from this transcript.

For each action item, identify:
1. **Task**: What needs to be done
2. **Owner**: Who is responsible (if mentioned)
3. **Due Date**: When it's due (if mentioned)
4. **Priority**: High/Medium/Low (infer from context)

Format as a clean Markdown checklist.

Transcript:
{transcript}"#.to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
    },
);
// Similarly for "summary" and "structure"
```

### `prompt_inline` Validation Update (Rust)
```rust
// In src-tauri/src/pipelines.rs, validate_step_config():
ConnectorType::Llm => {
    let has_template = step.config.get("prompt_template")
        .and_then(|v| v.as_str())
        .is_some();
    let has_inline = step.config.get("prompt_inline")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .is_some();
    if !has_template && !has_inline {
        return Err(format!(
            "Step '{}': LLM connector requires 'prompt_template' or 'prompt_inline'",
            step.name
        ));
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Native HTML5 DnD (`draggable="true"`) | SortableJS vendor file | Phase 5 | Reliable drag in macOS WKWebView |
| `+ Add Step` blank step with full editor | Categorized picker + preset steps | Phase 5 | Zero-friction step addition |
| Provider/Model always visible | Collapsed in `<details>` Advanced | Phase 5 | Cleaner default step view |
| Pipeline preview above steps | Preview below steps | Phase 5 | Visual flow matches top-to-bottom mental model |

**Deprecated approach to remove:**
- Native HTML5 DnD event listeners (`dragstart`, `dragover`, `dragend`, `dragleave`, `drop`) in `renderPipelineSteps()` — replace entirely with SortableJS `onEnd` callback.

---

## Open Questions

1. **`prompt_inline` pipeline execution path**
   - What we know: `pipeline_engine.rs` calls `connectors::llm::run()` which loads template by name from registry. The `prompt_inline` field would bypass the registry.
   - What's unclear: Whether `pipeline_engine.rs` constructs the prompt itself (before passing to connector) or whether the connector loads the template.
   - Recommendation: Read `pipeline_engine.rs` execution path before implementing. If prompt loading happens in the connector, add `prompt_inline` fallback there. If in the engine, add it there.

2. **New built-in templates (`action-items`, `summary`, `structure`) migration**
   - What we know: `get_builtin_templates()` is called only when `prompt-templates.json` does not exist (first run). Existing users already have `prompt-templates.json` without these templates.
   - What's unclear: Whether new built-ins should be injected into existing installs.
   - Recommendation: Add a `get_builtin_templates()` call in `load_prompt_templates()` that merges missing built-ins into existing templates on load (without overwriting user-modified ones). This is the same pattern used for the existing migration.

3. **Delivery picker for integrations with no connected instances**
   - What we know: If no Notion profiles, no save paths, and no Slack integrations are connected, the Delivery section is empty.
   - What's unclear: UX when Delivery section is empty — show a message or hide the section?
   - Recommendation: Show a placeholder message "Connect integrations in Settings → Integrations to enable delivery steps." Do not hide the section.

---

## Sources

### Primary (HIGH confidence)
- `/workspace/src/main.js` lines 1566–1991 — Direct analysis of existing pipeline builder code; state variables, render functions, DnD handlers all confirmed
- `/workspace/src-tauri/src/pipelines.rs` — `PipelineStep` struct, `validate_step_config()`, `ConnectorType` enum — all confirmed
- `/workspace/src-tauri/src/prompt_templates.rs` — `PromptTemplate` struct, `get_builtin_templates()`, validation logic — confirmed
- `/workspace/src-tauri/src/lib.rs` — Registered Tauri commands confirmed: `save_prompt_template`, `list_prompt_templates`, `list_pipelines`, `save_pipeline`
- `/workspace/src/integrations-settings.js` — `notionProfiles`, `savePathIntegrations`, `_slackIntegrations` declared with `var`; confirmed accessible as globals
- `/workspace/.planning/research/STACK.md` — SortableJS decision rationale, WKWebView DnD unreliability, vendor file approach — HIGH confidence from prior research

### Secondary (MEDIUM confidence)
- SortableJS GitHub (SortableJS/Sortable) — `onEnd` callback with `oldIndex`/`newIndex`, `handle` option, `ghostClass` option, `animation` — confirmed from STACK.md research (WebView behavior on specific macOS versions not independently retested)
- `/workspace/.planning/research/SUMMARY.md` — Phase 5 architecture overview and pitfall notes
- HTML `<details>/<summary>` spec — native browser support confirmed; behavior in WKWebView (Tauri) aligns with WebKit standard — MEDIUM confidence (not specifically tested in this app's WKWebView)

### Tertiary (LOW confidence)
- SortableJS `destroy()` + re-init pattern after `innerHTML` replacement — inferred from SortableJS internals and common practice; not independently verified against SortableJS 1.15.x source
- `<details>` styling resets in WKWebView — assumed to work; should be manually tested

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — SortableJS vendor file approach and load order confirmed from prior research and codebase analysis; no new libraries
- Architecture: HIGH — Existing code directly analyzed; extraction targets and module boundaries clearly defined; all Rust types confirmed
- Pitfalls: HIGH — Most pitfalls are directly observable in existing code patterns (innerHTML, global variable access, SortableJS lifecycle); Custom Prompt validation gap confirmed by reading `validate_step_config()` directly

**Research date:** 2026-02-18
**Valid until:** 2026-03-18 (30 days — stable stack, no fast-moving dependencies)
