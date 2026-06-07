// step-editor.js — Pipeline step editor (inline, self-contained steps).
//
// One step = pick a type → fill its inline config → write the prompt/script.
// The chain is strictly linear: step 1 eats the transcript, each later step
// eats the previous step's output. There is no separate "Connection" object —
// all config (which CLI, model, cwd, env, timeout) lives on the step itself.
//
// Two step types:
//   - CLI agent : config { cli, model?, timeout_secs? }; template is the prompt
//                 (with {transcript} / {processing_result} / {app} substituted
//                 before the agent sees it).
//   - Shell     : config { cwd, shell?, env?, timeout_secs? }; template is the
//                 bash script body. Raw values arrive via NBP_* env vars (no
//                 placeholder substitution — avoids shell-injection).

import { invoke } from '../core/tauri.js';
import { escapeHtml } from '../core/utils.js';
import { showToast } from '../ui/toast.js';
import * as pipelineState from './state.js';
import { closeStepEditorPanel, renderPipelineSteps } from './editor.js';
import { maybeAutoName } from './delivery-options.js';

// Step types, in dropdown order.
const STEP_TYPES = [
  { key: 'cli_agent',  label: 'CLI agent' },
  { key: 'shell',      label: 'Shell script' },
  { key: 'save_local', label: 'Save to folder' },
];

// Per-type copy for the prompt / script textarea.
const BODY_COPY = {
  cli_agent: {
    label: 'Prompt for the agent',
    placeholder: 'What to ask the CLI agent. Use {transcript} or {processing_result} to inline content.',
    hint: 'Sent to the agent as the prompt body. Placeholders below get substituted before the agent sees them.',
    showPlaceholders: true,
    mono: false,
  },
  shell: {
    label: 'Shell script',
    placeholder: 'echo "$NBP_TRANSCRIPT" | jq -R .   # any bash you like — multi-line ok',
    hint: 'Runs in the working dir set above (default shell: /bin/bash). Stdout becomes this step\'s output, stderr goes to the run log only. Env vars: $NBP_TRANSCRIPT (full transcript), $NBP_PROCESSING_RESULT (previous step output, empty on step 1), $NBP_APP (Zoom / FaceTime / NBP / …), $NBP_CALENDAR_TITLE, $NBP_CALENDAR_ATTENDEES, $NBP_DATE. Read them with $VAR — placeholder substitution is NOT applied to shell scripts.',
    showPlaceholders: false,
    mono: true,
  },
  save_local: {
    label: 'Content to save',
    placeholder: '{transcript}',
    hint: 'Written to a file in the folder above. Defaults to the raw transcript — compose your own note with the variables below.',
    showPlaceholders: true,
    mono: false,
  },
};

// `check_cli_availability` result cache. Refreshed when the editor opens —
// cheap (`which X` per supported CLI) so newly-installed agents show up
// without an app restart.
let cliAvailabilityCache = [];

async function refreshCliAvailability() {
  try {
    cliAvailabilityCache = await invoke('check_cli_availability');
  } catch (err) {
    console.error('check_cli_availability failed:', err);
    cliAvailabilityCache = [];
  }
}

// First installed CLI id (whichever comes first), or '' if none installed.
// Used as the default selection so a CLI step is runnable the moment you
// pick the type — no empty "choose a CLI" state.
function firstInstalledCli() {
  return cliAvailabilityCache.find(c => c.installed)?.id || '';
}

export async function addNewStep() {
  await refreshCliAvailability();
  const defaultType = 'cli_agent';
  // Pre-select the first installed CLI so the step is runnable out of the box.
  const firstCli = firstInstalledCli();
  const step = {
    name: '',
    step_type: defaultType,
    config: firstCli ? { cli: firstCli } : {},
    template: defaultTemplateFor(defaultType),
    description: null,
  };
  pipelineState.pipelineEditorSteps.push(step);
  renderPipelineSteps();
  showStepEditor(pipelineState.pipelineEditorSteps.length - 1);
}

