// ===== PIPELINE DEFINITION MANAGEMENT =====
// Globals from main.js (loaded before this script): escapeHtml, invoke, allPromptTemplates, slackIntegrations, updateSidebarCounts
// Globals from integrations-settings.js (loaded before this script): notionProfiles, linearProfiles, savePathIntegrations, webhookProfiles (all via typeof guard)

var allPipelineDefs = []; // var so main.js updateSidebarCounts() can access allPipelineDefs.length
let editingPipelineDef = null; // null = new, string = editing name
let pipelineEditorSteps = []; // Working copy of steps
let editingStepIndex = null;  // index of step currently open in panel
let sortableInstance = null;  // Sortable.js instance for drag-and-drop reordering
let lastAutoName = '';        // Track last auto-generated pipeline name
const slackTargetCache = {};  // Cache channels+members per integration_id

const SLACK_SVG = `<svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor"><path d="M5.042 15.165a2.528 2.528 0 0 1-2.52 2.523A2.528 2.528 0 0 1 0 15.165a2.527 2.527 0 0 1 2.522-2.52h2.52v2.52zm1.271 0a2.527 2.527 0 0 1 2.521-2.52 2.527 2.527 0 0 1 2.521 2.52v6.313A2.528 2.528 0 0 1 8.834 24a2.528 2.528 0 0 1-2.521-2.522v-6.313zm2.521-10.123a2.528 2.528 0 0 1-2.521-2.52A2.528 2.528 0 0 1 8.834 0a2.528 2.528 0 0 1 2.521 2.522v2.52H8.834zm0 1.271a2.528 2.528 0 0 1 2.521 2.521 2.528 2.528 0 0 1-2.521 2.521H2.522A2.528 2.528 0 0 1 0 8.834a2.528 2.528 0 0 1 2.522-2.521h6.312zm10.123 2.521a2.528 2.528 0 0 1 2.522-2.521A2.528 2.528 0 0 1 24 8.834a2.528 2.528 0 0 1-2.522 2.521h-2.522V8.834zm-1.268 0a2.528 2.528 0 0 1-2.523 2.521 2.527 2.527 0 0 1-2.52-2.521V2.522A2.527 2.527 0 0 1 15.165 0a2.528 2.528 0 0 1 2.523 2.522v6.312zm-2.523 10.123a2.528 2.528 0 0 1 2.523 2.522A2.528 2.528 0 0 1 15.165 24a2.527 2.527 0 0 1-2.52-2.522v-2.522h2.52zm0-1.268a2.527 2.527 0 0 1-2.52-2.523 2.526 2.526 0 0 1 2.52-2.52h6.313A2.527 2.527 0 0 1 24 15.165a2.528 2.528 0 0 1-2.522 2.523h-6.313z"/></svg>`;

const PROVIDER_META = {
  openai:    { img: 'assets/openai.svg',    filter: 'invert(1)',                                                          bgColor: 'rgba(16,163,127,0.15)' },
  google:    { img: 'assets/gemini.svg',    filter: 'invert(48%) sepia(90%) saturate(400%) hue-rotate(190deg)',           bgColor: 'rgba(66,133,244,0.15)' },
  anthropic: { img: 'assets/anthropic.svg', filter: 'invert(55%) sepia(80%) saturate(500%) hue-rotate(10deg)',            bgColor: 'rgba(217,119,6,0.15)'  },
};

function trimModelName(model, provider) {
  if (!model) return '';
  // Strip provider prefix: "claude-" → "", "gpt-" → "", "gemini-" → ""
  const prefixes = { anthropic: 'claude-', openai: 'gpt-', google: 'gemini-' };
  const prefix = prefixes[provider] || '';
  return prefix && model.startsWith(prefix) ? model.slice(prefix.length) : model;
}

const CONNECTOR_META = {
  llm:     { abbr: 'AI', textColor: 'var(--accent)',   bgColor: 'var(--accent-soft)' },
  save:    { abbr: '↓',  textColor: '#10b981',         bgColor: 'rgba(16,185,129,0.15)' },
  slack:   { svg: SLACK_SVG, textColor: '#fff',         bgColor: '#4A154B' },
  notion:  { abbr: 'N',  textColor: '#fff',            bgColor: '#2f2f2f' },
  webhook: { abbr: '⚡', textColor: '#60a5fa',          bgColor: 'rgba(59,130,246,0.2)' },
  linear:  { abbr: 'L',  textColor: '#fff',            bgColor: '#5E6AD2' },
  mcp:     { abbr: 'M',  textColor: '#f59e0b',         bgColor: 'rgba(245,158,11,0.15)' },
};

