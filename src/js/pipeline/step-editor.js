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
    hint: 'Runs in the working dir set above (default shell: /bin/bash). Stdout becomes this step\'s output, stderr goes to the run log only. Env vars: $NBP_TRANSCRIPT (full transcript), $NBP_PROCESSING_RESULT (previous step output, empty on step 1), $NBP_APP (Zoom / FaceTime / NBP / …). Read them with $VAR — placeholder substitution is NOT applied to shell scripts.',
    showPlaceholders: false,
    mono: true,
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

export async function addNewStep() {
  await refreshCliAvailability();
  const defaultType = 'cli_agent';
  // Pre-select the first installed CLI so the step is runnable out of the box.
  const firstCli = cliAvailabilityCache.find(c => c.installed)?.id || '';
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

      <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;">
        <div class="step-editor-row">
          <label>Name</label>
          <input class="step-name-input" value="${escapeHtml(step.name || '')}" placeholder="Auto-generated from type" />
        </div>
      </div>

      <div class="step-editor-actions">
        <button class="step-editor-done">Done</button>
      </div>
    </div>
  `;
}

// Body of the form below the Type row — the inline config + the prompt/script
// textarea. Differs by type.
function renderBody(typeKey, step) {
  if (typeKey === 'shell') return renderShellBody(step);
  if (typeKey === 'save_local') return renderSaveBody(step);
  return renderCliBody(step);
}

// --- CLI agent body -------------------------------------------------------
function renderCliBody(step) {
  const cfg = step.config || {};
  const copy = BODY_COPY.cli_agent;
  return `
    <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;display:flex;flex-direction:column;gap:12px;">
      ${renderCliDetectField(cfg.cli || '')}
      ${textField('step-cfg-model', 'Model', cfg.model || '', 'Optional. Free-text — CLI validates at runtime. Examples: claude → sonnet · codex → o3 · opencode → openai/gpt-4o · agy → gemini-3.1-pro.')}
      ${textField('step-cfg-workdir', 'Working dir', cfg.working_directory || '', 'Optional. Where the CLI runs — point at a repo so the agent can read its files. `~` expanded. Default: home directory.')}
      ${numberField('step-cfg-timeout', 'Timeout (sec)', cfg.timeout_secs, 'Default 300. The whole subprocess is killed at this limit.')}
    </div>
    ${renderTemplateField('cli_agent', step, copy)}
  `;
}

// --- Shell body -----------------------------------------------------------
function renderShellBody(step) {
  const cfg = step.config || {};
  const copy = BODY_COPY.shell;
  // env stored as { KEY: value }; rendered as KEY=value lines for editing.
  const envLines = (cfg.env && typeof cfg.env === 'object' && !Array.isArray(cfg.env))
    ? Object.entries(cfg.env).map(([k, v]) => `${k}=${v}`).join('\n')
    : '';
  return `
    <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;display:flex;flex-direction:column;gap:12px;">
      ${textField('step-cfg-cwd', 'Working dir', cfg.cwd || '', 'Required. Where the script runs. `~` expanded. Example: ~/work/notes', true)}
      ${textField('step-cfg-shell', 'Shell', cfg.shell || '', 'Binary that accepts -c <script>. Default /bin/bash. Use /bin/zsh, /usr/bin/env fish, etc. if you prefer.')}
      ${textareaField('step-cfg-env', 'Env vars (KEY=value per line)', envLines, 'Optional. Merged on top of NBP_TRANSCRIPT / NBP_APP / NBP_PROCESSING_RESULT (so you CAN override them — rare).', 3)}
      ${numberField('step-cfg-timeout', 'Timeout (sec)', cfg.timeout_secs, 'Default 120. Kills the subprocess if exceeded.')}
    </div>
    ${renderTemplateField('shell', step, copy)}
  `;
}

// --- Save-to-folder body --------------------------------------------------
// WHERE (folder, required) + WHAT (processing_result | transcript). The WHAT
// radio is materialised into step.template as `{processing_result}` /
// `{transcript}` on Done, so the engine's render path stays identical.
function renderSaveBody(step) {
  const cfg = step.config || {};
  const tpl = (step.template || '').trim();
  const isTranscript = tpl === '{transcript}';
  return `
    <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;display:flex;flex-direction:column;gap:12px;">
      ${textField('step-cfg-folder', 'Folder', cfg.folder_path || '', 'Required. Where the file lands. `~` expanded. Example: ~/Documents/Meetings. Saved as <date>-<pipeline>.md.', true)}
    </div>
    <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;">
      <div class="step-section-label">What to save</div>
      <div style="display:flex;flex-direction:column;gap:8px;margin-top:6px;">
        <label style="display:flex;align-items:flex-start;gap:8px;cursor:pointer;font-size:0.88rem;">
          <input type="radio" name="step-save-what" value="processing_result" ${!isTranscript ? 'checked' : ''} style="margin-top:3px;" />
          <span>
            <strong>Result of the previous step</strong>
            <span style="display:block;font-size:0.72rem;color:var(--text-secondary);opacity:0.85;">What the last step produced. Empty if this is the first step.</span>
          </span>
        </label>
        <label style="display:flex;align-items:flex-start;gap:8px;cursor:pointer;font-size:0.88rem;">
          <input type="radio" name="step-save-what" value="transcript" ${isTranscript ? 'checked' : ''} style="margin-top:3px;" />
          <span>
            <strong>Raw recording transcript</strong>
            <span style="display:block;font-size:0.72rem;color:var(--text-secondary);opacity:0.85;">The full transcribed audio, no post-processing.</span>
          </span>
        </label>
      </div>
    </div>
  `;
}

// Prompt (CLI) / script (Shell) textarea — freeform, with per-type copy.
function renderTemplateField(typeKey, step, copy) {
  const placeholdersLine = copy.showPlaceholders
    ? `<br>Placeholders: <code>{transcript}</code> raw recording transcript · <code>{processing_result}</code> previous step's output (empty on step 1) · <code>{app}</code> friendly app name (Zoom / FaceTime / NBP).`
    : '';
  const monoStyle = copy.mono ? 'font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:0.82rem;' : '';
  return `
    <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;">
      <div class="step-section-label">${escapeHtml(copy.label)}</div>
      <textarea class="step-template-textarea" rows="8" placeholder="${escapeHtml(copy.placeholder)}" style="${monoStyle}">${escapeHtml(step.template || '')}</textarea>
      <div style="font-size:0.72rem;color:var(--text-secondary);opacity:0.85;margin-top:6px;line-height:1.5;">
        ${escapeHtml(copy.hint)}${placeholdersLine}
      </div>
    </div>
  `;
}

