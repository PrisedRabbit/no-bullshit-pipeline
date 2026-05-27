// step-editor.js — Pipeline step editor (Connection model)
//
// One step = pick a type → pick a Connection of that type → write a template.
// All per-connector auth/target lives in the Connection (Settings →
// Connections); the step composes pre-built bricks. See
// `docs/connections-model.md` for the model.

import { escapeHtml } from '../core/utils.js';
import { showToast } from '../ui/toast.js';
import * as pipelineState from './state.js';
import { getConnectionTypes, getLoadedConnections, loadConnections } from '../connections/index.js';
import { closeStepEditorPanel, renderPipelineSteps } from './editor.js';
import { maybeAutoName } from './delivery-options.js';

// Refresh the Connections list when the editor opens. Lightweight — Tauri
// command just returns the in-memory settings.connections vector.
let connectionsFreshness = 0;
async function ensureConnectionsLoaded() {
  if (connectionsFreshness === 0 || getLoadedConnections().length === 0) {
    await loadConnections();
    connectionsFreshness = Date.now();
  }
}

export async function addNewStep() {
  // Default to the first type that has at least one Connection so the user
  // lands in a useful state. If nothing is configured yet, default to
  // CliAgent (most common path) and let the form show the empty-state link.
  await ensureConnectionsLoaded();
  const conns = getLoadedConnections();
  const firstTypeWithConn = getConnectionTypes().find(t => conns.some(c => (c.connection_type || c.type) === t.key));
  const defaultType = firstTypeWithConn?.key || 'cli_agent';
  const defaultConn = conns.find(c => (c.connection_type || c.type) === defaultType);

  const step = {
    name: '',
    connection_type: defaultType,
    connection_id: defaultConn?.id || '',
    // First step typically reads the transcript; later steps usually want the
    // previous Processing output. Engine renders missing keys as empty string,
    // so this is just a sensible starting nudge — user can change anything.
    template: pipelineState.pipelineEditorSteps.length === 0
      ? '{transcript}'
      : '{processing_result}',
    description: null,
  };
  pipelineState.pipelineEditorSteps.push(step);
  renderPipelineSteps();
  showStepEditor(pipelineState.pipelineEditorSteps.length - 1);
}

export async function showStepEditor(index) {
  const step = pipelineState.pipelineEditorSteps[index];
  if (!step) return;
  await ensureConnectionsLoaded();

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
  const types = getConnectionTypes();
  const currentType = step.connection_type || step.type || 'cli_agent';
  const typeOptions = types.map(t =>
    `<option value="${escapeHtml(t.key)}"${t.key === currentType ? ' selected' : ''}>${escapeHtml(t.label)}</option>`
  ).join('');

  const connectionPickerHTML = renderConnectionPicker(currentType, step.connection_id || '');

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
        <div class="step-editor-row step-connection-row">
          <label>Connection</label>
          <div class="step-connection-host" style="flex:1;min-width:0;">${connectionPickerHTML}</div>
        </div>
      </div>

      <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;">
        <div class="step-section-label">Template</div>
        <textarea class="step-template-textarea" rows="8" placeholder="What to send to this connector. Use the placeholders below.">${escapeHtml(step.template || '')}</textarea>
        <div style="font-size:0.72rem;color:var(--text-secondary);opacity:0.8;margin-top:6px;line-height:1.5;">
          Available placeholders:
          <code>{transcript}</code> · the raw recording transcript ·
          <code>{app}</code> · friendly app name (Zoom / FaceTime / NBP) ·
          <code>{processing_result}</code> · output of the immediately previous processing step (empty for step 1).
        </div>
      </div>

      <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;">
        <div class="step-editor-row">
          <label>Name</label>
          <input class="step-name-input" value="${escapeHtml(step.name || '')}" placeholder="Auto-generated from connection" />
        </div>
      </div>

      <div class="step-editor-actions">
        <button class="step-editor-done">Done</button>
      </div>
    </div>
  `;
}

function renderConnectionPicker(typeKey, currentId) {
  const matches = getLoadedConnections().filter(c => (c.connection_type || c.type) === typeKey);
  if (matches.length === 0) {
    return `
      <div style="display:flex;flex-direction:column;gap:4px;">
        <div style="font-size:0.8rem;color:var(--text-secondary);">No ${escapeHtml(typeKey)} connection yet.</div>
        <a href="#" class="step-create-connection-link" data-type="${escapeHtml(typeKey)}" style="font-size:0.8rem;color:var(--accent);text-decoration:underline;">Create one in Settings → Connections</a>
      </div>
    `;
  }

  // Pre-select rule: when only one Connection of this type exists, lock it
  // in even if the step had a stale id from a different type. Saves a click
  // for the most common single-Slack / single-Notion setups.
  const effectiveId = matches.length === 1 ? matches[0].id : (currentId || '');
  const options = matches.map(c =>
    `<option value="${escapeHtml(c.id)}"${c.id === effectiveId ? ' selected' : ''}>${escapeHtml(c.name)}</option>`
  ).join('');

  return `<select class="step-connection-select">${options}</select>`;
}

function wireFormEvents(editorEl, step, index) {
  // Close — drop empty drafts so abandoning a fresh "+ Add Step" cleans up.
  editorEl.querySelector('.step-editor-close').addEventListener('click', () => {
    if (!step.name && !step.connection_id) {
      pipelineState.pipelineEditorSteps.splice(index, 1);
    }
    pipelineState.setEditingStepIndex(null);
    closeStepEditorPanel();
    renderPipelineSteps();
    maybeAutoName();
  });

  // Type change — re-render the Connection picker for the new type.
  const typeSelect = editorEl.querySelector('.step-type-select');
  typeSelect.addEventListener('change', () => {
    const newType = typeSelect.value;
    const host = editorEl.querySelector('.step-connection-host');
    if (host) host.innerHTML = renderConnectionPicker(newType, '');
  });

  // Done — write the step + close.
  editorEl.querySelector('.step-editor-done').addEventListener('click', () => {
    const typeKey = editorEl.querySelector('.step-type-select')?.value || step.connection_type;
    const connSelect = editorEl.querySelector('.step-connection-select');
    const connId = connSelect?.value || '';
    if (!connSelect || !connId) {
      showToast(`Pick a ${typeKey} Connection first, or create one in Settings.`, 'error');
      return;
    }

    const template = editorEl.querySelector('.step-template-textarea')?.value ?? '';
    const nameInput = (editorEl.querySelector('.step-name-input')?.value || '').trim();

    step.connection_type = typeKey;
    step.connection_id = connId;
    step.template = template;
    step.name = nameInput || defaultStepName(typeKey, connId, index);

    pipelineState.setEditingStepIndex(null);
    closeStepEditorPanel();
    renderPipelineSteps();
    maybeAutoName();
  });
}

// Generate a sensible default name when the user hasn't typed one. Uses the
// Connection name so similar steps with different targets disambiguate
// themselves in the chip bar.
function defaultStepName(typeKey, connId, index) {
  const conn = getLoadedConnections().find(c => c.id === connId);
  const base = conn?.name || typeKey;
  // Lowercase + hyphenate to fit the chip-name aesthetic.
  return base
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 40) || `step-${index + 1}`;
}
