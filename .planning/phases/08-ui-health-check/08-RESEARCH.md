# Phase 8: UI Health Check - Research

**Researched:** 2026-02-19
**Domain:** DOM audit, startup sequencing, interactive walkthrough, Vanilla JS Tauri frontend
**Confidence:** HIGH

## Summary

Phase 8 is a pure frontend phase — no new Rust commands are needed. The health check is a new JS module (`ui-health-check.js`) that runs after `init()` completes in `main.js`, uses `requestIdleCallback` to defer work, audits the DOM for all v2 interactive elements, and writes a small status badge into the app bar. The badge is always visible (not hidden by view state logic). Clicking the badge when failures exist shows a report overlay. A guided walkthrough (HLTH-04) is an in-app step overlay that highlights real DOM elements using `getBoundingClientRect`, triggered on first launch (via a settings flag `walkthrough_completed`) and on demand from Settings.

The codebase follows a strict state-first, full re-render, vanilla JS pattern. All modules are static script files loaded in order: `sortable.min.js → main.js → integrations-settings.js → pipeline-builder.js`. The new `ui-health-check.js` should load last (after `pipeline-builder.js`) so all globals (`allPipelineDefs`, `notionProfiles`, `savePathIntegrations`, `escapeHtml`, `invoke`) are available.

Two critical constraints shape the implementation: (1) the capture-section and detail-controls are hidden by CSS body class logic (`.detail-open`, `.settings-open`, `.is-recording-active`), so the health badge must be placed outside those sections — the `app-logo` area or as a persistent sibling in the app bar; (2) the integrations tab content is lazy-loaded via MutationObserver on first activation, meaning elements inside `#connected-integrations-list` and `#available-integrations-list` are not present at startup audit time and must be audited separately or skipped in the silent startup audit.

**Primary recommendation:** Place the health badge as a `<div id="health-badge">` immediately after `.app-logo` in `index.html`. Implement `ui-health-check.js` as a self-contained module with `runAudit()`, `renderBadge()`, and `showReport()`. Wire the walkthrough as a step-overlay driven by an array of `{ selector, title, description }` step descriptors, creating a semi-transparent overlay with a spotlight hole around the target element. First-launch trigger: add `walkthrough_completed: false` to `AppSettings` (with `#[serde(default)]`), check it in `init()` after the audit, and run walkthrough automatically if false. On-demand: add a "UI Walkthrough" button to the Audio settings tab (most generic tab).

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| HLTH-01 | Automated DOM element audit runs on app startup (silent, badge in status bar) | `init()` in `main.js` ends at line 1937; `requestIdleCallback` deferred call to `runAudit()` goes after `init()` resolves; badge element must be outside `.capture-section` and `.detail-controls` to remain visible across all view states |
| HLTH-02 | Health check verifies all expected interactive elements exist and respond to events | Audit checks `document.getElementById(id) !== null` for static elements and dispatches synthetic `click` events to confirm `addEventListener` is wired (track via flag or one-time side-effect); elements split into Always-Present vs Lazy-Loaded categories (see Architecture section) |
| HLTH-03 | Health report shows specific failures with suggested fixes | Report is a modal overlay rendered by `showReport(issues)` where `issues[]` = `[{ element, description, fix }]`; plain HTML modal pattern consistent with existing `delete-modal` and `add-slack-modal` in `index.html` |
| HLTH-04 | Interactive walkthrough available on first launch and on demand from Settings | First-launch: `walkthrough_completed` flag in `AppSettings` (new `bool` field, `#[serde(default)]`); auto-trigger if false after audit; on-demand: "UI Walkthrough" button in Settings > Audio tab; walkthrough is a JS step-overlay with `getBoundingClientRect` spotlight |
</phase_requirements>

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Vanilla JS | ES2022 | DOM audit, badge rendering, walkthrough overlay | Consistent with entire codebase — no bundler, no framework |
| `requestIdleCallback` | Browser built-in | Defer audit until browser is idle after startup | Correct way to run non-critical work without blocking main thread; WKWebView in Tauri supports it |
| CSS custom properties | Already in styles.css | Badge and walkthrough overlay theming | Already used for all other components; no new variables needed |
| Tauri `invoke` / `save_settings` | Already available | Persist `walkthrough_completed` flag | All settings persistence must go through Tauri; localStorage is not reliable across Tauri app reinstalls |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `getBoundingClientRect()` | Browser built-in | Position walkthrough spotlight | Used to compute highlight position over real DOM elements |
| `MutationObserver` | Already used in integrations-settings.js | Monitor lazy-loaded tab content | For post-load verification of integrations tab elements if needed |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `requestIdleCallback` | `setTimeout(fn, 0)` | `requestIdleCallback` is smarter — waits until the browser reports idle time, not just next tick; better for WKWebView performance. Fallback: `setTimeout(fn, 500)` for environments that don't support it. |
| Modal overlay for walkthrough | Intro.js / Shepherd.js | External library adds bundle complexity; the project has no bundler and loads scripts as static files; vanilla step overlay is straightforward given the simple step count |
| `walkthrough_completed` in AppSettings | localStorage | AppSettings persists in `~/.nbp/settings.json` via Tauri; localStorage can be cleared; per-project convention, all persistent flags go through `save_settings` |