const pipelineDefsListEl = document.getElementById('pipeline-defs-list');
const addPipelineDefBtn = document.getElementById('add-pipeline-def-btn');
const pipelineEditor = document.getElementById('pipeline-editor');
const pipelineEditorTitle = document.getElementById('pipeline-editor-title');
const pipelineEditorName = document.getElementById('pipeline-editor-name');
const pipelineEditorDesc = document.getElementById('pipeline-editor-desc');
const pipelineStepsListEl = document.getElementById('pipeline-steps-list');
const addPipelineStepBtn = document.getElementById('add-pipeline-step-btn');
const stepEditorPanelEl = document.getElementById('step-editor-panel');
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

  // Insert picker below the tiles flow
  pipelineStepsListEl.parentNode.insertBefore(picker, pipelineStepsListEl.nextSibling);
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
  const addTile = document.getElementById('add-step-tile');
  if (picker && !picker.contains(e.target) && e.target !== addTile && !addTile?.contains(e.target)) {
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
  // Auto-wire: delivery steps chain from the last step's output, not raw transcript
  if (step.connector !== 'llm' && pipelineEditorSteps.length > 0) {
    const lastStep = pipelineEditorSteps[pipelineEditorSteps.length - 1];
    if (lastStep.name) step.input = lastStep.name;
  }
  pipelineEditorSteps.push(step);
  fixStepInputs();
  renderPipelineSteps();
  closePicker();
  maybeAutoName();
  // Auto-open editor for delivery connectors that need configuration
  const needsConfig = ['slack', 'notion', 'linear', 'webhook', 'save'];
  if (needsConfig.includes(step.connector)) {
    showStepEditor(pipelineEditorSteps.length - 1);
  }
}

function suggestPipelineName() {
  const processing = [];
  const delivery = [];
  for (const step of pipelineEditorSteps) {
    if (step.connector === 'llm') {
      // Title-case the step name: "meeting-notes" → "Meeting Notes"
      const titleCased = (step.name || 'Untitled')
        .replace(/[-_]/g, ' ')
        .replace(/\b\w/g, c => c.toUpperCase());
      processing.push(titleCased);
    } else {
      // Delivery: connector name + integration profile name if available
      const connectorName = (step.connector || 'Unknown').charAt(0).toUpperCase() + (step.connector || '').slice(1);
      delivery.push(connectorName);
    }
  }
  if (processing.length === 0 && delivery.length === 0) return '';
  const parts = [];
  if (processing.length) parts.push(processing.join(', '));
  if (delivery.length) parts.push(delivery.join(', '));
  return parts.join(' \u2192 '); // → arrow
}

function maybeAutoName() {
  const currentVal = pipelineEditorName.value.trim();
  if (currentVal === '' || currentVal === lastAutoName) {
    const suggested = suggestPipelineName();
    pipelineEditorName.value = suggested;
    lastAutoName = suggested;
  }
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
  lastAutoName = '';
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
  editingStepIndex = null;
  closeStepEditorPanel();
  pipelineEditor.style.display = 'block';
  renderPipelineSteps();
  pipelineEditorName.focus();
  const saveSettingsBtn = document.getElementById('save-settings-btn');
  if (saveSettingsBtn) saveSettingsBtn.style.display = 'none';
}

function closePipelineEditor() {
  if (pipelineEditor) pipelineEditor.style.display = 'none';
  editingPipelineDef = null;
  editingStepIndex = null;
  pipelineEditorSteps = [];
  closeStepEditorPanel();
  const saveSettingsBtn = document.getElementById('save-settings-btn');
  if (saveSettingsBtn) saveSettingsBtn.style.display = '';
}

function closeStepEditorPanel() {
  if (stepEditorPanelEl) {
    stepEditorPanelEl.innerHTML = '';
    stepEditorPanelEl.style.display = 'none';
  }
}

