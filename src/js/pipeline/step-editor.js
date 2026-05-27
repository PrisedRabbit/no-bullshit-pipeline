// step-editor.js — Pipeline step editor (Connection model).
//
// One step = pick a type → pick a Connection of that type → describe what
// goes to the connection. Three render modes driven by the chosen type:
//
//   1. Processing step (CLI agent / Shell) → freeform textarea with
//      placeholder substitution. The CONTENT differs by type:
//        - CLI agent : the prompt sent to the agent (with the recording
//                       transcript / previous step output substituted in).
//        - Shell     : the text piped to the configured command's stdin.
//
//   2. Delivery step (Slack / Telegram / Webhook / SaveLocal) → no
//      freeform text. Just a "Send" radio picker that writes
//      `{transcript}` or `{processing_result}` into step.template under
//      the hood. Anything richer than that has no business in delivery —
//      either send the raw recording or the previous step's output.
//
//   3. No Connection of the picked type yet → form collapses to a single
//      "Create one in Settings → Connections" link. Save is blocked.
//
// All auth / target / model config lives in the Connection (Settings →
// Connections); the step composes pre-built bricks. See
// `docs/connections-model.md`.

import { escapeHtml } from '../core/utils.js';
import { showToast } from '../ui/toast.js';
import * as pipelineState from './state.js';
import { getConnectionTypes, getLoadedConnections, loadConnections } from '../connections/index.js';
import { closeStepEditorPanel, renderPipelineSteps } from './editor.js';
import { maybeAutoName } from './delivery-options.js';

const PROCESSING_TYPES = new Set(['cli_agent', 'shell']);