---

## Architecture Patterns

### Recommended Project Structure

Phase 8 adds one new file and modifies four existing files:

```
src/
├── ui-health-check.js    # NEW: runAudit(), renderBadge(), showReport(), walkthrough engine
├── index.html            # Add: health badge div after .app-logo; health report modal; walkthrough overlay; UI Walkthrough button in Settings > Audio; <script src="ui-health-check.js"> after pipeline-builder.js
├── main.js               # Add: call to schedule audit after init() resolves; first-launch walkthrough trigger
└── styles.css            # Add: .health-badge, .health-badge-ok, .health-badge-fail, .health-report-modal, .walkthrough-overlay, .walkthrough-spotlight, .walkthrough-card

src-tauri/src/
└── config.rs             # Add: walkthrough_completed: bool field to AppSettings with #[serde(default)]
```

### Pattern 1: Deferred Audit After init()

**What:** `runAudit()` is called after `init()` resolves via `requestIdleCallback`. This guarantees all async startup calls (settings load, recordings load, pipeline defs load, permission check) have completed before the DOM is inspected.

**When to use:** Startup only. On-demand re-audit triggered from the badge click.

**Example:**
```javascript
// Source: codebase analysis — init() is called at line 1937 of main.js
init().catch(e => console.error('Init failed:', e)).finally(() => {
  // Defer audit until browser is idle
  const scheduleAudit = () => {
    if (typeof runHealthAudit === 'function') {
      runHealthAudit();
    }
  };
  if (typeof requestIdleCallback !== 'undefined') {
    requestIdleCallback(scheduleAudit, { timeout: 2000 });
  } else {
    setTimeout(scheduleAudit, 500); // fallback for environments without rIC
  }
});
```

### Pattern 2: DOM Audit with Synthetic Event Testing

**What:** `runAudit()` iterates a list of element descriptors. For each, it checks `document.getElementById(id)` exists, then dispatches a synthetic `click` or `change` event via `dispatchEvent(new Event('click'))` to confirm event listeners are attached. Response is detected via a one-time side-effect flag set by the handler.

**Critical constraint:** Many handlers check `isRecording`, `isRecordingBusy`, `selectedRecordingId`, etc. before acting. Synthetic events at startup will trigger these guards and exit early — meaning synthetic click detection cannot rely on observable UI state change. Use `element !== null` as the primary check; synthetic events are a secondary "listener attached" verification. For buttons that guard on state, check only existence.

**Element categories discovered from codebase:**

**Always-Present (in DOM at startup):**
```javascript
const ALWAYS_PRESENT_ELEMENTS = [
  // App bar
  { id: 'record-toggle-btn',      desc: 'Record button' },
  { id: 'pipeline-chip-bar',      desc: 'Pipeline chip bar container' },
  { id: 'status-indicator',       desc: 'Status indicator dot' },
  { id: 'timer',                  desc: 'Timer display' },
  { id: 'permission-warning',     desc: 'Permission warning banner' },
  // Sidebar
  { id: 'settings-btn',           desc: 'Settings button' },
  { id: 'sidebar-pipelines-btn',  desc: 'Pipelines sidebar nav' },
  { id: 'sidebar-templates-btn',  desc: 'Templates sidebar nav' },
  // Settings view
  { id: 'settings-view',          desc: 'Settings view section' },
  { id: 'settings-tabs',          desc: 'Settings tab bar' },
  { id: 'save-settings-btn',      desc: 'Save Settings button' },
  { id: 'settings-back-btn',      desc: 'Settings back button' },
  // Settings > Audio
  { id: 'settings-transcription-enabled', desc: 'Auto-transcribe toggle' },
  { id: 'settings-storage-path',  desc: 'Storage path input' },
  { id: 'browse-storage-btn',     desc: 'Browse storage button' },
  { id: 'settings-default-pipeline', desc: 'Default pipeline select' },
  // Settings > Pipelines
  { id: 'pipeline-defs-list',     desc: 'Pipeline definitions list' },
  { id: 'add-pipeline-def-btn',   desc: 'Add pipeline button' },
  { id: 'pipeline-editor',        desc: 'Pipeline editor (hidden)' },
  // Settings > Templates
  { id: 'prompt-templates-list',  desc: 'Prompt templates list' },
  { id: 'add-prompt-template-btn', desc: 'Add template button' },
  // Settings > Integrations (container only — content lazy)
  { id: 'connected-integrations-list', desc: 'Connected integrations container' },
  { id: 'available-integrations-list', desc: 'Available integrations container' },
  // Settings > Theme
  { id: 'theme-purple-btn',       desc: 'Neon Purple theme button' },
  { id: 'theme-blue-btn',         desc: 'Deep Blue theme button' },
  { id: 'theme-light-btn',        desc: 'Light theme button' },
  // Detail view
  { id: 'detail-view',            desc: 'Detail view section' },
  { id: 'back-btn',               desc: 'Back button in detail view' },
  { id: 'detail-title',           desc: 'Recording title input' },
  { id: 'process-btn',            desc: 'Transcribe button' },
  { id: 'detail-pipeline-select', desc: 'Pipeline assignment select' },
  // Modals
  { id: 'delete-modal',           desc: 'Delete confirmation modal' },
  { id: 'add-slack-modal',        desc: 'Add Slack workspace modal' },
  { id: 'notion-wizard-modal',    desc: 'Notion setup wizard modal' },
  { id: 'onboarding-overlay',     desc: 'Onboarding overlay' },
];
```

