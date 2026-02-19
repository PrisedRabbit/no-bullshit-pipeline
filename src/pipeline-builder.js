// ===== PIPELINE DEFINITION MANAGEMENT =====
// Globals from main.js (loaded before this script): escapeHtml, invoke, allPromptTemplates, slackIntegrations, updateSidebarCounts
// Globals from integrations-settings.js (loaded before this script): notionProfiles, linearProfiles, savePathIntegrations, webhookProfiles (all via typeof guard)

var allPipelineDefs = []; // var so main.js updateSidebarCounts() can access allPipelineDefs.length
let editingPipelineDef = null; // null = new, string = editing name
let pipelineEditorSteps = []; // Working copy of steps
let sortableInstance = null;

const pipelineDefsListEl = document.getElementById('pipeline-defs-list');
const addPipelineDefBtn = document.getElementById('add-pipeline-def-btn');
const pipelineEditor = document.getElementById('pipeline-editor');
const pipelineEditorTitle = document.getElementById('pipeline-editor-title');
const pipelineEditorName = document.getElementById('pipeline-editor-name');
const pipelineEditorDesc = document.getElementById('pipeline-editor-desc');
const pipelineStepsListEl = document.getElementById('pipeline-steps-list');
const addPipelineStepBtn = document.getElementById('add-pipeline-step-btn');
const pipelinePreviewEl = document.getElementById('pipeline-preview');
const savePipelineDefBtn = document.getElementById('save-pipeline-def-btn');
const deletePipelineDefBtn = document.getElementById('delete-pipeline-def-btn');
const closePipelineEditorBtn = document.getElementById('close-pipeline-editor');

const PROCESSING_PRESETS = [
  {
    label: 'Meeting Notes',
    icon: '📝',
    step: {
      name: 'meeting-notes',
      connector: 'llm',
      input: 'transcript',
      config: { prompt_template: 'meeting-notes' },
      description: 'Extract attendees, decisions, action items'
    }
  },
  {
    label: 'Action Items',
    icon: '✅',
    step: {
      name: 'action-items',
      connector: 'llm',
      input: 'transcript',
      config: { prompt_template: 'action-items' },
      description: 'Extract tasks and owners'
    }
  },
  {
    label: 'Summary',
    icon: '📋',
    step: {
      name: 'summary',
      connector: 'llm',
      input: 'transcript',
      config: { prompt_template: 'summary' },
      description: 'Concise summary of key points'
    }
  },
  {
    label: 'Structure',
    icon: '🏗',
    step: {
      name: 'structure',
      connector: 'llm',
      input: 'transcript',
      config: { prompt_template: 'structure' },
      description: 'Organize content into sections'
    }
  },
  {
    label: 'Custom Prompt',
    icon: '✏️',
    step: null  // handled specially in Plan 05-03
  }
];

let pickerVisible = false;

function buildDeliveryOptions() {
  const options = [];
  const profiles = (typeof notionProfiles !== 'undefined') ? notionProfiles : [];
  for (const p of profiles) {
    options.push({
      label: p.name + ' (Notion)',
      icon: '📓',
      step: {
        name: 'send-to-' + p.name.toLowerCase().replace(/\s+/g, '-'),
        connector: 'notion',
        input: 'transcript',
        config: { integration_id: p.id },
        description: 'Send to ' + p.name
      }
    });
  }
  const linProfiles = (typeof linearProfiles !== 'undefined') ? linearProfiles : [];
  for (const p of linProfiles) {
    options.push({
      label: p.name + ' (Linear)',
      icon: '🔷',
      step: {
        name: 'create-in-' + p.name.toLowerCase().replace(/\s+/g, '-'),
        connector: 'linear',
        input: 'transcript',
        config: { integration_id: p.id },
        description: 'Create issue in ' + p.name
      }
    });
  }
  const savePaths = (typeof savePathIntegrations !== 'undefined') ? savePathIntegrations : [];
  for (const sp of savePaths) {
    options.push({
      label: sp.name + ' (Save)',
      icon: '💾',
      step: {
        name: 'save-to-' + sp.name.toLowerCase().replace(/\s+/g, '-'),
        connector: 'save',
        input: 'transcript',
        config: { save_path_id: sp.id, path: sp.path },
        description: 'Save to ' + sp.name
      }
    });
  }
  const slackWs = (typeof slackIntegrations !== 'undefined') ? slackIntegrations : {};
  for (const [id, data] of Object.entries(slackWs)) {
    options.push({
      label: (data.name || id) + ' (Slack)',
      icon: '💬',
      step: {
        name: 'send-to-' + (data.name || id).toLowerCase().replace(/\s+/g, '-'),
        connector: 'slack',
        input: 'transcript',
        config: { integration_id: id },
        description: 'Send to ' + (data.name || id)
      }
    });
  }
  const whooks = (typeof webhookProfiles !== 'undefined') ? webhookProfiles : [];
  for (const wh of whooks) {
    options.push({
      label: wh.name + ' (Webhook)',
      icon: '🔗',
      step: {
        name: 'send-to-' + wh.name.toLowerCase().replace(/\s+/g, '-'),
        connector: 'webhook',
        input: 'transcript',
        config: { integration_id: wh.id },
        description: 'Send to ' + wh.name
      }
    });
  }
  return options;
}

