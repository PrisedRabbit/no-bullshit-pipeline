# Phase 6: Pre-Assignment UX and Execution - Research

**Researched:** 2026-02-19
**Domain:** Tauri frontend UX (chip bar, overflow popover, persistence), recording lifecycle integration, pipeline execution wiring
**Confidence:** HIGH

## Summary

Phase 6 is primarily a frontend-heavy phase that wires together fully-implemented backend capabilities that already exist. The Rust backend already has `execute_pipeline`, `assign_pipeline`, `get_all_pipeline_states`, `get_step_outputs`, and `transcribe_recording` as registered Tauri commands. The pipeline engine already emits `pipeline-progress` events. The recording lifecycle (`start_recording`, `stop_recording`) already exists. The work is almost entirely:

1. **06-01**: Adding chip UI to `main.js` (render chips from `allPipelineDefs`, `startRecordingWithPipeline()`, 5-chip cap + overflow popover, chips active during recording)
2. **06-02**: Persisting last-used pipeline and default pipeline in `AppSettings`; adding post-recording pipeline assignment in detail view
3. **06-03**: Auto-transcribe + auto-execute pipeline chain after `stop_recording`; surfacing pipeline run status in detail view with step-level error details

No new Rust infrastructure is needed for ASGN-01 through EXEC-03. The only Rust additions are small: adding `default_pipeline` and `last_used_pipeline` to `AppSettings`, and potentially a command to auto-execute after stop. The UI patterns (chip bar, overflow popover) are standard CSS/JS patterns with no library needed.

**Primary recommendation:** Implement the three plans in strict order. 06-01 is pure JS/CSS UI. 06-02 requires a small `AppSettings` struct change + JS persistence. 06-03 requires wiring the stop-recording callback to trigger transcription then pipeline execution, plus rendering pipeline run status in the detail view using already-registered commands.

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| ASGN-01 | Pipeline chips appear in the app bar next to the record button | Chip bar implemented in `main.js` capture-section; chips rendered from `allPipelineDefs` (already global via `var` from `pipeline-builder.js`); styles added to `styles.css` |
| ASGN-02 | Clicking a pipeline chip starts recording immediately with that pipeline pre-assigned | New `startRecordingWithPipeline(pipelineName)` function in `main.js`; calls `start_recording` then `assign_pipeline` (already registered Tauri command); no new Rust needed |
| ASGN-03 | Pipeline chips remain active during recording for mid-recording assignment | Chip `onclick` calls `assign_pipeline` directly when `isRecording`; button not disabled during recording; CSS class to visually dim unselected chips while recording active |
| ASGN-04 | User can assign/change pipeline after recording in the detail view | Pipeline selector dropdown or chips rendered inside detail view; calls `assign_pipeline` from JS; Rust `assign_pipeline` command already handles this |
| ASGN-05 | Default pipeline setting in Settings applies to all new recordings unless overridden | New `default_pipeline: Option<String>` field in `AppSettings` struct; Settings > Audio tab gets a pipeline select dropdown; auto-select on `startRecording()` if no chip was clicked |
| ASGN-06 | Last-used pipeline is remembered and highlighted on next app launch | New `last_used_pipeline: Option<String>` field in `AppSettings`; written to settings after each recording; chip with matching name gets `.is-last-used` CSS class on render |
| ASGN-07 | Chip bar shows top N pipelines with overflow menu for additional pipelines | Cap at 5 chips in chip bar; a "+" or "..." button shows a dropdown popover with remaining pipelines when `allPipelineDefs.length > 5` |
| EXEC-01 | After recording stops, auto-transcribe followed by auto-pipeline execution with no user action | `stopRecording()` in `main.js` checks if transcription enabled and if a pipeline is assigned; chains `transcribe_recording` then `execute_pipeline`; error in either step is non-blocking (shown inline) |
| EXEC-02 | Pipeline run status per recording visible in recording detail view (Waiting/Running/Done/Failed) | `showDetailView()` calls `get_all_pipeline_states` to load current pipeline states; renders status badges next to pipeline names; subscribes to `pipeline-progress` event for live updates |
| EXEC-03 | Failed pipeline step shows inline error with the specific step that failed and why | On `status === 'partial'`, calls `get_step_outputs` to find the failed step; `StepStatus.error` field contains the error message; renders inline below the pipeline status badge |
</phase_requirements>

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Vanilla JS | ES2022 | Chip bar, overflow popover, auto-execution wiring | No bundler in this project; consistent with all other frontend code |
| CSS custom properties | n/a | Chip bar theming (accent color, borders) | Already used throughout `styles.css` for all components |
| Tauri events (`window.__TAURI__.event.listen`) | 2.x | Live `pipeline-progress` event subscription | Already used in `main.js` for `transcription_segment` and `recording_warning` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| SortableJS | already vendored | Drag-and-drop | Already loaded but NOT needed for Phase 6 |
| localStorage | browser built-in | NOT to be used | Settings must go through Tauri `save_settings`/`load_settings` commands for persistence across app restarts |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Inline popover div (JS) | Native `<details>` element | `<details>` cannot be positioned absolutely; popover div gives full control over placement |
| AppSettings struct change | localStorage | AppSettings persists in `~/.nbp/settings.json`; localStorage is ephemeral across Tauri rebuilds and doesn't work reliably |