**Dynamically-rendered (populated after pipeline/integration load — check container only):**
- `#pipeline-chip-bar` children — rendered by `renderPipelineChips()` after `loadPipelineDefs()`. Container exists at startup; chips may be empty if no pipelines defined. This is NOT a failure.
- `#pipeline-defs-list` children — rendered by `renderPipelineDefsList()`. Container exists; empty state is valid.
- `#prompt-templates-list` children — rendered by `renderPromptTemplatesList()`. Same.
- `#connected-integrations-list` / `#available-integrations-list` children — loaded when integrations tab is activated. NOT audited at startup (lazy-load).

**Example audit function:**
```javascript
function runAudit() {
  const issues = [];
  for (const el of ALWAYS_PRESENT_ELEMENTS) {
    const node = document.getElementById(el.id);
    if (!node) {
      issues.push({
        element: el.id,
        description: el.desc + ' is missing from DOM',
        fix: 'Check index.html for element with id="' + el.id + '"'
      });
    }
  }
  return { passed: ALWAYS_PRESENT_ELEMENTS.length - issues.length, failed: issues.length, issues };
}
```

### Pattern 3: Health Badge in App Bar

**What:** A persistent badge element in the app bar, placed after `.app-logo` in `index.html`. It is NOT inside `.capture-section` or `.detail-controls`, so it is never hidden by the body class view-state CSS.

**Critical constraint discovered from CSS analysis:** The `.capture-section` is hidden via `body.detail-open .capture-section { display: none !important }` and `body.settings-open .capture-section { display: none !important }`. The `app-logo` and its siblings outside these sections remain visible in all states. The badge belongs as a sibling of `.app-logo` and `.capture-section`.

**HTML placement:**
```html
<!-- In index.html, inside <header class="app-bar"> -->
<div class="app-logo">
  <span class="logo-text">NBP</span> <span id="app-version"></span>
</div>

<!-- Health badge — always visible, placed after app-logo -->
<div id="health-badge" class="health-badge" style="display:none" title="UI Health Status">
  <!-- Populated by ui-health-check.js -->
</div>
```

**JavaScript badge update:**
```javascript
function renderBadge(result) {
  const badge = document.getElementById('health-badge');
  if (!badge) return;
  if (result.failed === 0) {
    badge.className = 'health-badge health-badge-ok';
    badge.textContent = '✓';
    badge.title = 'All UI elements healthy';
    badge.style.display = '';
    badge.onclick = null;
  } else {
    badge.className = 'health-badge health-badge-fail';
    badge.textContent = '⚠ ' + result.failed;
    badge.title = result.failed + ' UI elements failed health check — click for report';
    badge.style.display = '';
    badge.onclick = () => showReport(result.issues);
  }
}
```

**CSS (consistent with existing styles):**
```css
.health-badge {
  font-size: 0.7rem;
  font-weight: 700;
  border-radius: 4px;
  padding: 2px 6px;
  cursor: default;
  line-height: 1;
  transition: background 0.2s, color 0.2s;
}
.health-badge-ok {
  background: rgba(16, 185, 129, 0.15);
  color: var(--success);
  cursor: default;
}
.health-badge-fail {
  background: rgba(248, 113, 113, 0.15);
  color: var(--danger);
  cursor: pointer;
}
.health-badge-fail:hover {
  background: rgba(248, 113, 113, 0.25);
}
```

### Pattern 4: Health Report Modal

**What:** Clicking the fail badge opens a modal overlay showing the list of issues. Same `.modal-overlay` + `.modal-card` pattern as `#delete-modal`.

**HTML template (added to index.html before closing `</body>`):**
```html
<div id="health-report-modal" class="modal-overlay" style="display:none">
  <div class="modal-card" style="max-width:480px; text-align:left;">
    <h3>UI Health Report</h3>
    <div id="health-report-body" style="margin: 16px 0; max-height: 300px; overflow-y: auto;">
      <!-- Populated by showReport() -->
    </div>
    <div class="modal-actions">
      <button id="health-report-close-btn" class="modal-btn secondary">Close</button>
      <button id="health-report-walkthrough-btn" class="modal-btn primary">Start Walkthrough</button>
    </div>
  </div>
</div>
```

**JS showReport():**
```javascript
function showReport(issues) {
  const modal = document.getElementById('health-report-modal');
  const body = document.getElementById('health-report-body');
  if (!modal || !body) return;

  body.innerHTML = issues.map(issue => `
    <div style="margin-bottom:12px; padding:8px; background:var(--bg-card); border-radius:6px; border-left:3px solid var(--danger);">
      <div style="font-weight:600; font-size:0.85rem;">${escapeHtml(issue.element)}</div>
      <div style="font-size:0.8rem; color:var(--text-secondary); margin-top:2px;">${escapeHtml(issue.description)}</div>
      <div style="font-size:0.75rem; color:var(--accent); margin-top:4px;">Fix: ${escapeHtml(issue.fix)}</div>
    </div>
  `).join('');

  modal.style.display = 'flex';
}
```