function togglePicker() {
  const existing = document.getElementById('step-picker');
  if (existing) {
    existing.remove();
    pickerVisible = false;
    return;
  }
  showPicker();
}

function showPicker() {
  const existing = document.getElementById('step-picker');
  if (existing) existing.remove();

  const picker = document.createElement('div');
  picker.id = 'step-picker';
  picker.className = 'step-picker';

  // Processing section
  let html = '<div class="step-picker-section"><div class="step-picker-section-title">Processing</div>';
  for (const preset of PROCESSING_PRESETS) {
    html += `<button class="step-picker-option" data-preset-label="${escapeHtml(preset.label)}">
      <span class="step-picker-icon">${preset.icon}</span>
      <span class="step-picker-label">${escapeHtml(preset.label)}</span>
    </button>`;
  }
  html += '</div>';

  // Delivery section
  const deliveryOptions = buildDeliveryOptions();
  html += '<div class="step-picker-section"><div class="step-picker-section-title">Delivery</div>';
  if (deliveryOptions.length === 0) {
    html += '<div class="step-picker-empty">Connect integrations in Settings &gt; Integrations to enable delivery steps.</div>';
  } else {
    for (let i = 0; i < deliveryOptions.length; i++) {
      const opt = deliveryOptions[i];
      html += `<button class="step-picker-option" data-delivery-index="${i}">
        <span class="step-picker-icon">${opt.icon}</span>
        <span class="step-picker-label">${escapeHtml(opt.label)}</span>
      </button>`;
    }
  }
  html += '</div>';

  picker.innerHTML = html;

  // Insert picker right after the add-pipeline-step-btn
  addPipelineStepBtn.parentNode.insertBefore(picker, addPipelineStepBtn.nextSibling);
  pickerVisible = true;

  // Wire preset clicks
  picker.querySelectorAll('[data-preset-label]').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const label = btn.dataset.presetLabel;
      const preset = PROCESSING_PRESETS.find(p => p.label === label);
      if (!preset) return;
      if (preset.step === null) {
        closePicker();
        showCustomPromptForm();
        return;
      } else {
        addPresetStep(preset);
      }
    });
  });

  // Wire delivery clicks
  picker.querySelectorAll('[data-delivery-index]').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const idx = parseInt(btn.dataset.deliveryIndex);
      const opt = deliveryOptions[idx];
      if (opt) addPresetStep(opt);
    });
  });

  // Dismiss picker on outside click
  setTimeout(() => {
    document.addEventListener('click', dismissPicker);
  }, 0);
}

function dismissPicker(e) {
  const picker = document.getElementById('step-picker');
  if (picker && !picker.contains(e.target) && e.target !== addPipelineStepBtn) {
    picker.remove();
    pickerVisible = false;
    document.removeEventListener('click', dismissPicker);
  }
}

function closePicker() {
  const picker = document.getElementById('step-picker');
  if (picker) picker.remove();
  pickerVisible = false;
  document.removeEventListener('click', dismissPicker);
}

function addPresetStep(preset) {
  const step = JSON.parse(JSON.stringify(preset.step));
  pipelineEditorSteps.push(step);
  fixStepInputs();
  renderPipelineSteps();
  renderPipelinePreview();
  closePicker();
}