---

## Architecture Patterns

### Recommended Project Structure

Phase 6 modifies only existing files — no new files needed:

```
src/
├── main.js              # chip bar, startRecordingWithPipeline(), overflow popover, auto-exec wiring
├── styles.css           # .pipeline-chip, .pipeline-chip-bar, .chip-overflow-btn, .pipeline-status-badge
└── index.html           # chip bar HTML (injected into capture-section next to record button)

src-tauri/src/
└── config.rs            # AppSettings: add default_pipeline, last_used_pipeline fields
```

### Pattern 1: Pipeline Chip Bar

**What:** A row of pill-shaped buttons inserted into `#capture-section` (the header area next to the record button). Each chip maps to one `Pipeline` definition from `allPipelineDefs`. Clicking a chip when not recording calls `startRecordingWithPipeline(chip.dataset.pipelineName)`. Clicking a chip when recording calls `assign_pipeline`.

**When to use:** Always rendered; hidden via CSS when settings view or detail view is open.

**Example:**
```javascript
// Render at most 5 chips; add overflow button if more exist
function renderPipelineChips() {
  const chipBar = document.getElementById('pipeline-chip-bar');
  if (!chipBar) return;

  const MAX_CHIPS = 5;
  const visible = allPipelineDefs.slice(0, MAX_CHIPS);
  const overflow = allPipelineDefs.slice(MAX_CHIPS);

  chipBar.innerHTML = visible.map(p => {
    const isLastUsed = appSettings?.last_used_pipeline === p.name;
    return `<button class="pipeline-chip${isLastUsed ? ' is-last-used' : ''}"
      data-pipeline-name="${escapeHtml(p.name)}"
      title="${escapeHtml(p.description || p.name)}">
      ${escapeHtml(p.name)}
    </button>`;
  }).join('');

  if (overflow.length > 0) {
    chipBar.innerHTML += `<button class="chip-overflow-btn" id="chip-overflow-btn"
      title="${overflow.length} more pipelines">+${overflow.length}</button>`;
  }

  chipBar.querySelectorAll('.pipeline-chip').forEach(chip => {
    chip.addEventListener('click', () => handleChipClick(chip.dataset.pipelineName));
  });
}

async function handleChipClick(pipelineName) {
  if (isRecordingBusy) return;
  if (isRecording) {
    // Mid-recording assignment
    await invoke('assign_pipeline', { recordingId: selectedRecordingId, pipelineName });
    currentAssignedPipeline = pipelineName;
  } else {
    await startRecordingWithPipeline(pipelineName);
  }
}
```

### Pattern 2: startRecordingWithPipeline

**What:** Extends `startRecording()` with a pipeline assignment immediately after the recording is created. The recording ID comes back from `start_recording`. Then `assign_pipeline` is called with that ID.