// --- Field helpers --------------------------------------------------------
function fieldHint(hint) {
  return hint ? `<span style="font-size:0.72rem;color:var(--text-secondary);opacity:0.8;">${escapeHtml(hint)}</span>` : '';
}

function textField(cls, label, value, hint, required = false) {
  return `
    <label style="display:flex;flex-direction:column;gap:4px;">
      <span style="font-size:0.8rem;color:var(--text-secondary);">${escapeHtml(label)}${required ? ' *' : ''}</span>
      <input class="${cls}" type="text" value="${escapeHtml(value == null ? '' : String(value))}" />
      ${fieldHint(hint)}
    </label>
  `;
}

function numberField(cls, label, value, hint) {
  return `
    <label style="display:flex;flex-direction:column;gap:4px;">
      <span style="font-size:0.8rem;color:var(--text-secondary);">${escapeHtml(label)}</span>
      <input class="${cls}" type="number" value="${value == null ? '' : escapeHtml(String(value))}" />
      ${fieldHint(hint)}
    </label>
  `;
}

function textareaField(cls, label, value, hint, rows = 3) {
  return `
    <label style="display:flex;flex-direction:column;gap:4px;">
      <span style="font-size:0.8rem;color:var(--text-secondary);">${escapeHtml(label)}</span>
      <textarea class="${cls}" rows="${rows}" style="font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:0.8rem;">${escapeHtml(value || '')}</textarea>
      ${fieldHint(hint)}
    </label>
  `;
}

// CLI picker built from `check_cli_availability`. Only installed CLIs are
// pickable; missing ones list below with a copy-paste install command. A
// previously-saved CLI that's no longer installed is preserved with a ⚠.
function renderCliDetectField(currentValue) {
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
    ? `<select class="step-cfg-cli" disabled style="opacity:0.6;"><option value="">No CLI agents installed</option></select>`
    : `<select class="step-cfg-cli">${placeholderOpt}${staleOpt}${installedOpts}</select>`;

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
    <label style="display:flex;flex-direction:column;gap:4px;">
      <span style="font-size:0.8rem;color:var(--text-secondary);">CLI *</span>
      ${selectHTML}
      <span style="font-size:0.72rem;color:var(--text-secondary);opacity:0.8;">Only CLIs currently on your PATH show up. Install one to unlock it.</span>
      ${missingHTML}
    </label>
  `;
}

function wireFormEvents(editorEl, step, index) {
  // Close — drop empty drafts so abandoning a fresh "+ Add Step" cleans up.
  editorEl.querySelector('.step-editor-close').addEventListener('click', () => {
    if (!step.name && !step.template) {
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
    }
  });

  // Done — collect config + template + name, write the step, close.
  editorEl.querySelector('.step-editor-done').addEventListener('click', () => {
    const typeKey = typeSelect.value || step.step_type;
    // Save steps have no freeform textarea — the WHAT radio materialises into
    // a canonical placeholder so the engine renders it like any other step.
    const template = (typeKey === 'save_local')
      ? `{${editorEl.querySelector('input[name="step-save-what"]:checked')?.value || 'processing_result'}}`
      : (editorEl.querySelector('.step-template-textarea')?.value ?? '');
    const config = collectConfig(editorEl, typeKey);
    const nameInput = (editorEl.querySelector('.step-name-input')?.value || '').trim();

    step.step_type = typeKey;
    step.config = config;
    step.template = template;
    step.name = nameInput || defaultStepName(typeKey, config, index);

    pipelineState.setEditingStepIndex(null);
    closeStepEditorPanel();
    renderPipelineSteps();
    maybeAutoName();
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
    const envRaw = editorEl.querySelector('.step-cfg-env')?.value || '';
    const env = {};
    for (const line of envRaw.split('\n')) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) continue;
      const eq = trimmed.indexOf('=');
      if (eq <= 0) continue;
      const key = trimmed.slice(0, eq).trim();
      const val = trimmed.slice(eq + 1).trim();
      if (key) env[key] = val;
    }
    if (Object.keys(env).length > 0) config.env = env;
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
    // Save defaults to the previous step's output (the common "process then
    // save" shape); the WHAT radio lets the user switch to the transcript.
    return '{processing_result}';
  }
  // CLI agent: first-step heuristic — transcript on step 1, else previous result.
  return pipelineState.pipelineEditorSteps.length <= 1
    ? '{transcript}'
    : '{processing_result}';
}

// Default step name from the type + its config when the user hasn't typed one.
function defaultStepName(typeKey, config, index) {
  const base = typeKey === 'cli_agent'
    ? (config.cli || 'cli')
    : (typeKey === 'save_local' ? 'save' : 'shell');
  return base
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 40) || `step-${index + 1}`;
}