function showCustomPromptForm() {
  // Remove any existing form
  const existingForm = document.querySelector('.custom-prompt-form');
  if (existingForm) existingForm.remove();

  const formEl = document.createElement('div');
  formEl.className = 'custom-prompt-form';
  formEl.innerHTML = `
    <div class="custom-prompt-header">Custom Prompt Step</div>
    <textarea class="custom-prompt-textarea" rows="4"
      placeholder="Write your prompt here. Use {transcript} for the input text."></textarea>
    <label class="custom-prompt-save-label">
      <input type="checkbox" class="custom-prompt-save-checkbox" />
      Save as reusable template
    </label>
    <div class="custom-prompt-name-row" style="display:none;">
      <input type="text" class="custom-prompt-name-input settings-input-text" placeholder="Template name (e.g. weekly-report)" />
    </div>
    <div class="custom-prompt-actions">
      <button class="mini-action-btn custom-prompt-cancel">Cancel</button>
      <button class="mini-action-btn primary custom-prompt-confirm">Add Step</button>
    </div>
  `;

  // Insert form after the step list
  pipelineStepsListEl.parentNode.insertBefore(formEl, pipelineStepsListEl.nextSibling);

  // Wire checkbox toggle for name input
  const checkbox = formEl.querySelector('.custom-prompt-save-checkbox');
  const nameRow = formEl.querySelector('.custom-prompt-name-row');
  checkbox.addEventListener('change', () => {
    nameRow.style.display = checkbox.checked ? 'block' : 'none';
  });

  // Cancel
  formEl.querySelector('.custom-prompt-cancel').addEventListener('click', () => {
    formEl.remove();
  });

  // Confirm — Add Step
  formEl.querySelector('.custom-prompt-confirm').addEventListener('click', async () => {
    const promptText = formEl.querySelector('.custom-prompt-textarea').value.trim();
    if (!promptText) {
      alert('Prompt text is required');
      return;
    }

    const saveAsTemplate = checkbox.checked;
    let stepConfig = {};

    if (saveAsTemplate) {
      const templateName = formEl.querySelector('.custom-prompt-name-input').value.trim();
      if (!templateName) {
        alert('Template name is required when saving as template');
        return;
      }
      // Save template to backend
      try {
        await invoke('save_prompt_template', {
          template: {
            name: templateName,
            description: 'Custom prompt template',
            prompt: promptText,
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString()
          }
        });
        // Reload templates so the step editor can see the new template
        if (typeof loadPromptTemplates === 'function') await loadPromptTemplates();
        stepConfig = { prompt_template: templateName };
      } catch (err) {
        alert('Failed to save template: ' + err);
        return;
      }
    } else {
      // Inline prompt — no template saved
      stepConfig = { prompt_inline: promptText };
    }

    const step = {
      name: saveAsTemplate
        ? formEl.querySelector('.custom-prompt-name-input').value.trim()
        : 'custom-prompt',
      connector: 'llm',
      input: 'transcript',
      config: stepConfig,
      description: saveAsTemplate ? null : promptText.substring(0, 60) + (promptText.length > 60 ? '...' : '')
    };

    pipelineEditorSteps.push(step);
    fixStepInputs();
    formEl.remove();
    renderPipelineSteps();
    renderPipelinePreview();
  });

  // Focus textarea
  formEl.querySelector('.custom-prompt-textarea').focus();
}

async function loadPipelineDefs() {
  try {
    allPipelineDefs = await invoke('list_pipelines');
    renderPipelineDefsList();
    updateSidebarCounts();
    if (typeof renderPipelineChips === 'function') renderPipelineChips();
    if (typeof populateDefaultPipelineSelect === 'function') populateDefaultPipelineSelect();
  } catch (err) {
    console.error('Failed to load pipelines:', err);
  }
}

function renderPipelineDefsList() {
  if (!pipelineDefsListEl) return;
  if (allPipelineDefs.length === 0) {
    pipelineDefsListEl.innerHTML = '<div style="color: var(--text-secondary); opacity: 0.6; font-size: 0.85rem;">No pipelines yet.</div>';
    return;
  }
  pipelineDefsListEl.innerHTML = allPipelineDefs.map(p => {
    const safeName = escapeHtml(p.name);
    const safeDesc = escapeHtml(p.description || '');
    return `
    <div class="pipeline-def-item" data-name="${safeName}">
      <div class="pipeline-def-info">
        <div class="pipeline-def-name">${safeName}</div>
        <div class="pipeline-def-desc">${safeDesc} &middot; ${p.steps.length} step${p.steps.length !== 1 ? 's' : ''}</div>
      </div>
    </div>
  `;
  }).join('');

  pipelineDefsListEl.querySelectorAll('.pipeline-def-item').forEach(el => {
    el.addEventListener('click', () => openPipelineEditor(el.dataset.name));
  });
}