### Pattern 5: Interactive Walkthrough

**What:** A step-by-step overlay that spotlights each key UI element with a positioned card explaining it. 8-12 steps covering the primary v2 interactive surfaces. Uses CSS `clip-path` or a transparent hole in a dark overlay for the spotlight effect.

**Implementation approach — simplest viable pattern:**
- Full-screen semi-opaque overlay `div.walkthrough-overlay` covering the viewport
- `div.walkthrough-spotlight` as an absolutely-positioned transparent hole, sized and placed via `getBoundingClientRect()` on the target element
- `div.walkthrough-card` positioned adjacent to the spotlight with step text, Prev/Next/Done buttons
- Steps array drives the sequence; state is a single integer `currentStep`

**Walkthrough steps (derived from all v2 elements):**
```javascript
const WALKTHROUGH_STEPS = [
  { selector: '#pipeline-chip-bar',        title: 'Pipeline Chips',       desc: 'Click a chip to start recording with that pipeline pre-assigned.' },
  { selector: '#record-toggle-btn',        title: 'Record Button',        desc: 'Press to start or stop recording. Press again to play back.' },
  { selector: '#sidebar-pipelines-btn',    title: 'Pipelines Nav',        desc: 'Go to Settings > Pipelines to create and manage pipelines.' },
  { selector: '#sidebar-templates-btn',    title: 'Templates Nav',        desc: 'Go to Settings > Templates to create reusable AI prompt templates.' },
  { selector: '#settings-btn',             title: 'Settings',             desc: 'Access all configuration: audio, integrations, pipelines, and theme.' },
  // Settings pages are not navigated during walkthrough — just highlight entry points
  // Detail view elements require a recording to exist — skip or show with placeholder note
  { selector: '#recordings-list',          title: 'Recordings List',      desc: 'Your recordings appear here. Click one to open the detail view.' },
  { selector: '#health-badge',             title: 'Health Badge',         desc: 'This badge shows UI health status. Green = all good. Red = elements missing.' },
];
```

**Walkthrough spotlight positioning:**
```javascript
function positionSpotlight(selector) {
  const target = document.querySelector(selector);
  const overlay = document.getElementById('walkthrough-overlay');
  const spotlight = document.getElementById('walkthrough-spotlight');
  if (!target || !overlay || !spotlight) return;

  const rect = target.getBoundingClientRect();
  const padding = 8;
  spotlight.style.position = 'fixed';
  spotlight.style.top = (rect.top - padding) + 'px';
  spotlight.style.left = (rect.left - padding) + 'px';
  spotlight.style.width = (rect.width + padding * 2) + 'px';
  spotlight.style.height = (rect.height + padding * 2) + 'px';
  spotlight.style.borderRadius = '8px';
  spotlight.style.boxShadow = '0 0 0 9999px rgba(0,0,0,0.6)';
  spotlight.style.zIndex = '1000';
}
```

**First-launch trigger (in main.js `init()`):**
```javascript
// After init() resolves and after health audit:
if (!appSettings.walkthrough_completed) {
  startWalkthrough();
}
```

**On-demand trigger (Settings > Audio tab):**
```html
<!-- Added to Settings > Audio tab in index.html -->
<div class="settings-section">
  <h3>Help</h3>
  <div class="settings-item">
    <div class="settings-info">
      <label>UI Walkthrough</label>
      <p>Interactive tour of all v2 features</p>
    </div>
    <button id="start-walkthrough-btn" class="mini-action-btn">Start Tour</button>
  </div>
</div>
```

**Walkthrough completion saves flag:**
```javascript
async function finishWalkthrough() {
  document.getElementById('walkthrough-overlay').style.display = 'none';
  appSettings.walkthrough_completed = true;
  await invoke('save_settings', { settings: appSettings });
}
```

### Anti-Patterns to Avoid

- **Do not run the audit synchronously in `init()`.** Audit must run after all async startup calls. Use `requestIdleCallback` after `init()` resolves.
- **Do not place the health badge inside `#capture-section`.** It will be hidden when the detail view or settings view is open. Place as sibling of `.app-logo`.
- **Do not audit the integrations tab content at startup.** `#connected-integrations-list` and `#available-integrations-list` inner cards are lazy-loaded via MutationObserver when the integrations tab becomes active. Their absence at startup is expected and correct.
- **Do not synthesize clicks that trigger state-guarded handlers as the primary test.** `record-toggle-btn` click checks `isRecordingBusy`; synthetic click at startup is immediately no-op'd. Check `element !== null` as the pass condition; synthetic events are an additional check only for non-guarded elements.
- **Do not add `walkthrough_completed` to the JS-side `AppSettings` without adding `#[serde(default)]` to the Rust struct.** Existing settings.json files without this field will fail to deserialize.
- **Do not stack the health-report-close-btn event listener.** Add it once after creating the modal HTML (same pattern as other modals in `main.js`).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Spotlight overlay | Complex canvas masking | CSS `box-shadow: 0 0 0 9999px rgba(0,0,0,0.6)` on positioned element | Single CSS property creates full-screen dark overlay with hole; no canvas needed |
| Walkthrough positioning | Viewport-aware popover library | `getBoundingClientRect()` + `position:fixed` on card | Simple fixed elements avoid scroll offset math; app is fixed-height |
| Settings persistence | localStorage | Tauri `save_settings` / `load_settings` with new `walkthrough_completed: bool` field | Consistent with all other persistent flags in the codebase |
| DOM readiness detection | MutationObserver polling | `init().finally(() => requestIdleCallback(runAudit))` | init() already awaits all async startup calls; no polling needed |