**Example:**
```javascript
async function startRecordingWithPipeline(pipelineName) {
  isRecordingBusy = true;
  ViewManager.showRecordings();
  const saveMixOnly = appSettings?.save_mix_only !== false;
  try {
    const metadata = await invoke('start_recording', { saveMixOnly });
    isRecording = true;
    currentAssignedPipeline = pipelineName;

    // Assign pipeline immediately after recording starts
    await invoke('assign_pipeline', {
      recordingId: metadata.id,
      pipelineName
    });

    // Save last-used pipeline
    appSettings.last_used_pipeline = pipelineName;
    await invoke('save_settings', { settings: appSettings });

    setRecordingUI(true);
    await loadRecordings();
    startTimer();
    startWaveformAnimation();
    showDetailView(metadata.id);
  } catch (error) {
    isRecording = false;
    stopTimer();
    stopWaveformAnimation();
    setRecordingUI(false);
    console.error('Failed to start recording with pipeline:', error);
    alert('Failed to start: ' + error);
  } finally {
    isRecordingBusy = false;
  }
}
```

### Pattern 3: Auto-Execute on Stop

**What:** After `stop_recording` completes, if the recording has an assigned pipeline AND transcription is enabled, run `transcribe_recording` then `execute_pipeline` automatically. All errors are surfaced inline, never blocking the user.

**Example:**
```javascript
async function stopRecording() {
  // ... existing stop logic ...
  await invoke('stop_recording');
  isRecording = false;
  // ... timer/UI cleanup ...

  // Auto-execute pipeline chain (non-blocking — do not await the outer stopRecording)
  if (currentAssignedPipeline && appSettings?.transcription?.enabled) {
    const recordingId = currentId;
    const pipelineName = currentAssignedPipeline;
    currentAssignedPipeline = null;

    // Fire and forget — errors handled inside
    autoTranscribeAndExecute(recordingId, pipelineName);
  }
}

async function autoTranscribeAndExecute(recordingId, pipelineName) {
  try {
    if (window.__NBP_setTranscribingId) window.__NBP_setTranscribingId(recordingId);
    await invoke('transcribe_recording', { recordingId });
  } catch (err) {
    console.error('Auto-transcription failed:', err);
    // Show inline error in detail view if still open
    return; // Do not proceed to pipeline execution if transcription failed
  }

  try {
    await invoke('execute_pipeline', { recordingId, pipelineName });
    await loadRecordings();
    if (selectedRecordingId === recordingId) showDetailView(recordingId);
  } catch (err) {
    console.error('Auto-pipeline execution failed:', err);
  }
}
```

### Pattern 4: Pipeline Run Status in Detail View

**What:** `showDetailView()` loads `get_all_pipeline_states` after loading the transcript. Status is rendered as a small badge per pipeline. The `pipeline-progress` event updates badges live. If status is `partial`, `get_step_outputs` is called to show the failed step.

**Example:**
```javascript
async function renderPipelineStatus(recordingId) {
  const statusEl = document.getElementById('pipeline-status-section');
  if (!statusEl) return;

  const states = await invoke('get_all_pipeline_states', { recordingId });
  if (!states || states.length === 0) {
    statusEl.innerHTML = '';
    return;
  }

  statusEl.innerHTML = await Promise.all(states.map(async state => {
    let errorHtml = '';
    if (state.status === 'partial' && state.error) {
      // Get step-level details
      try {
        const steps = await invoke('get_step_outputs', { recordingId, pipelineName: state.name });
        const failedStep = steps.find(s => s.status === 'failed');
        if (failedStep) {
          errorHtml = `<div class="pipeline-step-error">
            Step "${escapeHtml(failedStep.name)}" failed: ${escapeHtml(failedStep.error || '')}
          </div>`;
        }
      } catch (e) { /* best-effort */ }
    }

    return `<div class="pipeline-status-row">
      <span class="pipeline-status-name">${escapeHtml(state.name)}</span>
      <span class="pipeline-status-badge status-${state.status}">${state.status}</span>
      ${errorHtml}
    </div>`;
  })).then(parts => parts.join(''));
}
```

### Anti-Patterns to Avoid