/// Per-type copy used to dress the textarea for Processing steps. Keeps
/// labels honest to what the connector actually does with the content.
const PROCESSING_COPY = {
  cli_agent: {
    label: 'Prompt for the agent',
    placeholder: 'What to ask the CLI agent. Use {transcript} or {processing_result} to inline content.',
    hint: 'Sent to the agent as the prompt body. Placeholders below get substituted before the agent sees them.',
    showPlaceholders: true,
  },
  shell: {
    label: 'Shell script',
    placeholder: 'echo "$NBP_TRANSCRIPT" | jq -R .   # any bash you like — multi-line ok',
    hint: 'Runs in the cwd set on the Connection (default shell: /bin/bash). Stdout becomes this step\'s output, stderr goes to the run log only. Available env vars: $NBP_TRANSCRIPT (full transcript), $NBP_PROCESSING_RESULT (previous step output, empty on step 1), $NBP_APP (Zoom / FaceTime / NBP / …). Read them with $VAR — placeholder substitution is NOT applied to shell scripts (avoids shell-injection from transcript content).',
    showPlaceholders: false,
  },
};

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
    // Defer to defaultTemplateFor so Shell gets script-mode skeleton (env
    // vars demo) and CLI gets placeholder skeleton — keeping the two ideas
    // in one place.
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

  // Group by role so the picker mirrors the Connections-tab layout:
  // Processing types feed their output downstream; Delivery types are
  // terminal. Without optgroups the dropdown was a flat soup that didn't
  // communicate the difference (and CLI agent ended up next to Save Local).
  const optionFor = (t) =>
    `<option value="${escapeHtml(t.key)}"${t.key === currentType ? ' selected' : ''}>${escapeHtml(t.label)}</option>`;
  const processingOpts = types.filter(t => t.role === 'processing').map(optionFor).join('');
  const deliveryOpts = types.filter(t => t.role === 'delivery').map(optionFor).join('');
  const typeOptions = `
    <optgroup label="Processing">${processingOpts}</optgroup>
    <optgroup label="Delivery">${deliveryOpts}</optgroup>
  `;

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
          <input class="step-name-input" value="${escapeHtml(step.name || '')}" placeholder="Auto-generated from connection" />
        </div>
      </div>

      <div class="step-editor-actions">
        <button class="step-editor-done">Done</button>
      </div>
    </div>
  `;
}

// Body of the form below the Type row — depends on the chosen type. Three
// flavours: empty-state (no connection), processing (textarea with prompt /
// stdin label), delivery (simple "what to send" picker).
function renderBody(typeKey, step) {
  const matches = getLoadedConnections().filter(c => (c.connection_type || c.type) === typeKey);
  const typeLabel = (getConnectionTypes().find(t => t.key === typeKey)?.label) || typeKey;

  // --- Empty state: no Connection of this type yet ----------------------
  if (matches.length === 0) {
    return `
      <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;">
        <div style="text-align:center;padding:24px 8px;display:flex;flex-direction:column;gap:8px;">
          <div style="font-size:0.9rem;color:var(--text-secondary);">No <strong>${escapeHtml(typeLabel)}</strong> Connection configured yet.</div>
          <a href="#" class="step-create-connection-link" data-type="${escapeHtml(typeKey)}" style="font-size:0.85rem;color:var(--accent);text-decoration:underline;">Open Settings → Connections to create one</a>
          <div style="font-size:0.72rem;color:var(--text-secondary);opacity:0.7;">Once it exists, come back here to wire it into this step.</div>
        </div>
      </div>
    `;
  }

  // Pre-select the only Connection of this type — typical "single Slack /
  // single Notion" setups don't need an extra click.
  const effectiveId = matches.length === 1 ? matches[0].id : (step.connection_id || '');
  const connOptions = matches.map(c =>
    `<option value="${escapeHtml(c.id)}"${c.id === effectiveId ? ' selected' : ''}>${escapeHtml(c.name)}</option>`
  ).join('');

  const connectionRow = `
    <div class="step-section">
      <div class="step-editor-row">
        <label>Connection</label>
        <select class="step-connection-select" style="flex:1;min-width:0;">${connOptions}</select>
      </div>
    </div>
  `;

  if (PROCESSING_TYPES.has(typeKey)) {
    return connectionRow + renderProcessingBody(typeKey, step);
  }
  return connectionRow + renderDeliveryBody(step);
}

// Processing body — freeform textarea, label/hint/placeholder per CLI vs
// Shell so the user knows whether they're writing a prompt or a script.
function renderProcessingBody(typeKey, step) {
  const copy = PROCESSING_COPY[typeKey] || PROCESSING_COPY.cli_agent;
  // CLI agent uses `{placeholder}` substitution. Shell uses env vars
  // (NBP_TRANSCRIPT etc.) — the placeholder line would be misleading there.
  const placeholdersLine = copy.showPlaceholders
    ? `<br>Placeholders: <code>{transcript}</code> raw recording transcript · <code>{processing_result}</code> previous processing step's output (empty on step 1) · <code>{app}</code> friendly app name (Zoom / FaceTime / NBP).`
    : '';
  return `
    <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;">
      <div class="step-section-label">${escapeHtml(copy.label)}</div>
      <textarea class="step-template-textarea" rows="8" placeholder="${escapeHtml(copy.placeholder)}" style="${typeKey === 'shell' ? 'font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:0.82rem;' : ''}">${escapeHtml(step.template || '')}</textarea>
      <div style="font-size:0.72rem;color:var(--text-secondary);opacity:0.85;margin-top:6px;line-height:1.5;">
        ${escapeHtml(copy.hint)}${placeholdersLine}
      </div>
    </div>
  `;
}