**Key insight:** This phase is almost entirely additive UI work with zero new Rust commands (only one small `AppSettings` field addition). The audit logic is straightforward element presence checking; the walkthrough is a simple step array + positioned overlay.

---

## Common Pitfalls

### Pitfall 1: Health Badge Disappears in Settings/Detail Views

**What goes wrong:** Badge is placed inside `#capture-section`. It vanishes when `body.settings-open` or `body.detail-open` is applied because CSS hides `.capture-section`.
**Why it happens:** CSS view-state logic (`body.settings-open .capture-section { display: none !important }`) hides the entire section.
**How to avoid:** Place `#health-badge` as a direct child of `.app-bar`, NOT inside `.capture-section`. The app bar layout is `justify-content: space-between` with three logical slots: `.app-logo` (left), `.capture-section`/`.detail-controls` (center), and the right side is implicit. Insert health badge between `.app-logo` and the permission warning (or after `.app-logo`) as a flex sibling.
**Warning signs:** Badge shows on recordings view but disappears when settings are opened.

### Pitfall 2: Audit Runs Before allPipelineDefs / AppSettings Are Loaded

**What goes wrong:** `runAudit()` fires before `init()` finishes, causing it to check elements whose state (e.g., chip bar population) is incomplete.
**Why it happens:** `requestIdleCallback` without waiting for `init()` to complete could fire during init if the browser reports idle time between async calls.
**How to avoid:** Chain the audit scheduling AFTER `init()` resolves:
```javascript
init().catch(...).finally(() => { requestIdleCallback(runAudit, { timeout: 2000 }); });
```
Do NOT call `requestIdleCallback(runAudit)` outside the `init()` promise chain.
**Warning signs:** Audit reports false failures because elements aren't populated yet.

### Pitfall 3: Walkthrough Overlay Blocks User Input

**What goes wrong:** Walkthrough overlay has `pointer-events: all` on the dark background but not on the spotlight hole, preventing users from interacting with the highlighted element.
**Why it happens:** Full-screen overlay captures all mouse events by default.
**How to avoid:** Set `pointer-events: none` on the full overlay; set `pointer-events: auto` on the walkthrough card buttons (Prev/Next/Done). The spotlight hole itself doesn't need to be interactive — the walkthrough is explanatory, not interactive with the element.

### Pitfall 4: walkthrough_completed Missing from AppSettings Causes Deserialization Failure

**What goes wrong:** Existing `~/.nbp/settings.json` files without `walkthrough_completed` fail to deserialize in Rust, falling back to `AppSettings::default()` and losing all user settings.
**Why it happens:** Serde's default behavior treats missing fields as an error without `#[serde(default)]`.
**How to avoid:** Add `#[serde(default)] pub walkthrough_completed: bool` to `AppSettings` in `config.rs`. Default is `false` (bool default in Rust). Existing settings will deserialize with `walkthrough_completed = false`, triggering the walkthrough once on upgrade.
**Warning signs:** All settings reset on upgrade; settings.json shows missing field in Rust error log.

### Pitfall 5: Walkthrough Target Elements Not in Viewport

**What goes wrong:** Walkthrough tries to spotlight an element that's in a hidden view (e.g., pipeline editor inside settings which is not open, or detail view which is not open).
**Why it happens:** `getBoundingClientRect()` returns `{ top: 0, left: 0, width: 0, height: 0 }` for elements with `display: none`.
**How to avoid:** Skip steps where the target element has zero dimensions. Only spotlight elements that are currently visible. Design walkthrough steps to avoid elements that require navigation to another view. Or add a "navigate then spotlight" mechanism that calls `ViewManager.showSettings()` before spotlighting settings elements.
**Warning signs:** Spotlight appears in top-left corner at 0,0 with 0 size.

### Pitfall 6: Multiple Health Report Event Listeners Stack

**What goes wrong:** Each badge click adds a new `onclick` to the close button, stacking listeners.
**Why it happens:** `showReport()` sets innerHTML and attaches listeners each time without cleanup.
**How to avoid:** Wire close/walkthrough button listeners once at page load (same as other modals in `main.js`), not inside `showReport()`. Only update `#health-report-body` innerHTML in `showReport()`.

---

## Code Examples

### AppSettings Addition (config.rs)

```rust
// Source: direct reading of src-tauri/src/config.rs
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    // ... existing fields (storage_path, auto_discard_seconds, theme, onboarding_completed,
    //     transcription, show_recording_notification, save_mix_only, integrations,
    //     default_pipeline, last_used_pipeline) ...

    /// Whether the user has completed the interactive UI walkthrough
    #[serde(default)]
    pub walkthrough_completed: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            walkthrough_completed: false,
        }
    }
}
```