function openPipelineEditor(name) {
  if (!pipelineEditor) return;
  if (name) {
    const p = allPipelineDefs.find(p => p.name === name);
    if (!p) return;
    editingPipelineDef = name;
    pipelineEditorTitle.textContent = 'Edit Pipeline';
    pipelineEditorName.value = p.name;
    pipelineEditorDesc.value = p.description || '';
    pipelineEditorSteps = JSON.parse(JSON.stringify(p.steps));
    if (deletePipelineDefBtn) deletePipelineDefBtn.style.display = 'inline-block';
  } else {
    editingPipelineDef = null;
    pipelineEditorTitle.textContent = 'New Pipeline';
    pipelineEditorName.value = '';
    pipelineEditorDesc.value = '';
    pipelineEditorSteps = [];
    if (deletePipelineDefBtn) deletePipelineDefBtn.style.display = 'none';
  }
  pipelineEditor.style.display = 'block';
  renderPipelineSteps();
  initSortable();
  renderPipelinePreview();
  pipelineEditorName.focus();
}

function closePipelineEditor() {
  if (pipelineEditor) pipelineEditor.style.display = 'none';
  editingPipelineDef = null;
  pipelineEditorSteps = [];
  if (sortableInstance) {
    sortableInstance.destroy();
    sortableInstance = null;
  }
}

function initSortable() {
  if (sortableInstance) {
    sortableInstance.destroy();
    sortableInstance = null;
  }
  if (pipelineEditorSteps.length > 0 && pipelineStepsListEl) {
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
}

function renderPipelineSteps() {
  if (!pipelineStepsListEl) return;
  if (pipelineEditorSteps.length === 0) {
    pipelineStepsListEl.innerHTML = '<div style="color: var(--text-secondary); opacity: 0.5; font-size: 0.85rem;">No steps. Click "+ Add Step" to begin.</div>';
    return;
  }

  pipelineStepsListEl.innerHTML = pipelineEditorSteps.map((step, i) => {
    const inputLabel = step.input === 'transcript' ? 'transcript' : escapeHtml(step.input);
    const safeName = escapeHtml(step.name || 'Unnamed');
    const safeConnector = escapeHtml(step.connector);
    const safeDesc = step.description ? escapeHtml(step.description) : '';
    return `
      <div class="pipeline-step-item" data-index="${i}">
        <span class="step-drag-handle">&#9776;</span>
        <span class="step-number">${i + 1}</span>
        <div class="step-info" data-index="${i}" style="cursor: pointer;">
          <div class="step-name-row">
            <span class="step-name">${safeName}</span>
            <span class="step-connector-badge">${safeConnector}</span>
          </div>
          ${safeDesc ? `<div class="step-description">${safeDesc}</div>` : ''}
          <div class="step-input-label">input: ${inputLabel}</div>
        </div>
        <button class="step-remove-btn" data-index="${i}" title="Remove step">&times;</button>
      </div>
    `;
  }).join('');

  // Re-initialize SortableJS after innerHTML replacement
  if (sortableInstance) sortableInstance.destroy();
  if (pipelineEditorSteps.length > 0) {
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

  // Click step to edit
  pipelineStepsListEl.querySelectorAll('.step-info').forEach(el => {
    el.addEventListener('click', () => showStepEditor(parseInt(el.dataset.index)));
  });

  // Remove step
  pipelineStepsListEl.querySelectorAll('.step-remove-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const idx = parseInt(btn.dataset.index);
      pipelineEditorSteps.splice(idx, 1);
      fixStepInputs();
      renderPipelineSteps();
      renderPipelinePreview();
    });
  });
}

function fixStepInputs() {
  // Fix input references: first step must reference 'transcript'
  // Others can reference 'transcript' or a previous step name
  for (let i = 0; i < pipelineEditorSteps.length; i++) {
    const step = pipelineEditorSteps[i];
    if (i === 0) {
      step.input = 'transcript';
    } else {
      const validInputs = ['transcript', ...pipelineEditorSteps.slice(0, i).map(s => s.name)];
      if (!validInputs.includes(step.input)) {
        step.input = pipelineEditorSteps[i - 1].name || 'transcript';
      }
    }
  }
}

function getInputOptions(stepIndex) {
  const options = ['transcript'];
  for (let i = 0; i < stepIndex; i++) {
    if (pipelineEditorSteps[i].name) options.push(pipelineEditorSteps[i].name);
  }
  return options;
}