export async function showStepEditor(index) {
  const step = pipelineState.pipelineEditorSteps[index];
  if (!step) return;
  await refreshCliAvailability();

  const panelEl = document.getElementById('step-editor-panel');
  if (!panelEl) return;

  pipelineState.setEditingStepIndex(index);
  renderPipelineSteps();

  panelEl.innerHTML = renderForm(step, index);
  panelEl.style.display = 'block';
  const editorEl = panelEl.querySelector('.step-editor');
  setTimeout(() => editorEl?.scrollIntoView({ behavior: 'smooth', block: 'nearest' }), 50);

  wireFormEvents(editorEl, step, index);
}

function renderForm(step, index) {
  const currentType = step.step_type || 'cli_agent';
  const typeOptions = STEP_TYPES.map(t =>
    `<option value="${escapeHtml(t.key)}"${t.key === currentType ? ' selected' : ''}>${escapeHtml(t.label)}</option>`
  ).join('');

  return `
    <div class="step-editor">
      <div class="step-editor-header">
        <span class="step-editor-title">Step ${index + 1}</span>
        <button class="step-editor-close" title="Close">×</button>
      </div>

      <div class="step-section">
        <div class="step-editor-row">
          <label>Type</label>
          <select class="step-type-select">${typeOptions}</select>
        </div>
      </div>

      <div class="step-body-host" data-current-type="${escapeHtml(currentType)}">
        ${renderBody(currentType, step)}
      </div>

      <div class="step-editor-actions">
        <button class="step-editor-done">Done</button>
      </div>
    </div>
  `;
}

// Body of the form below the Type row — purely type-specific config + template.
// Steps carry no user-facing name: the chip shows the type (CLI binary / Shell
// / Save to folder), which is enough to read the pipeline. An internal unique
// name is auto-generated on save (it backs the per-step artifact file).
function renderBody(typeKey, step) {
  if (typeKey === 'save_local') return renderSaveBody(step);
  return (typeKey === 'shell') ? renderShellBody(step) : renderCliBody(step);
}

// --- CLI agent body -------------------------------------------------------
// Compact one-line rows (matching the Save view). Working dir gets a Browse
// picker like Save's folder — it's where the CLI runs (point at a repo so the
// agent can read those files). Model is rarely touched, kept minimal.
function renderCliBody(step) {
  const cfg = step.config || {};
  const copy = BODY_COPY.cli_agent;
  const rowLabel = 'font-size:0.85rem;color:var(--text-secondary);min-width:88px;';
  const browseBtn = 'white-space:nowrap;padding:6px 12px;border:1px solid var(--border-color);border-radius:6px;background:var(--bg-input);color:var(--text-primary);cursor:pointer;font-size:0.82rem;';
  return `
    <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;display:flex;flex-direction:column;gap:12px;">
      ${renderCliDetectField(cfg.cli || firstInstalledCli(), rowLabel)}
      <div style="display:flex;gap:8px;align-items:center;">
        <span style="${rowLabel}">Model</span>
        <input class="step-cfg-model" type="text" value="${escapeHtml(cfg.model || '')}" placeholder="CLI default — e.g. sonnet, o3, openai/gpt-4o" style="flex:1;min-width:0;" />
      </div>
      <div>
        <div style="display:flex;gap:8px;align-items:center;">
          <span style="${rowLabel}">Working dir</span>
          <input class="step-cfg-workdir" type="text" value="${escapeHtml(cfg.working_directory || '')}" placeholder="default: home folder" style="flex:1;min-width:0;" />
          <button type="button" class="js-folder-browse" data-input=".step-cfg-workdir" style="${browseBtn}">Browse…</button>
        </div>
        <div style="font-size:0.72rem;color:var(--text-secondary);opacity:0.8;margin-top:4px;">Where the CLI runs — point it at a repo/folder so the agent can read those files. \`~\` expanded.</div>
      </div>
      <div style="display:flex;gap:8px;align-items:center;">
        <span style="${rowLabel}">Timeout</span>
        <input class="step-cfg-timeout" type="number" value="${cfg.timeout_secs == null ? '' : escapeHtml(String(cfg.timeout_secs))}" placeholder="3600" style="width:90px;" />
        <span style="font-size:0.72rem;color:var(--text-secondary);opacity:0.7;">seconds</span>
      </div>
    </div>
    ${renderTemplateField('cli_agent', step, copy)}
  `;
}