### ui-health-check.js Module Structure

```javascript
// ui-health-check.js
// Globals available (loaded after pipeline-builder.js): escapeHtml, invoke, appSettings
// Does NOT declare any var globals — all state is module-local

const AUDIT_ELEMENTS = [
  // Always-present elements (see Architecture section for full list)
  { id: 'record-toggle-btn',   desc: 'Record button' },
  { id: 'pipeline-chip-bar',   desc: 'Pipeline chip bar' },
  { id: 'status-indicator',    desc: 'Status indicator' },
  { id: 'settings-btn',        desc: 'Settings button' },
  { id: 'settings-view',       desc: 'Settings view' },
  { id: 'save-settings-btn',   desc: 'Save settings button' },
  { id: 'settings-tabs',       desc: 'Settings tab bar' },
  { id: 'pipeline-defs-list',  desc: 'Pipeline list container' },
  { id: 'add-pipeline-def-btn','desc': 'Add pipeline button' },
  { id: 'connected-integrations-list', desc: 'Integrations connected list' },
  { id: 'available-integrations-list', desc: 'Integrations available list' },
  { id: 'detail-view',         desc: 'Detail view section' },
  { id: 'back-btn',            desc: 'Detail view back button' },
  { id: 'process-btn',         desc: 'Transcribe button' },
  { id: 'detail-pipeline-select', desc: 'Pipeline assignment select' },
  { id: 'onboarding-overlay',  desc: 'Onboarding overlay' },
  { id: 'delete-modal',        desc: 'Delete confirmation modal' },
  { id: 'add-slack-modal',     desc: 'Slack workspace modal' },
  { id: 'notion-wizard-modal', desc: 'Notion setup wizard modal' },
  // ... complete list per Architecture section
];

function runHealthAudit() {
  const issues = [];
  for (const spec of AUDIT_ELEMENTS) {
    const el = document.getElementById(spec.id);
    if (!el) {
      issues.push({
        element: spec.id,
        description: spec.desc + ' element is missing from the DOM',
        fix: 'Check index.html for id="' + spec.id + '" — may have been removed or renamed'
      });
    }
  }
  const result = {
    passed: AUDIT_ELEMENTS.length - issues.length,
    failed: issues.length,
    issues
  };
  renderHealthBadge(result);
  return result;
}

function renderHealthBadge(result) {
  const badge = document.getElementById('health-badge');
  if (!badge) return;
  badge.style.display = '';
  if (result.failed === 0) {
    badge.className = 'health-badge health-badge-ok';
    badge.textContent = '✓';
    badge.title = 'UI health: all ' + result.passed + ' elements verified';
    badge.style.cursor = 'default';
    badge.onclick = null;
  } else {
    badge.className = 'health-badge health-badge-fail';
    badge.textContent = '⚠ ' + result.failed;
    badge.title = result.failed + ' elements failed — click for details';
    badge.style.cursor = 'pointer';
    badge.onclick = () => showHealthReport(result.issues);
  }
}

function showHealthReport(issues) {
  const modal = document.getElementById('health-report-modal');
  const body = document.getElementById('health-report-body');
  if (!modal || !body) return;
  body.innerHTML = issues.map(issue =>
    '<div class="health-issue-row">' +
    '<div class="health-issue-id">' + escapeHtml(issue.element) + '</div>' +
    '<div class="health-issue-desc">' + escapeHtml(issue.description) + '</div>' +
    '<div class="health-issue-fix">Fix: ' + escapeHtml(issue.fix) + '</div>' +
    '</div>'
  ).join('');
  modal.style.display = 'flex';
}

// Walkthrough engine
const WALKTHROUGH_STEPS = [
  { selector: '#pipeline-chip-bar',      title: 'Pipeline Chips',    desc: 'Click any chip to instantly start recording with that pipeline pre-assigned.' },
  { selector: '#record-toggle-btn',      title: 'Record Button',     desc: 'Start or stop recording. While a recording is selected, this button plays it back.' },
  { selector: '#sidebar-pipelines-btn',  title: 'Pipelines',         desc: 'Click to open Settings > Pipelines and build multi-step AI processing pipelines.' },
  { selector: '#sidebar-templates-btn',  title: 'Templates',         desc: 'Click to open Settings > Templates and create reusable AI prompt templates.' },
  { selector: '#settings-btn',           title: 'Settings',          desc: 'Configure audio, integrations (Notion, Slack), pipelines, and appearance.' },
  { selector: '#recordings-list',        title: 'Recordings',        desc: 'All your recordings appear here. Click any recording to open its detail view.' },
  { selector: '#health-badge',           title: 'Health Badge',      desc: 'This badge confirms all UI elements loaded correctly. Green = healthy, Red = issues found.' },
];

let walkthroughStep = 0;

function startWalkthrough() {
  walkthroughStep = 0;
  const overlay = document.getElementById('walkthrough-overlay');
  if (overlay) overlay.style.display = 'flex';
  showWalkthroughStep(walkthroughStep);
}

function showWalkthroughStep(stepIndex) {
  const step = WALKTHROUGH_STEPS[stepIndex];
  if (!step) return;
  const target = document.querySelector(step.selector);
  const spotlight = document.getElementById('walkthrough-spotlight');
  const card = document.getElementById('walkthrough-card');
  const titleEl = document.getElementById('walkthrough-title');
  const descEl = document.getElementById('walkthrough-desc');
  const prevBtn = document.getElementById('walkthrough-prev');
  const nextBtn = document.getElementById('walkthrough-next');
  const stepCounter = document.getElementById('walkthrough-step');

  if (titleEl) titleEl.textContent = step.title;
  if (descEl) descEl.textContent = step.desc;
  if (stepCounter) stepCounter.textContent = (stepIndex + 1) + ' / ' + WALKTHROUGH_STEPS.length;
  if (prevBtn) prevBtn.style.display = stepIndex === 0 ? 'none' : '';
  if (nextBtn) nextBtn.textContent = stepIndex === WALKTHROUGH_STEPS.length - 1 ? 'Done' : 'Next';

  if (target && spotlight) {
    const rect = target.getBoundingClientRect();
    const pad = 8;
    Object.assign(spotlight.style, {
      display: 'block',
      top: (rect.top - pad) + 'px',
      left: (rect.left - pad) + 'px',
      width: (rect.width + pad * 2) + 'px',
      height: (rect.height + pad * 2) + 'px',
    });
  } else if (spotlight) {
    spotlight.style.display = 'none'; // Target not visible; skip spotlight
  }
}

async function finishWalkthrough() {
  const overlay = document.getElementById('walkthrough-overlay');
  if (overlay) overlay.style.display = 'none';
  if (typeof appSettings !== 'undefined') {
    appSettings.walkthrough_completed = true;
    try {
      await invoke('save_settings', { settings: appSettings });
    } catch (e) {
      console.error('Failed to save walkthrough_completed:', e);
    }
  }
}

// Wire walkthrough button controls once DOM is ready
// (Called from main.js after init() resolves, so appSettings is available)
function initHealthCheck() {
  const closeBtn = document.getElementById('health-report-close-btn');
  if (closeBtn) {
    closeBtn.addEventListener('click', () => {
      const modal = document.getElementById('health-report-modal');
      if (modal) modal.style.display = 'none';
    });
  }

  const walkthroughFromReportBtn = document.getElementById('health-report-walkthrough-btn');
  if (walkthroughFromReportBtn) {
    walkthroughFromReportBtn.addEventListener('click', () => {
      const modal = document.getElementById('health-report-modal');
      if (modal) modal.style.display = 'none';
      startWalkthrough();
    });
  }

  const onDemandBtn = document.getElementById('start-walkthrough-btn');
  if (onDemandBtn) {
    onDemandBtn.addEventListener('click', startWalkthrough);
  }

  const prevBtn = document.getElementById('walkthrough-prev');
  if (prevBtn) {
    prevBtn.addEventListener('click', () => {
      if (walkthroughStep > 0) showWalkthroughStep(--walkthroughStep);
    });
  }

  const nextBtn = document.getElementById('walkthrough-next');
  if (nextBtn) {
    nextBtn.addEventListener('click', async () => {
      if (walkthroughStep < WALKTHROUGH_STEPS.length - 1) {
        showWalkthroughStep(++walkthroughStep);
      } else {
        await finishWalkthrough();
      }
    });
  }

  const skipBtn = document.getElementById('walkthrough-skip');
  if (skipBtn) {
    skipBtn.addEventListener('click', finishWalkthrough);
  }
}
```