- **Do not use localStorage for pipeline assignments or last-used state.** `localStorage` does not survive Tauri app reinstallation and can get out of sync with `settings.json`. Use `save_settings`/`load_settings`.
- **Do not block recording stop on pipeline execution.** Auto-exec must be fire-and-forget. Blocking stop would make the app feel frozen.
- **Do not disable chips during recording.** ASGN-03 requires chips remain interactive. Instead, change the chip click behavior based on `isRecording`.
- **Do not use `innerHTML` assignment for the overflow popover if it contains user-supplied data without `escapeHtml`.** All pipeline names must be escaped.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Overflow popover positioning | Custom positioning math | Simple absolute CSS with `position: absolute; top: 100%; left: 0` on a container with `position: relative` | The app bar is fixed-height; no complex viewport math needed |
| Pipeline status polling | `setInterval` polling for status | `pipeline-progress` Tauri event subscription | Event already emitted by `pipeline_engine.rs` on each step; no polling needed |
| Pipeline assignment storage | Custom metadata field | Existing `assign_pipeline` Tauri command + `PipelineState.Waiting` status | Backend already handles this; writing directly to metadata would bypass locking |

**Key insight:** Almost everything in this phase is wiring existing capabilities. The chip bar is the most novel UI component, but it follows the same pattern as other button rows in the app bar.

---

## Common Pitfalls

### Pitfall 1: allPipelineDefs not loaded before renderPipelineChips

**What goes wrong:** `renderPipelineChips()` is called before `loadPipelineDefs()` finishes in `init()`, resulting in an empty chip bar.
**Why it happens:** `init()` calls `loadPipelineDefs()` but chip rendering is triggered synchronously during init.
**How to avoid:** Call `renderPipelineChips()` inside `loadPipelineDefs()` after `allPipelineDefs` is populated — just like `updateSidebarCounts()` is already called there.
**Warning signs:** Chip bar renders empty on first load; populated after navigating away and back.

### Pitfall 2: currentAssignedPipeline reset race condition

**What goes wrong:** User stops recording; auto-exec starts; user immediately starts a new recording. `currentAssignedPipeline` is cleared for the old recording but the new recording doesn't have it set correctly.
**Why it happens:** `currentAssignedPipeline` is a single global variable, not per-recording-session.
**How to avoid:** Capture `currentAssignedPipeline` into a local variable at the start of `stopRecording()` and pass it to `autoTranscribeAndExecute()`. Clear the global immediately after capture.

### Pitfall 3: pipeline-progress event handler leaks

**What goes wrong:** Each call to `showDetailView()` adds a new `pipeline-progress` listener without removing the old one, causing multiple handlers to fire for each event.
**Why it happens:** Tauri's `event.listen` returns an unlisten function but it must be called explicitly.
**How to avoid:** Store the unlisten function in a module-level variable. Call it before registering a new listener in `showDetailView()`.
```javascript
let pipelineProgressUnlisten = null;

async function subscribeToProgress(recordingId) {
  if (pipelineProgressUnlisten) {
    pipelineProgressUnlisten();
    pipelineProgressUnlisten = null;
  }
  pipelineProgressUnlisten = await window.__TAURI__.event.listen('pipeline-progress', (event) => {
    if (event.payload.recording_id !== recordingId) return;
    // Update status badge for event.payload.pipeline_name
    renderPipelineStatus(recordingId);
  });
}
```

### Pitfall 4: Overflow popover not dismissed on outside click

**What goes wrong:** Overflow popover stays open when user clicks elsewhere in the app.
**Why it happens:** No document-level dismiss listener added.
**How to avoid:** Add a `setTimeout(() => document.addEventListener('click', dismissOverflow), 0)` pattern — same pattern already used for `dismissPicker` in `pipeline-builder.js`.

### Pitfall 5: AppSettings struct not handling missing fields gracefully

**What goes wrong:** Adding `default_pipeline` and `last_used_pipeline` to Rust `AppSettings` without `#[serde(default)]` causes existing settings.json files (without these fields) to fail to deserialize.
**Why it happens:** Serde's default behavior rejects unknown/missing fields unless configured otherwise.
**How to avoid:** Add `#[serde(default)]` to new `Option<String>` fields:
```rust
#[serde(default)]
pub default_pipeline: Option<String>,
#[serde(default)]
pub last_used_pipeline: Option<String>,
```