// --- Shell body -----------------------------------------------------------
// Compact one-line rows (matching CLI/Save). Working dir is required and gets
// the same Browse picker — it's where the script runs. No separate env field:
// if you're writing the script you set vars there; the NBP_* vars we inject
// are documented under the script box.
function renderShellBody(step) {
  const cfg = step.config || {};
  const copy = BODY_COPY.shell;
  const rowLabel = 'font-size:0.85rem;color:var(--text-secondary);min-width:88px;';
  const browseBtn = 'white-space:nowrap;padding:6px 12px;border:1px solid var(--border-color);border-radius:6px;background:var(--bg-input);color:var(--text-primary);cursor:pointer;font-size:0.82rem;';
  return `
    <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;display:flex;flex-direction:column;gap:12px;">
      <div>
        <div style="display:flex;gap:8px;align-items:center;">
          <span style="${rowLabel}">Working dir</span>
          <input class="step-cfg-cwd" type="text" value="${escapeHtml(cfg.cwd || '')}" placeholder="required — e.g. ~/work/notes" style="flex:1;min-width:0;" />
          <button type="button" class="js-folder-browse" data-input=".step-cfg-cwd" style="${browseBtn}">Browse…</button>
        </div>
        <div style="font-size:0.72rem;color:var(--text-secondary);opacity:0.8;margin-top:4px;">Where the script runs. \`~\` expanded.</div>
      </div>
      <div style="display:flex;gap:8px;align-items:center;">
        <span style="${rowLabel}">Shell</span>
        <input class="step-cfg-shell" type="text" value="${escapeHtml(cfg.shell || '')}" placeholder="/bin/bash (default)" style="flex:1;min-width:0;" />
      </div>
      <div style="display:flex;gap:8px;align-items:center;">
        <span style="${rowLabel}">Timeout</span>
        <input class="step-cfg-timeout" type="number" value="${cfg.timeout_secs == null ? '' : escapeHtml(String(cfg.timeout_secs))}" placeholder="120" style="width:90px;" />
        <span style="font-size:0.72rem;color:var(--text-secondary);opacity:0.7;">seconds</span>
      </div>
    </div>
    ${renderTemplateField('shell', step, copy)}
  `;
}

// --- Save-to-folder body --------------------------------------------------
// Compact: the auto file-name shown as text above the folder, then Folder
// (one line + Browse) and a single free-form Content template (no toggles) —
// defaults to `{transcript}`, the user tunes it with the same placeholders as a
// CLI prompt. The engine renders step.template like any other step, so the
// content is whatever the user composed. No Name input — the file is
// auto-named from the recording.
function renderSaveBody(step) {
  const cfg = step.config || {};
  const rowLabel = 'font-size:0.85rem;color:var(--text-secondary);min-width:52px;';
  return `
    <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;display:flex;flex-direction:column;gap:12px;">
      <div>
        <div style="font-size:0.8rem;color:var(--text-secondary);">File name</div>
        <div style="font-size:0.85rem;color:var(--text-primary);margin-top:2px;"><code>&lt;app&gt; &lt;date&gt; &lt;time&gt;.md</code></div>
        <div style="font-size:0.72rem;color:var(--text-secondary);opacity:0.8;margin-top:2px;">Auto-named from the recording — e.g. <code>Zoom 2026-06-01 14-30.md</code>. You don't set it.</div>
      </div>
      <div style="display:flex;gap:8px;align-items:center;">
        <span style="${rowLabel}">Folder</span>
        <input class="step-cfg-folder" type="text" value="${escapeHtml(cfg.folder_path || '')}" placeholder="~/Documents/Meetings" style="flex:1;min-width:0;" />
        <button type="button" class="js-folder-browse" data-input=".step-cfg-folder" style="white-space:nowrap;padding:6px 12px;border:1px solid var(--border-color);border-radius:6px;background:var(--bg-input);color:var(--text-primary);cursor:pointer;font-size:0.82rem;">Browse…</button>
      </div>
      <div style="font-size:0.72rem;color:var(--text-secondary);opacity:0.85;margin-top:-4px;line-height:1.5;">
        Date/time placeholders route into dated subfolders, in any order/format —
        <code>{YYYY}</code> (year), <code>{YY}</code> (2-digit year),
        <code>{MM}</code> (month), <code>{DD}</code> (day), <code>{date}</code> (YYYY-MM-DD),
        <code>{HH}</code> (hour), <code>{mm}</code> (minute), <code>{SS}</code> (second),
        <code>{time}</code> (HH-MM-SS). Note <code>{mm}</code> is minutes, <code>{MM}</code> is month.
        E.g. <code>~/output/{YYYY}/{MM}-{DD}</code> or <code>~/notes/{DD}-{MM}-{YY}/{HH}-{mm}</code>.
      </div>
    </div>
    ${renderTemplateField('save_local', step, BODY_COPY.save_local)}
  `;
}