function showStepEditor(index) {
  const step = pipelineEditorSteps[index];
  if (!step) return;

  const inputOptions = getInputOptions(index).map(o =>
    `<option value="${escapeHtml(o)}" ${step.input === o ? 'selected' : ''}>${escapeHtml(o)}</option>`
  ).join('');

  const promptTemplateOptions = allPromptTemplates.map(t =>
    `<option value="${escapeHtml(t.name)}" ${step.config?.prompt_template === t.name ? 'selected' : ''}>${escapeHtml(t.name)}</option>`
  ).join('');

  // Build connector-specific config fields
  let configFields = '';
  if (step.connector === 'llm') {
    let promptField;
    if (step.config?.prompt_inline) {
      promptField = `<div class="step-editor-row"><label>Prompt</label><textarea data-field="prompt_inline" rows="3">${escapeHtml(step.config.prompt_inline)}</textarea></div>`;
    } else {
      promptField = `<div class="step-editor-row"><label>Prompt</label><select data-field="prompt_template"><option value="">Select template...</option>${promptTemplateOptions}</select></div>`;
    }
    configFields = `
      ${promptField}
      <details class="step-editor-advanced">
        <summary>Advanced</summary>
        <div class="step-editor-row"><label>Provider</label><select data-field="provider">
          <option value="openai" ${step.config?.provider === 'openai' ? 'selected' : ''}>OpenAI</option>
          <option value="google" ${step.config?.provider === 'google' ? 'selected' : ''}>Google</option>
          <option value="anthropic" ${step.config?.provider === 'anthropic' ? 'selected' : ''}>Anthropic</option>
        </select></div>
        <div class="step-editor-row"><label>Model</label><input data-field="model" value="${escapeHtml(step.config?.model || '')}" placeholder="e.g. gpt-4o" /></div>
      </details>
    `;
  } else if (step.connector === 'save') {
    const savePaths = (typeof savePathIntegrations !== 'undefined') ? savePathIntegrations : [];

    if (savePaths.length === 0) {
      // No save path integrations — fall back to free-text path input
      configFields = `
        <div class="step-editor-row"><label>Path</label><input data-field="path" value="${escapeHtml(step.config?.path || '')}" placeholder="~/Documents/{date}-{pipeline-name}.md" /></div>
        <div class="step-editor-row" style="color: var(--text-secondary); font-size: 0.8rem;">Tip: Add named save paths in Settings &gt; Integrations for quick selection.</div>
      `;
    } else {
      const saveOptions = savePaths.map(sp =>
        `<option value="${escapeHtml(sp.id)}" ${step.config?.save_path_id === sp.id ? 'selected' : ''}>${escapeHtml(sp.name)} (${escapeHtml(sp.path)})</option>`
      ).join('');
      configFields = `
        <div class="step-editor-row"><label>Save Location</label><select data-field="save_path_id">
          <option value="">Select save path...</option>
          ${saveOptions}
        </select></div>
        <div class="step-editor-row"><label>Filename</label><input data-field="filename" value="${escapeHtml(step.config?.filename || '')}" placeholder="{date}-{pipeline-name}.md" /></div>
      `;
    }
  } else if (step.connector === 'webhook') {
    const whProfiles = (typeof webhookProfiles !== 'undefined') ? webhookProfiles : [];
    const webhookOptions = whProfiles.map(p =>
      `<option value="${escapeHtml(p.id)}" ${step.config?.integration_id === p.id ? 'selected' : ''}>${escapeHtml(p.name)} (${escapeHtml(p.method || 'POST')})</option>`
    ).join('');

    if (whProfiles.length === 0) {
      configFields = `
        <div class="step-editor-row" style="color: var(--text-secondary); font-size: 0.85rem;">No webhook endpoints configured. Add one in Settings &gt; Integrations.</div>
      `;
    } else {
      configFields = `
        <div class="step-editor-row"><label>Endpoint</label><select data-field="integration_id">
          <option value="">Select endpoint...</option>
          ${webhookOptions}
        </select></div>
      `;
    }
  } else if (step.connector === 'slack') {
    const slackIntegrationOptions = Object.entries(slackIntegrations).map(([id, data]) =>
      `<option value="${escapeHtml(id)}" ${step.config?.integration_id === id ? 'selected' : ''}>${escapeHtml(data.name)}</option>`
    ).join('');
    configFields = `
      <div class="step-editor-row"><label>Workspace</label><select data-field="integration_id">
        <option value="">Select workspace...</option>
        ${slackIntegrationOptions}
      </select></div>
      <div class="step-editor-row"><label>Target</label><input data-field="target" value="${escapeHtml(step.config?.target || '')}" placeholder="#channel, email@example.com, or U123456" /></div>
      <div class="step-editor-row"><label>Thread TS (optional)</label><input data-field="thread_ts" value="${escapeHtml(step.config?.thread_ts || '')}" placeholder="1234567890.123456" /></div>
    `;
  } else if (step.connector === 'mcp') {
    configFields = `
      <div class="step-editor-row"><label>Server</label><input data-field="server" value="${escapeHtml(step.config?.server || '')}" placeholder="e.g. slack-mcp" /></div>
      <div class="step-editor-row"><label>Tool</label><input data-field="tool" value="${escapeHtml(step.config?.tool || '')}" placeholder="e.g. send-message" /></div>
      <div class="step-editor-row"><label>Args</label><textarea data-field="args" rows="2" placeholder='{"channel": "#team"}'>${escapeHtml(step.config?.args ? JSON.stringify(step.config.args, null, 2) : '')}</textarea></div>
    `;
  } else if (step.connector === 'notion') {
    // Build Notion integration options from notionProfiles (loaded by integrations-settings.js)
    const profiles = (typeof notionProfiles !== 'undefined') ? notionProfiles : [];
    const notionOptions = profiles.map(p =>
      `<option value="${escapeHtml(p.id)}" ${step.config?.integration_id === p.id ? 'selected' : ''}>${escapeHtml(p.name)} (${escapeHtml(p.database_name || 'No DB')})</option>`
    ).join('');

    if (profiles.length === 0) {
      configFields = `
        <div class="step-editor-row" style="color: var(--text-secondary); font-size: 0.85rem;">No Notion integrations connected. Add one in Settings &gt; Integrations.</div>
      `;
    } else {
      configFields = `
        <div class="step-editor-row"><label>Integration</label><select data-field="integration_id">
          <option value="">Select Notion database...</option>
          ${notionOptions}
        </select></div>
        <div class="step-editor-row">
          <button class="mini-action-btn resync-notion-schema-btn" style="font-size: 0.8rem;">Re-sync Schema</button>
          <span class="resync-status" style="font-size: 0.8rem; color: var(--text-secondary); margin-left: 8px;"></span>
        </div>
      `;
    }
  } else if (step.connector === 'linear') {
    // Build Linear integration options from linearProfiles (loaded by integrations-settings.js)
    const linProfiles = (typeof linearProfiles !== 'undefined') ? linearProfiles : [];
    const linearOptions = linProfiles.map(p =>
      `<option value="${escapeHtml(p.id)}" ${step.config?.integration_id === p.id ? 'selected' : ''}>${escapeHtml(p.name)} (${escapeHtml(p.team_name || 'No team')})</option>`
    ).join('');

    if (linProfiles.length === 0) {
      configFields = `
        <div class="step-editor-row" style="color: var(--text-secondary); font-size: 0.85rem;">No Linear integrations connected. Add one in Settings &gt; Integrations.</div>
      `;
    } else {
      configFields = `
        <div class="step-editor-row"><label>Integration</label><select data-field="integration_id">
          <option value="">Select Linear team...</option>
          ${linearOptions}
        </select></div>
        <div class="step-editor-row">
          <button class="mini-action-btn resync-linear-schema-btn" style="font-size: 0.8rem;">Re-sync Schema</button>
          <span class="resync-status" style="font-size: 0.8rem; color: var(--text-secondary); margin-left: 8px;"></span>
        </div>
      `;
    }
  }

  // Replace step item (or existing editor) with new editor
  const stepChildren = pipelineStepsListEl.querySelectorAll('.pipeline-step-item, .step-editor');
  const stepEl = stepChildren[index];
  if (!stepEl) return;

  const editorEl = document.createElement('div');
  editorEl.className = 'step-editor';
  editorEl.innerHTML = `
    <div class="step-editor-row"><label>Name</label><input data-field="name" value="${escapeHtml(step.name)}" placeholder="step name" /></div>
    <div class="step-editor-row"><label>Connector</label><select data-field="connector">
      <option value="llm" ${step.connector === 'llm' ? 'selected' : ''}>LLM</option>
      <option value="save" ${step.connector === 'save' ? 'selected' : ''}>Save</option>
      <option value="webhook" ${step.connector === 'webhook' ? 'selected' : ''}>Webhook</option>
      <option value="slack" ${step.connector === 'slack' ? 'selected' : ''}>Slack</option>
      <option value="mcp" ${step.connector === 'mcp' ? 'selected' : ''}>MCP</option>
      <option value="notion" ${step.connector === 'notion' ? 'selected' : ''}>Notion</option>
      <option value="linear" ${step.connector === 'linear' ? 'selected' : ''}>Linear</option>
    </select></div>
    <div class="step-editor-row"><label>Input</label><select data-field="input">${inputOptions}</select></div>
    <div class="step-editor-row"><label>Description</label><input data-field="description" value="${escapeHtml(step.description || '')}" placeholder="What this step does..." /></div>
    <div id="step-config-fields">${configFields}</div>
    <div class="step-editor-actions">
      <button class="mini-action-btn compact-add-btn step-editor-done">Done</button>
    </div>
  `;

  stepEl.replaceWith(editorEl);

  // Re-sync Schema button handler (Notion only, wired after editor is in DOM)
  if (step.connector === 'notion') {
    const resyncBtn = editorEl.querySelector('.resync-notion-schema-btn');
    if (resyncBtn) {
      resyncBtn.addEventListener('click', async () => {
        // Get currently selected integration_id from the dropdown
        const integrationSelect = editorEl.querySelector('[data-field="integration_id"]');
        const integrationId = integrationSelect ? integrationSelect.value : '';
        if (!integrationId) {
          const statusSpan = editorEl.querySelector('.resync-status');
          if (statusSpan) statusSpan.textContent = 'Select an integration first';
          return;
        }

        // Find the profile to get database_id and database_name
        const profiles = (typeof notionProfiles !== 'undefined') ? notionProfiles : [];
        const profile = profiles.find(p => p.id === integrationId);
        if (!profile || !profile.database_id) {
          const statusSpan = editorEl.querySelector('.resync-status');
          if (statusSpan) statusSpan.textContent = 'No database synced for this integration';
          return;
        }

        resyncBtn.disabled = true;
        resyncBtn.textContent = 'Syncing...';
        const statusSpan = editorEl.querySelector('.resync-status');
        if (statusSpan) statusSpan.textContent = '';

        try {
          const updatedProfile = await window.__TAURI__.core.invoke('sync_notion_schema', {
            integrationId: integrationId,
            databaseId: profile.database_id,
            databaseName: profile.database_name,
          });

          // Update the notionProfiles global so other UI stays current
          const idx = notionProfiles.findIndex(p => p.id === integrationId);
          if (idx >= 0) {
            notionProfiles[idx] = updatedProfile;
          }

          resyncBtn.textContent = 'Re-sync Schema';
          resyncBtn.disabled = false;
          if (statusSpan) {
            statusSpan.style.color = 'var(--text-secondary)';
            statusSpan.textContent = 'Schema synced successfully';
          }
        } catch (err) {
          resyncBtn.textContent = 'Re-sync Schema';
          resyncBtn.disabled = false;
          if (statusSpan) {
            statusSpan.style.color = '#e6453d';
            statusSpan.textContent = 'Sync failed: ' + String(err);
          }
        }
      });
    }
  }

  // Re-sync Schema button handler (Linear only, wired after editor is in DOM)
  if (step.connector === 'linear') {
    const resyncBtn = editorEl.querySelector('.resync-linear-schema-btn');
    if (resyncBtn) {
      resyncBtn.addEventListener('click', async () => {
        const integrationSelect = editorEl.querySelector('[data-field="integration_id"]');
        const integrationId = integrationSelect ? integrationSelect.value : '';
        if (!integrationId) {
          const statusSpan = editorEl.querySelector('.resync-status');
          if (statusSpan) statusSpan.textContent = 'Select an integration first';
          return;
        }

        const linProfiles = (typeof linearProfiles !== 'undefined') ? linearProfiles : [];
        const profile = linProfiles.find(p => p.id === integrationId);
        if (!profile || !profile.team_id) {
          const statusSpan = editorEl.querySelector('.resync-status');
          if (statusSpan) statusSpan.textContent = 'No team synced for this integration';
          return;
        }

        resyncBtn.disabled = true;
        resyncBtn.textContent = 'Syncing...';
        const statusSpan = editorEl.querySelector('.resync-status');
        if (statusSpan) statusSpan.textContent = '';

        try {
          const updatedProfile = await window.__TAURI__.core.invoke('sync_linear_schema', {
            integrationId: integrationId,
            teamId: profile.team_id,
            teamName: profile.team_name,
          });

          // Update the linearProfiles global
          const idx = linearProfiles.findIndex(p => p.id === integrationId);
          if (idx >= 0) {
            linearProfiles[idx] = updatedProfile;
          }

          resyncBtn.textContent = 'Re-sync Schema';
          resyncBtn.disabled = false;
          if (statusSpan) {
            statusSpan.style.color = 'var(--text-secondary)';
            statusSpan.textContent = 'Schema synced successfully';
          }
        } catch (err) {
          resyncBtn.textContent = 'Re-sync Schema';
          resyncBtn.disabled = false;
          if (statusSpan) {
            statusSpan.style.color = '#e6453d';
            statusSpan.textContent = 'Sync failed: ' + String(err);
          }
        }
      });
    }
  }

  // Connector change → re-render config fields
  const connectorSelect = editorEl.querySelector('[data-field="connector"]');
  connectorSelect.addEventListener('change', () => {
    step.connector = connectorSelect.value;
    step.config = {};
    showStepEditor(index);
  });

  // Done button
  editorEl.querySelector('.step-editor-done').addEventListener('click', () => {
    // Read values back
    step.name = editorEl.querySelector('[data-field="name"]').value.trim();
    step.connector = editorEl.querySelector('[data-field="connector"]').value;
    step.input = editorEl.querySelector('[data-field="input"]').value;
    step.description = editorEl.querySelector('[data-field="description"]').value.trim() || null;

    // Read connector-specific config
    const configFieldsEl = editorEl.querySelector('#step-config-fields');
    step.config = {};
    configFieldsEl.querySelectorAll('[data-field]').forEach(field => {
      const key = field.dataset.field;
      let val = field.value.trim();
      if (key === 'args') {
        try { val = JSON.parse(val); } catch { val = {}; }
      }
      if (val !== '') step.config[key] = val;
    });

    renderPipelineSteps();
    renderPipelinePreview();
  });
}