### Pitfall 6: Auto-execute when transcription is disabled

**What goes wrong:** Auto-execute runs pipeline on a recording with no transcript, causing `execute_pipeline_internal` to return "No transcript found" error.
**Why it happens:** `EXEC-01` says auto-transcribe followed by auto-execute; if transcription is disabled, there's no transcript.
**How to avoid:** Gate the auto-exec chain: only run if `appSettings.transcription.enabled === true`. If disabled but pipeline was assigned, the recording will have `Waiting` status — the user can manually transcribe later, then run the pipeline from detail view.

---

## Code Examples

### AppSettings Changes (config.rs)

```rust
// Source: direct codebase analysis of src-tauri/src/config.rs
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    // ... existing fields ...
    #[serde(default)]
    pub default_pipeline: Option<String>,
    #[serde(default)]
    pub last_used_pipeline: Option<String>,
}
```

### HTML Chip Bar (index.html insertion point)

The `#capture-section` in `index.html` currently contains:
- `#status-indicator`
- `#timer`
- `#recording-waveform`
- `.capture-controls` (containing `#record-toggle-btn`)

The chip bar inserts **between `#recording-waveform` and `.capture-controls`** or after `.capture-controls`. Order matters for layout. Based on the requirement "next to the record button," insert immediately before the record button:

```html
<!-- Pipeline chip bar: rendered dynamically by JS -->
<div id="pipeline-chip-bar" class="pipeline-chip-bar">
  <!-- Chips injected by renderPipelineChips() -->
</div>
```

### CSS Chip Styles

```css
.pipeline-chip-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: nowrap;
  overflow: visible;
}

.pipeline-chip {
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  border-radius: 999px;
  padding: 3px 10px;
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
}

.pipeline-chip:hover {
  border-color: var(--accent);
  color: var(--text-primary);
}

.pipeline-chip.is-last-used {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

.chip-overflow-btn {
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  border-radius: 999px;
  padding: 3px 8px;
  font-size: 0.75rem;
  cursor: pointer;
}

.pipeline-status-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
}

.pipeline-status-badge {
  font-size: 0.7rem;
  font-weight: 600;
  border-radius: 4px;
  padding: 2px 6px;
  text-transform: uppercase;
}

.pipeline-status-badge.status-waiting { background: var(--bg-card); color: var(--text-secondary); }
.pipeline-status-badge.status-running { background: var(--accent-soft); color: var(--accent); }
.pipeline-status-badge.status-done { background: rgba(16, 185, 129, 0.15); color: var(--success); }
.pipeline-status-badge.status-partial { background: rgba(248, 113, 113, 0.15); color: var(--danger); }

.pipeline-step-error {
  font-size: 0.75rem;
  color: var(--danger);
  margin-top: 4px;
  padding: 4px 8px;
  background: rgba(248, 113, 113, 0.08);
  border-radius: 4px;
  border-left: 2px solid var(--danger);
}
```

### Existing Tauri Commands Available (no new Rust needed for core ASGN/EXEC)