// Prompt (CLI) / script (Shell) textarea — full width, with per-type help.
// CLI shows the available variables as a bullet list; Shell keeps its env-var
// note (placeholder substitution doesn't apply to shell scripts).
function renderTemplateField(typeKey, step, copy) {
  const monoStyle = copy.mono ? 'font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:0.82rem;' : '';
  const help = copy.showPlaceholders
    ? `<div style="font-size:0.72rem;color:var(--text-secondary);opacity:0.85;margin-top:8px;line-height:1.5;">
         Available variables — drop them into your prompt:
         <ul style="margin:4px 0 0;padding-left:18px;">
           <li><code>{transcript}</code> — the full recording transcript</li>
           <li><code>{processing_result}</code> — the previous step's output (empty on the first step)</li>
           <li><code>{app}</code> — the app name (Zoom / FaceTime / NBP)</li>
           <li><code>{calendar_title}</code> — matched calendar event title (empty if none)</li>
           <li><code>{calendar_attendees}</code> — attendees of the matched event</li>
           <li><code>{date}</code> — recording date (YYYY-MM-DD)</li>
         </ul>
       </div>`
    : `<div style="font-size:0.72rem;color:var(--text-secondary);opacity:0.85;margin-top:6px;line-height:1.5;">${escapeHtml(copy.hint)}</div>`;
  return `
    <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;">
      <div class="step-section-label">${escapeHtml(copy.label)}</div>
      <textarea class="step-template-textarea" rows="8" placeholder="${escapeHtml(copy.placeholder)}" style="width:100%;box-sizing:border-box;${monoStyle}">${escapeHtml(step.template || '')}</textarea>
      ${help}
    </div>
  `;
}