function renderPipelineSteps() {
  if (!pipelineStepsListEl) return;

  // Source tile (always first)
  let html = `
    <div class="step-tile step-tile--source">
      <div class="step-tile-icon-wrap" style="background:var(--accent-soft);color:var(--accent);">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
          <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
          <line x1="12" y1="19" x2="12" y2="23"/>
          <line x1="8" y1="23" x2="16" y2="23"/>
        </svg>
      </div>
      <div class="step-tile-name">Transcript</div>
    </div>
  `;

  // Step tiles
  for (let i = 0; i < pipelineEditorSteps.length; i++) {
    const step = pipelineEditorSteps[i];
    let meta = CONNECTOR_META[step.connector] || {
      abbr: step.connector.substring(0, 2).toUpperCase(),
      textColor: 'var(--text-primary)',
      bgColor: 'var(--bg-input)'
    };
    // LLM: use provider-specific colors + show model
    let iconContent = meta.svg || meta.abbr || '';
    let tileSubtitle = '';
    if (step.connector === 'llm') {
      const provider = step.config?.provider || 'openai';
      const provMeta = PROVIDER_META[provider] || PROVIDER_META.openai;
      meta = { ...meta, bgColor: provMeta.bgColor };
      iconContent = `<img src="${provMeta.img}" style="width:20px;height:20px;filter:${provMeta.filter};" alt="${provider}" />`;
      const model = step.config?.model || '';
      if (model) {
        const short = trimModelName(model, provider);
        tileSubtitle = `<div class="step-tile-model">${escapeHtml(short)}</div>`;
      }
    }
    const safeName = escapeHtml(step.name || 'Unnamed');
    const isEditing = editingStepIndex === i;

    html += `
      <div class="step-tile step-tile--step${isEditing ? ' is-editing' : ''}" data-index="${i}">
        <span class="step-tile-num">${i + 1}</span>
        <button class="step-tile-remove" data-index="${i}" title="Remove step">×</button>
        <div class="step-tile-icon-wrap" style="background:${meta.bgColor};color:${meta.textColor};">
          ${iconContent}
        </div>
        <div class="step-tile-name" title="${safeName}">${safeName}</div>
        <div class="step-tile-connector">${escapeHtml(step.connector)}</div>
        ${tileSubtitle}
        <div class="step-tile-input-from">from: ${escapeHtml(step.input || 'transcript')}</div>
      </div>
    `;
  }

  // Add step tile
  html += `
    <div class="step-tile step-tile--add" id="add-step-tile" title="Add step">
      <div class="step-tile-plus">+</div>
      <div class="step-tile-name">Add Step</div>
    </div>
  `;

  pipelineStepsListEl.innerHTML = html;

  // Wire: click step tile to edit (ignore remove/move buttons)
  pipelineStepsListEl.querySelectorAll('.step-tile[data-index]').forEach(tile => {
    tile.addEventListener('click', (e) => {
      if (e.target.closest('.step-tile-remove')) return;
      showStepEditor(parseInt(tile.dataset.index));
    });
  });

  // Wire: remove buttons
  pipelineStepsListEl.querySelectorAll('.step-tile-remove').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const idx = parseInt(btn.dataset.index);
      const stepName = pipelineEditorSteps[idx]?.name || `Step ${idx + 1}`;
      if (!confirm(`Remove step "${stepName}"?`)) return;
      pipelineEditorSteps.splice(idx, 1);
      if (editingStepIndex === idx) {
        editingStepIndex = null;
        closeStepEditorPanel();
      } else if (editingStepIndex !== null && editingStepIndex > idx) {
        editingStepIndex--;
      }
      fixStepInputs();
      renderPipelineSteps();
      maybeAutoName();
    });
  });

  // Wire: add step tile
  const addTile = document.getElementById('add-step-tile');
  if (addTile) {
    addTile.addEventListener('click', (e) => {
      e.stopPropagation();
      togglePicker();
    });
  }

  // Initialize Sortable.js for drag-and-drop reordering
  if (sortableInstance) {
    sortableInstance.destroy();
    sortableInstance = null;
  }
  if (typeof Sortable !== 'undefined') {
    sortableInstance = Sortable.create(pipelineStepsListEl, {
      draggable: '.step-tile--step',
      filter: '.step-tile--source, .step-tile--add',
      ghostClass: 'step-tile-ghost',
      chosenClass: 'step-tile-chosen',
      dragClass: 'step-tile-dragging',
      animation: 150,
      onEnd(evt) {
        // DOM: [source(0), step1(1), step2(2), ..., stepN(N), add(N+1)]
        const oldStepIdx = evt.oldIndex - 1;
        const newStepIdx = evt.newIndex - 1;
        if (oldStepIdx === newStepIdx || oldStepIdx < 0 || newStepIdx < 0) return;
        const clampedNew = Math.min(Math.max(newStepIdx, 0), pipelineEditorSteps.length - 1);
        const [moved] = pipelineEditorSteps.splice(oldStepIdx, 1);
        pipelineEditorSteps.splice(clampedNew, 0, moved);
        if (editingStepIndex === oldStepIdx) editingStepIndex = clampedNew;
        else if (editingStepIndex !== null) {
          if (oldStepIdx < editingStepIndex && clampedNew >= editingStepIndex) editingStepIndex--;
          else if (oldStepIdx > editingStepIndex && clampedNew <= editingStepIndex) editingStepIndex++;
        }
        fixStepInputs();
        renderPipelineSteps();
        if (editingStepIndex !== null) showStepEditor(editingStepIndex);
      }
    });
  }
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
        <div class="step-editor-row"><label>Model</label><input data-field="model" value="${escapeHtml(step.config?.model || '')}" placeholder="e.g. gpt-5.2" /></div>
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
    const slackEntries = Object.entries(slackIntegrations);
    const slackIntegrationOptions = slackEntries.map(([id, data]) =>
      `<option value="${escapeHtml(id)}" ${step.config?.integration_id === id ? 'selected' : ''}>${escapeHtml(data.name)}</option>`
    ).join('');
    const wsRowStyle = slackEntries.length <= 1 ? 'display:none;' : '';
    configFields = `
      <div class="step-editor-row" style="${wsRowStyle}"><label>Workspace</label><select data-field="integration_id" class="slack-workspace-select">
        <option value="">Select workspace...</option>
        ${slackIntegrationOptions}
      </select></div>
      <div class="step-editor-row"><label>Target</label>
        <select data-field="target" class="slack-target-select">
          <option value="">Select channel or person...</option>
        </select>
        <div class="slack-target-loading" style="display:none;font-size:0.8rem;color:var(--text-secondary);margin-top:4px;">Loading channels &amp; members...</div>
      </div>
      <div class="step-editor-row slack-custom-target-row" style="display:none;">
        <label>Custom Target</label>
        <input data-field="target_custom" value="" placeholder="#channel, email@example.com, or U123456" />
      </div>
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

  // Update editing index and re-render tiles to show active state
  editingStepIndex = index;
  renderPipelineSteps();

  // Position popup below the active tile
  const activeTile = pipelineStepsListEl.querySelector('.step-tile.is-editing');
  const tileOffset = activeTile ? (activeTile.offsetLeft - pipelineStepsListEl.scrollLeft) : 0;

  // Build editor into the panel below tiles
  const editorEl = document.createElement('div');
  editorEl.className = 'step-editor';
  editorEl.style.marginLeft = Math.max(0, tileOffset) + 'px';
  editorEl.innerHTML = `
    <div class="step-editor-header">
      <span class="step-editor-title">Step ${index + 1} — ${escapeHtml(step.name || 'Unnamed')}</span>
      <button class="step-editor-close" title="Close editor">×</button>
    </div>
    <div class="step-editor-row"><label>Input</label><select data-field="input">${inputOptions}</select></div>
    <div id="step-config-fields">${configFields}</div>
    <details class="step-editor-advanced">
      <summary>Advanced</summary>
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
      <div class="step-editor-row"><label>Description</label><input data-field="description" value="${escapeHtml(step.description || '')}" placeholder="What this step does..." /></div>
    </details>
    <div class="step-editor-actions">
      <button class="step-editor-done">Done</button>
    </div>
  `;

  stepEditorPanelEl.innerHTML = '';
  stepEditorPanelEl.appendChild(editorEl);
  stepEditorPanelEl.style.display = 'block';
  setTimeout(() => editorEl.scrollIntoView({ behavior: 'smooth', block: 'nearest' }), 50);

  // Close button
  editorEl.querySelector('.step-editor-close').addEventListener('click', () => {
    editingStepIndex = null;
    closeStepEditorPanel();
    renderPipelineSteps();
  });

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

  // Slack workspace change → fetch channels + members for target dropdown
  if (step.connector === 'slack') {
    const wsSelect = editorEl.querySelector('.slack-workspace-select');
    const targetSelect = editorEl.querySelector('.slack-target-select');
    const loadingEl = editorEl.querySelector('.slack-target-loading');
    const customRow = editorEl.querySelector('.slack-custom-target-row');

    async function populateSlackTargets(integrationId) {
      if (!integrationId) {
        targetSelect.innerHTML = '<option value="">Select channel or person...</option>';
        customRow.style.display = 'none';
        return;
      }
      // Check cache
      if (slackTargetCache[integrationId]) {
        renderSlackTargetOptions(slackTargetCache[integrationId], targetSelect, step.config?.target, customRow);
        return;
      }
      loadingEl.style.display = 'block';
      targetSelect.disabled = true;
      try {
        const [channels, members] = await Promise.all([
          invoke('list_slack_channels', { integrationId }),
          invoke('list_slack_members', { integrationId })
        ]);
        slackTargetCache[integrationId] = { channels, members };
        renderSlackTargetOptions({ channels, members }, targetSelect, step.config?.target, customRow);
      } catch (err) {
        targetSelect.innerHTML = `<option value="">Failed to load: ${escapeHtml(String(err))}</option>`;
      } finally {
        loadingEl.style.display = 'none';
        targetSelect.disabled = false;
      }
    }

    function renderSlackTargetOptions(data, selectEl, currentValue, customRowEl) {
      let opts = '<option value="">Select channel or person...</option>';
      if (data.channels.length > 0) {
        opts += '<optgroup label="Channels">';
        for (const ch of data.channels) {
          const prefix = ch.is_private ? '🔒 ' : '#';
          const sel = currentValue === ch.id ? ' selected' : '';
          opts += `<option value="${escapeHtml(ch.id)}"${sel}>${prefix}${escapeHtml(ch.name)}</option>`;
        }
        opts += '</optgroup>';
      }
      if (data.members.length > 0) {
        opts += '<optgroup label="People">';
        for (const m of data.members) {
          const sel = currentValue === m.id ? ' selected' : '';
          opts += `<option value="${escapeHtml(m.id)}"${sel}>${escapeHtml(m.display_name)}</option>`;
        }
        opts += '</optgroup>';
      }
      opts += '<option value="__custom__">Custom target...</option>';
      selectEl.innerHTML = opts;
      // If current value doesn't match any option, select custom
      if (currentValue && currentValue !== '__custom__' && !selectEl.querySelector(`option[value="${CSS.escape(currentValue)}"]`)) {
        selectEl.value = '__custom__';
        customRowEl.style.display = 'block';
        const customInput = customRowEl.querySelector('[data-field="target_custom"]');
        if (customInput) customInput.value = currentValue;
      }
    }

    wsSelect.addEventListener('change', () => {
      populateSlackTargets(wsSelect.value);
    });

    targetSelect.addEventListener('change', () => {
      if (targetSelect.value === '__custom__') {
        customRow.style.display = 'block';
      } else {
        customRow.style.display = 'none';
      }
    });

    // Auto-select workspace if only one exists or already configured
    const wsIds = Object.keys(slackIntegrations);
    if (!step.config?.integration_id && wsIds.length === 1) {
      wsSelect.value = wsIds[0];
      step.config = step.config || {};
      step.config.integration_id = wsIds[0];
    }
    if (wsSelect.value) {
      populateSlackTargets(wsSelect.value);
    }
  }

  // Connector change → re-render config fields
  const connectorSelect = editorEl.querySelector('[data-field="connector"]');
  connectorSelect.addEventListener('change', () => {
    step.connector = connectorSelect.value;
    step.config = {};
    showStepEditor(index);
  });

  // Done button — save and collapse panel
  editorEl.querySelector('.step-editor-done').addEventListener('click', () => {
    step.name = editorEl.querySelector('[data-field="name"]').value.trim();
    step.connector = editorEl.querySelector('[data-field="connector"]').value;
    step.input = editorEl.querySelector('[data-field="input"]').value;
    step.description = editorEl.querySelector('[data-field="description"]').value.trim() || null;

    const configFieldsEl = editorEl.querySelector('#step-config-fields');
    const prevTarget = step.config?.target; // preserve for Slack async-load case
    step.config = {};
    configFieldsEl.querySelectorAll('[data-field]').forEach(field => {
      const key = field.dataset.field;
      let val = field.value.trim();
      if (key === 'args') {
        try { val = JSON.parse(val); } catch { val = {}; }
      }
      // Slack target: if dropdown says "__custom__", use custom input instead
      if (key === 'target' && val === '__custom__') return;
      if (key === 'target_custom') {
        // Only use custom value if target dropdown is "__custom__"
        const targetSel = configFieldsEl.querySelector('[data-field="target"]');
        if (targetSel && targetSel.value === '__custom__' && val !== '') {
          step.config.target = val;
        }
        return;
      }
      if (val !== '') step.config[key] = val;
    });
    // If Slack target dropdown was empty (async not loaded yet), restore previous value
    if (step.connector === 'slack' && !step.config.target && prevTarget) {
      step.config.target = prevTarget;
    }

    editingStepIndex = null;
    closeStepEditorPanel();
    renderPipelineSteps();
  });
}

if (addPipelineDefBtn) addPipelineDefBtn.addEventListener('click', () => openPipelineEditor(null));
if (closePipelineEditorBtn) closePipelineEditorBtn.addEventListener('click', closePipelineEditor);

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