// Delivery body — no freeform. Just pick which canonical variable gets
// sent. The picker is rendered as radios for one-click switching; on save
// the choice is materialised as `{transcript}` or `{processing_result}` in
// step.template so the engine's rendering path stays identical to
// processing steps. Anything else would be premature flexibility for a
// 3-user tool — see review feedback на «free-text для delivery — нахуя?».
function renderDeliveryBody(step) {
  // Existing step might carry an arbitrary template (older pipelines /
  // hand-edited). Match it to a canonical choice; fall back to
  // "processing_result" if the template is anything else — that's the
  // useful default for most delivery steps.
  const tpl = (step.template || '').trim();
  const isTranscript = tpl === '{transcript}';
  return `
    <div class="step-section" style="border-top:1px solid var(--border-color);padding-top:12px;">
      <div class="step-section-label">What to send</div>
      <div style="display:flex;flex-direction:column;gap:8px;margin-top:6px;">
        <label style="display:flex;align-items:flex-start;gap:8px;cursor:pointer;font-size:0.88rem;">
          <input type="radio" name="step-delivery-source" value="processing_result" ${!isTranscript ? 'checked' : ''} style="margin-top:3px;" />
          <span>
            <strong>Result of the previous step</strong>
            <span style="display:block;font-size:0.72rem;color:var(--text-secondary);opacity:0.85;">What the last processing step produced. Empty if there was none.</span>
          </span>
        </label>
        <label style="display:flex;align-items:flex-start;gap:8px;cursor:pointer;font-size:0.88rem;">
          <input type="radio" name="step-delivery-source" value="transcript" ${isTranscript ? 'checked' : ''} style="margin-top:3px;" />
          <span>
            <strong>Raw recording transcript</strong>
            <span style="display:block;font-size:0.72rem;color:var(--text-secondary);opacity:0.85;">The full transcribed audio, no post-processing.</span>
          </span>
        </label>
      </div>
    </div>
  `;
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

  // Type change — re-render the body for the new type (empty-state /
  // processing / delivery picker).
  const typeSelect = editorEl.querySelector('.step-type-select');
  typeSelect.addEventListener('change', () => {
    const newType = typeSelect.value;
    const host = editorEl.querySelector('.step-body-host');
    if (host) {
      host.dataset.currentType = newType;
      // Reset connection_id when switching types — old id belongs to a
      // different type and would render as "stale". User picks fresh.
      const shadow = { ...step, connection_id: '', template: defaultTemplateFor(newType) };
      host.innerHTML = renderBody(newType, shadow);
    }
  });

  // Done — write the step + close.
  editorEl.querySelector('.step-editor-done').addEventListener('click', () => {
    const typeKey = editorEl.querySelector('.step-type-select')?.value || step.connection_type;
    const connSelect = editorEl.querySelector('.step-connection-select');
    const connId = connSelect?.value || '';
    if (!connSelect) {
      showToast(`No ${typeKey} Connection exists yet — create one in Settings before saving this step.`, 'error');
      return;
    }
    if (!connId) {
      showToast(`Pick a ${typeKey} Connection first.`, 'error');
      return;
    }

    let template;
    if (PROCESSING_TYPES.has(typeKey)) {
      template = editorEl.querySelector('.step-template-textarea')?.value ?? '';
    } else {
      // Delivery: materialise the radio choice into the canonical template
      // string. Engine's render path doesn't know the difference.
      const choice = editorEl.querySelector('input[name="step-delivery-source"]:checked')?.value
        || 'processing_result';
      template = `{${choice}}`;
    }

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

// Sensible default template when the user switches type — keeps the
// previous content from looking weird (e.g. a CLI prompt suddenly shown
// under "What to send" radio of delivery).
function defaultTemplateFor(typeKey) {
  if (typeKey === 'shell') {
    // Shell uses env vars, not {placeholders}. Default = noop passthrough so
    // the user has a working starting point that demonstrates the env vars.
    return '# Whatever bash you want. Available env vars:\n#   $NBP_TRANSCRIPT, $NBP_PROCESSING_RESULT, $NBP_APP\n# stdout becomes this step\'s output.\n\necho "$NBP_PROCESSING_RESULT"';
  }
  if (PROCESSING_TYPES.has(typeKey)) {
    // CLI agent: first-step heuristic — transcript on step 1, else result.
    return pipelineState.pipelineEditorSteps.length <= 1
      ? '{transcript}'
      : '{processing_result}';
  }
  // Delivery picker materialises its own value; keep a safe fallback.
  return '{processing_result}';
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