```javascript
// All already registered in lib.rs:
await invoke('assign_pipeline', { recordingId, pipelineName });            // ASGN-02, ASGN-03, ASGN-04
await invoke('get_all_pipeline_states', { recordingId });                   // EXEC-02
await invoke('get_step_outputs', { recordingId, pipelineName });            // EXEC-03
await invoke('execute_pipeline', { recordingId, pipelineName });            // EXEC-01
await invoke('transcribe_recording', { recordingId });                      // EXEC-01

// Pipeline state values from Rust PipelineStatus enum:
// 'waiting', 'running', 'done', 'partial'  (serde lowercase)
// Note: roadmap says "Failed" but Rust uses "Partial" — the UI should display "Failed" text
// when status is "partial" to match the success criteria language

// pipeline-progress event payload (from PipelineProgressPayload):
// { recording_id, pipeline_name, step_name, step_index, total_steps, status }
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual pipeline assignment via Settings nav | Pipeline chips in app bar | Phase 6 | Single-click start with pre-assigned pipeline |
| Manual transcribe → manual execute | Auto-chain on stop | Phase 6 | Zero-step post-recording workflow |
| No pipeline run visibility | Status badges in detail view | Phase 6 | User can see what's happening |

**Key naming inconsistency to resolve:**
- Rust `PipelineStatus` uses `Partial` for a pipeline that stopped due to step failure
- Success criteria EXEC-02 says "Failed" as a display state
- Resolution: In the UI, render `partial` status as "Failed" text to match user expectation. The underlying storage value remains `partial` — do not change the Rust enum.

---

## Open Questions

1. **Default pipeline UX in Settings**
   - What we know: ASGN-05 requires a "Default pipeline" setting in Settings > General
   - What's unclear: Which settings tab — the roadmap says "Settings > General" but the app has Audio/Integrations/Templates/Pipelines/Theme tabs. No "General" tab exists.
   - Recommendation: Add to the **Audio** tab (most general tab) under a new "Recording" section, or add a new "General" tab. Planner should decide.

2. **Overflow popover style: modal vs. inline dropdown**
   - What we know: "overflow control for additional pipelines" is the requirement
   - What's unclear: Should it be a floating dropdown (positioned below the `+N` button) or a modal?
   - Recommendation: Floating dropdown for minimal disruption — same pattern as the step picker in `pipeline-builder.js`.

3. **Auto-execute when transcription is disabled but a pipeline is assigned**
   - What we know: EXEC-01 says auto-transcribe then auto-execute; transcription may be disabled
   - What's unclear: Should the pipeline be queued in `Waiting` state silently, or should the user be notified that they must manually transcribe first?
   - Recommendation: Leave pipeline in `Waiting` state silently. Show `Waiting` badge in detail view as implicit cue.

4. **ASGN-04: Detail view pipeline assignment UI**
   - What we know: User can assign/change pipeline after recording in detail view
   - What's unclear: Is this a dropdown select? A chip-like list? A "Run with pipeline" button?
   - Recommendation: Render a compact dropdown showing all pipelines. Selecting assigns it via `assign_pipeline` and updates the status display.

5. **"status-partial" vs "status-failed" badge CSS class**
   - What we know: Rust uses `partial`, success criteria says "Failed"
   - Recommendation: Use CSS class `.status-partial` (matching backend value) but display text "Failed". Avoids backend change.

---

## Sources

### Primary (HIGH confidence)
- Direct codebase reading: `src-tauri/src/pipeline_engine.rs` — `execute_pipeline`, `assign_pipeline`, `get_all_pipeline_states`, `get_step_outputs`, `PipelineStatus` enum, `PipelineProgressPayload`, `pipeline-progress` event
- Direct codebase reading: `src-tauri/src/config.rs` — `AppSettings` struct, serde defaults pattern
- Direct codebase reading: `src-tauri/src/lib.rs` — registered Tauri commands (confirmation of what's already available)
- Direct codebase reading: `src/main.js` — `startRecording()`, `stopRecording()`, `showDetailView()`, `loadPipelineDefs()` placement in `init()`, `isRecordingBusy` guard pattern, event listener pattern
- Direct codebase reading: `src/pipeline-builder.js` — `allPipelineDefs` global (`var`), `dismissPicker` pattern for outside-click dismissal
- Direct codebase reading: `src/index.html` — `#capture-section` structure, script load order
- Direct codebase reading: `src/styles.css` — CSS variable names, existing chip/badge patterns

### Secondary (MEDIUM confidence)
- Pattern inference: `transcription_segment` event listener in `main.js` used as model for `pipeline-progress` event listener with unlisten cleanup pattern

### Tertiary (LOW confidence)
- None — all claims verified from codebase

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no external libraries needed; all patterns from existing codebase
- Architecture: HIGH — all backend commands verified as registered; event names verified from Rust source
- Pitfalls: HIGH — derived from direct code analysis (serde defaults, event listener leaks, race conditions)

**Research date:** 2026-02-19
**Valid until:** 2026-03-19 (stable codebase; no fast-moving dependencies)