### Script Load Order Update (index.html)

```html
<!-- Current order (end of <body>): -->
<script src="vendor/sortable.min.js"></script>
<script src="main.js"></script>
<script src="integrations-settings.js"></script>
<script src="pipeline-builder.js"></script>
<!-- Add: -->
<script src="ui-health-check.js"></script>
```

### main.js Modification (init scheduling)

```javascript
// Current at end of main.js (line 1937):
// init().catch(e => console.error('Init failed:', e));

// Replace with:
init().catch(e => console.error('Init failed:', e)).finally(() => {
  // Initialize health check controls (event listeners wired once)
  if (typeof initHealthCheck === 'function') initHealthCheck();

  // Schedule DOM audit after browser is idle
  const scheduleAudit = () => {
    if (typeof runHealthAudit === 'function') {
      const result = runHealthAudit();
      // Trigger walkthrough on first launch (after audit so badge is visible)
      if (typeof appSettings !== 'undefined' && !appSettings.walkthrough_completed) {
        if (typeof startWalkthrough === 'function') startWalkthrough();
      }
    }
  };
  if (typeof requestIdleCallback !== 'undefined') {
    requestIdleCallback(scheduleAudit, { timeout: 2000 });
  } else {
    setTimeout(scheduleAudit, 500);
  }
});
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No startup DOM verification | `requestIdleCallback`-deferred audit | Phase 8 | Catch element regressions at startup |
| Manual user discovery of features | Step-by-step walkthrough overlay | Phase 8 | First-launch onboarding beyond permissions |
| Onboarding = permission request only | Onboarding + UI walkthrough | Phase 8 | Users understand pre-assignment UX and pipeline chips |

**Deprecated/outdated:**
- No prior walkthrough mechanism existed — `onboarding-overlay` only handled permission requests; walkthrough is additive.

---

## Open Questions

1. **Badge visibility when capture-section is hidden**
   - What we know: CSS hides `.capture-section` when settings or detail view is open; the app bar uses `justify-content: space-between` with app-logo on left, capture-section/detail-controls in the right area
   - What's unclear: The exact flex order and whether adding a new flex child after `.app-logo` will visually displace the capture-section
   - Recommendation: Place badge between `.app-logo` and `.permission-warning` (or immediately after `.app-logo`). If it causes layout shift, use `position: absolute; left: [logo-width + gap]px` to avoid affecting flex layout.

2. **Walkthrough skips detail view elements**
   - What we know: `#back-btn`, `#process-btn`, `#detail-title` are in the detail view which is `display:none` at startup
   - What's unclear: Should the walkthrough navigate to detail view to show those elements, or simply skip them?
   - Recommendation: Skip detail view elements in the walkthrough. They are straightforward (Back, Transcribe buttons) and discovered naturally during first recording. The walkthrough focuses on the v2 pre-assignment UX which is less obvious.