// CLI picker built from `check_cli_availability`. Only installed CLIs are
// pickable; missing ones list below with a copy-paste install command. A
// previously-saved CLI that's no longer installed is preserved with a ⚠.
function renderCliDetectField(currentValue, rowLabel) {
  const installed = cliAvailabilityCache.filter(c => c.installed);
  const missing = cliAvailabilityCache.filter(c => !c.installed);

  const installedOpts = installed.map(c => {
    const sel = currentValue === c.id ? ' selected' : '';
    return `<option value="${escapeHtml(c.id)}"${sel}>${escapeHtml(c.name)} (${escapeHtml(c.id)})</option>`;
  }).join('');

  const stale = currentValue && !installed.some(c => c.id === currentValue);
  const staleOpt = stale
    ? `<option value="${escapeHtml(currentValue)}" selected>⚠ ${escapeHtml(currentValue)} — not installed</option>`
    : '';

  const placeholderOpt = !currentValue && installed.length > 0
    ? '<option value="" disabled selected>— choose a CLI —</option>'
    : '';

  const noneInstalled = installed.length === 0 && !stale;
  const selectHTML = noneInstalled
    ? `<select class="step-cfg-cli" disabled style="flex:1;min-width:0;opacity:0.6;"><option value="">No CLI agents installed</option></select>`
    : `<select class="step-cfg-cli" style="flex:1;min-width:0;">${placeholderOpt}${staleOpt}${installedOpts}</select>`;

  const missingHTML = missing.length === 0
    ? ''
    : `<div style="margin-top:6px;display:flex;flex-direction:column;gap:4px;">
         ${missing.map(c => `
           <div style="font-size:0.72rem;color:var(--text-secondary);opacity:0.75;display:flex;gap:6px;align-items:center;">
             <span style="opacity:0.7;">Install ${escapeHtml(c.name)}:</span>
             <code style="background:var(--bg-input);padding:1px 6px;border-radius:3px;font-size:0.7rem;">${escapeHtml(c.install_hint)}</code>
           </div>
         `).join('')}
       </div>`;

  return `
    <div style="display:flex;flex-direction:column;gap:4px;">
      <div style="display:flex;gap:8px;align-items:center;">
        <span style="${rowLabel}">CLI</span>
        ${selectHTML}
      </div>
      ${missingHTML}
    </div>
  `;
}

function wireFormEvents(editorEl, step, index) {
  // Close — drop untouched drafts so abandoning a fresh "+ Add Step" cleans up.
  // A step only gets a name once the user edits it (syncStepFromForm auto-names
  // on any change / Done); steps always have a default template, so name is the
  // real "was this touched?" signal.
  editorEl.querySelector('.step-editor-close').addEventListener('click', () => {
    if (!step.name) {
      pipelineState.pipelineEditorSteps.splice(index, 1);
    }
    pipelineState.setEditingStepIndex(null);
    closeStepEditorPanel();
    renderPipelineSteps();
    maybeAutoName();
  });

  // Type change — re-render the body for the new type, resetting config +
  // template (old config belongs to a different type).
  const typeSelect = editorEl.querySelector('.step-type-select');
  typeSelect.addEventListener('change', () => {
    const newType = typeSelect.value;
    const host = editorEl.querySelector('.step-body-host');
    if (host) {
      host.dataset.currentType = newType;
      const shadow = { ...step, config: {}, template: defaultTemplateFor(newType) };
      host.innerHTML = renderBody(newType, shadow);
      wireBodyExtras(editorEl);
    }
  });

  // Per-type body widgets (re-wired after every body re-render).
  wireBodyExtras(editorEl);

  // Live preview: any field change (CLI dropdown, folder, model, template,
  // type, …) writes the current form state into the step and re-renders the
  // chips, so the chip label tracks the selected CLI immediately instead of
  // only on Done. `change` bubbles from the inner inputs/selects to here.
  editorEl.addEventListener('change', () => {
    syncStepFromForm(editorEl, step, index);
    renderPipelineSteps();
    maybeAutoName();
  });

  // Done — finalise the step + close.
  editorEl.querySelector('.step-editor-done').addEventListener('click', () => {
    syncStepFromForm(editorEl, step, index);
    pipelineState.setEditingStepIndex(null);
    closeStepEditorPanel();
    renderPipelineSteps();
    maybeAutoName();
  });
}

// Read the whole form into `step` (type + inline config + template + an
// auto-generated unique internal name). Shared by the live-preview change
// handler and Done so they can't drift.
function syncStepFromForm(editorEl, step, index) {
  const typeKey = editorEl.querySelector('.step-type-select')?.value || step.step_type;
  // Every type now uses the same freeform template textarea (save included).
  const template = editorEl.querySelector('.step-template-textarea')?.value ?? '';
  const config = collectConfig(editorEl, typeKey);
  step.step_type = typeKey;
  step.config = config;
  step.template = template;
  // No user-facing name — auto-generate a unique internal id (backs the
  // per-step artifact filename + the pipeline validator's uniqueness check).
  step.name = defaultStepName(typeKey, config, index);
}