function renderPipelinePreview() {
  if (!pipelinePreviewEl) return;
  if (pipelineEditorSteps.length === 0) {
    pipelinePreviewEl.innerHTML = '<span class="pipeline-preview-empty">Add steps to see preview</span>';
    return;
  }

  const deliveryConnectors = ['save', 'notion', 'linear', 'slack', 'webhook', 'mcp'];
  let html = '<span class="preview-node source">transcript</span>';
  for (const step of pipelineEditorSteps) {
    const isDelivery = deliveryConnectors.includes(step.connector);
    const nodeClass = isDelivery ? 'preview-node delivery' : 'preview-node step';
    html += '<span class="preview-arrow">&rarr;</span>';
    html += `<span class="${nodeClass}">${escapeHtml(step.name || '?')} <small style="opacity:0.6">(${escapeHtml(step.connector)})</small></span>`;
  }
  pipelinePreviewEl.innerHTML = html;
}

if (addPipelineDefBtn) addPipelineDefBtn.addEventListener('click', () => openPipelineEditor(null));
if (closePipelineEditorBtn) closePipelineEditorBtn.addEventListener('click', closePipelineEditor);

if (addPipelineStepBtn) {
  addPipelineStepBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    togglePicker();
  });
}

if (savePipelineDefBtn) {
  savePipelineDefBtn.addEventListener('click', async () => {
    const name = pipelineEditorName.value.trim();
    const desc = pipelineEditorDesc.value.trim();
    if (!name) { alert('Pipeline name is required'); return; }

    // Validate step names
    for (let i = 0; i < pipelineEditorSteps.length; i++) {
      if (!pipelineEditorSteps[i].name.trim()) {
        alert(`Step ${i + 1} needs a name`);
        return;
      }
    }

    try {
      const pipeline = { name, description: desc, steps: pipelineEditorSteps };

      // If renaming, delete old first
      if (editingPipelineDef && editingPipelineDef !== name) {
        await invoke('delete_pipeline', { name: editingPipelineDef });
      }
      await invoke('save_pipeline', { pipeline });
      closePipelineEditor();
      await loadPipelineDefs();
    } catch (err) {
      console.error('Failed to save pipeline:', err);
      alert('Failed to save: ' + err);
    }
  });
}

if (deletePipelineDefBtn) {
  deletePipelineDefBtn.addEventListener('click', async () => {
    if (!editingPipelineDef) return;
    if (!confirm(`Delete pipeline "${editingPipelineDef}"?`)) return;
    try {
      await invoke('delete_pipeline', { name: editingPipelineDef });
      closePipelineEditor();
      await loadPipelineDefs();
    } catch (err) {
      console.error('Failed to delete pipeline:', err);
      alert('Failed to delete: ' + err);
    }
  });
}