3. **Walkthrough and onboarding ordering**
   - What we know: `onboarding-overlay` (permissions) shows if `!appSettings.onboarding_completed`; walkthrough shows if `!appSettings.walkthrough_completed`
   - What's unclear: If both are false (truly fresh install), should permissions come first, then walkthrough after user dismisses?
   - Recommendation: Yes — maintain existing onboarding priority. Only trigger walkthrough if `onboarding_completed && !walkthrough_completed`. Users who haven't granted permissions cannot meaningfully record and shouldn't be walked through the pipeline chip bar yet. In `init()`, after existing onboarding check, add: `else if (!appSettings.walkthrough_completed) { schedule walkthrough after audit }`.

4. **requestIdleCallback timeout value**
   - What we know: `requestIdleCallback(fn, { timeout: 2000 })` fires the callback at most 2 seconds after calling, even if the browser is not idle
   - What's unclear: Whether WKWebView on macOS always supports `requestIdleCallback` or has the polyfill
   - Recommendation: Use `typeof requestIdleCallback !== 'undefined'` guard and fall back to `setTimeout(fn, 500)`. The 500ms fallback gives enough time for the page to settle without being perceptibly slow.

---

## Sources

### Primary (HIGH confidence)
- Direct codebase reading: `/workspace/src/index.html` — full HTML structure, all element IDs, `#capture-section` contents, `#app-logo` placement, script load order, existing modal patterns (`#delete-modal`, `#add-slack-modal`, `#notion-wizard-modal`, `#onboarding-overlay`)
- Direct codebase reading: `/workspace/src/main.js` — `init()` function (lines 1913-1937), existing `onboarding_completed` flag pattern (lines 432, 524, 1932), view state management (`ViewManager`), guard variables (`isRecordingBusy`, `isRecording`, `selectedRecordingId`)
- Direct codebase reading: `/workspace/src/styles.css` — `.app-bar` layout (`justify-content: space-between`, `position: fixed`), view state CSS (`body.settings-open .capture-section { display: none !important }`, `body.detail-open`), existing badge CSS pattern (`.pipeline-status-badge`, `.health-warning`)
- Direct codebase reading: `/workspace/src/pipeline-builder.js` — `dismissPicker` outside-click pattern, `allPipelineDefs` global declaration
- Direct codebase reading: `/workspace/src/integrations-settings.js` — MutationObserver lazy-load pattern for integrations tab, `var notionProfiles`, `var savePathIntegrations` globals
- Direct codebase reading: `/workspace/src-tauri/src/config.rs` — `AppSettings` struct with all current fields, `#[serde(default)]` pattern, `get_settings_path()`, `save_settings` command
- Direct codebase reading: `/workspace/.planning/REQUIREMENTS.md` — HLTH-01 through HLTH-04 requirements
- Direct codebase reading: `/workspace/.planning/ROADMAP.md` — Phase 8 plan outlines (08-01, 08-02)
- Direct codebase reading: `/workspace/.planning/STATE.md` — accumulated decisions affecting this phase

### Secondary (MEDIUM confidence)
- CSS spotlight technique (box-shadow approach): well-known vanilla CSS pattern; `box-shadow: 0 0 0 9999px rgba(0,0,0,0.6)` creates full-screen overlay with transparent hole — verified as working in WKWebView contexts from general web knowledge

### Tertiary (LOW confidence)
- None — all primary claims verified from codebase

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no external libraries; pure vanilla JS consistent with entire codebase
- Architecture: HIGH — all element IDs verified from index.html; startup sequence verified from main.js; CSS constraints verified from styles.css
- Pitfalls: HIGH — derived from direct code analysis (CSS view-state hiding, lazy-load pattern in integrations-settings.js, serde default pattern in config.rs)

**Research date:** 2026-02-19
**Valid until:** 2026-03-19 (stable vanilla JS codebase; no external dependencies)