// Wire the native folder pickers (Save folder + CLI working dir). Each Browse
// button carries `data-input` = the selector of the text field it fills. The
// fields stay normal editable inputs; Browse just fills them. Re-run after
// each body re-render. Fires `change` after filling so the live preview syncs.
function wireBodyExtras(editorEl) {
  editorEl.querySelectorAll('.js-folder-browse').forEach((btn) => {
    if (btn.dataset.wired) return;
    btn.dataset.wired = '1';
    btn.addEventListener('click', async () => {
      const input = editorEl.querySelector(btn.dataset.input);
      try {
        const selected = await window.__TAURI__.dialog.open({
          directory: true,
          multiple: false,
          defaultPath: (input?.value || '').trim() || undefined,
        });
        if (selected && input) {
          input.value = selected;
          input.dispatchEvent(new Event('change', { bubbles: true }));
        }
      } catch (err) {
        console.error('folder picker failed:', err);
      }
    });
  });
}

// Read the inline config fields out of the form for the given type. Only
// non-empty values are persisted so the stored config stays lean.
function collectConfig(editorEl, typeKey) {
  const config = {};
  const timeoutRaw = editorEl.querySelector('.step-cfg-timeout')?.value?.trim();
  const timeout = timeoutRaw ? parseInt(timeoutRaw, 10) : NaN;

  if (typeKey === 'cli_agent') {
    const cli = editorEl.querySelector('.step-cfg-cli')?.value || '';
    if (cli) config.cli = cli;
    const model = editorEl.querySelector('.step-cfg-model')?.value?.trim();
    if (model) config.model = model;
    const workdir = editorEl.querySelector('.step-cfg-workdir')?.value?.trim();
    if (workdir) config.working_directory = workdir;
  } else if (typeKey === 'shell') {
    const cwd = editorEl.querySelector('.step-cfg-cwd')?.value?.trim();
    if (cwd) config.cwd = cwd;
    const shell = editorEl.querySelector('.step-cfg-shell')?.value?.trim();
    if (shell) config.shell = shell;
  } else if (typeKey === 'save_local') {
    const folder = editorEl.querySelector('.step-cfg-folder')?.value?.trim();
    if (folder) config.folder_path = folder;
    return config; // no timeout for save
  }

  if (!Number.isNaN(timeout) && timeout > 0) config.timeout_secs = timeout;
  return config;
}

// Sensible default template when the user adds a step or switches type.
function defaultTemplateFor(typeKey) {
  if (typeKey === 'shell') {
    return '# Whatever bash you want. Available env vars:\n#   $NBP_TRANSCRIPT, $NBP_PROCESSING_RESULT, $NBP_APP\n# stdout becomes this step\'s output.\n\necho "$NBP_PROCESSING_RESULT"';
  }
  if (typeKey === 'save_local') {
    // Save defaults to the raw transcript — the user tunes the template freely
    // (e.g. prepend {calendar_title} / {calendar_attendees}).
    return '{transcript}';
  }
  // CLI agent: first-step heuristic — transcript on step 1, else previous result.
  return pipelineState.pipelineEditorSteps.length <= 1
    ? '{transcript}'
    : '{processing_result}';
}

// Default step name when the user leaves Name blank. Derived from the type
// (CLI binary / "shell" / "save"), then de-duplicated against the other steps
// — names must be unique (they're the artifact filename + the pipeline
// validator rejects duplicates), so two save steps become "save" + "save-2".
function defaultStepName(typeKey, config, index) {
  const base = (typeKey === 'cli_agent'
    ? (config.cli || 'cli')
    : (typeKey === 'save_local' ? 'save' : 'shell'))
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 40) || `step-${index + 1}`;

  const taken = new Set(
    pipelineState.pipelineEditorSteps
      .filter((_, i) => i !== index)
      .map(s => s.name)
      .filter(Boolean)
  );
  if (!taken.has(base)) return base;
  let n = 2;
  while (taken.has(`${base}-${n}`)) n++;
  return `${base}-${n}`;
}
