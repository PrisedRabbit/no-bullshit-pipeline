// ===== PIPELINE DEFINITION MANAGEMENT =====
// Globals from main.js (loaded before this script): escapeHtml, invoke, allPromptTemplates, slackIntegrations
// Globals from integrations-settings.js (loaded before this script): notionProfiles, linearProfiles, savePathIntegrations, webhookProfiles (all via typeof guard)

var allPipelineDefs = [];
let editingPipelineDef = null; // null = new, string = editing name
let pipelineEditorSteps = []; // Working copy of steps
let editingStepIndex = null;  // index of step currently open in panel
let sortableInstance = null;  // Sortable.js instance for drag-and-drop reordering
let lastAutoName = '';        // Track last auto-generated pipeline name
const slackTargetCache = {};  // Cache channels+members per integration_id

const SLACK_SVG = `<svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor"><path d="M5.042 15.165a2.528 2.528 0 0 1-2.52 2.523A2.528 2.528 0 0 1 0 15.165a2.527 2.527 0 0 1 2.522-2.52h2.52v2.52zm1.271 0a2.527 2.527 0 0 1 2.521-2.52 2.527 2.527 0 0 1 2.521 2.52v6.313A2.528 2.528 0 0 1 8.834 24a2.528 2.528 0 0 1-2.521-2.522v-6.313zm2.521-10.123a2.528 2.528 0 0 1-2.521-2.52A2.528 2.528 0 0 1 8.834 0a2.528 2.528 0 0 1 2.521 2.522v2.52H8.834zm0 1.271a2.528 2.528 0 0 1 2.521 2.521 2.528 2.528 0 0 1-2.521 2.521H2.522A2.528 2.528 0 0 1 0 8.834a2.528 2.528 0 0 1 2.522-2.521h6.312zm10.123 2.521a2.528 2.528 0 0 1 2.522-2.521A2.528 2.528 0 0 1 24 8.834a2.528 2.528 0 0 1-2.522 2.521h-2.522V8.834zm-1.268 0a2.528 2.528 0 0 1-2.523 2.521 2.527 2.527 0 0 1-2.52-2.521V2.522A2.527 2.527 0 0 1 15.165 0a2.528 2.528 0 0 1 2.523 2.522v6.312zm-2.523 10.123a2.528 2.528 0 0 1 2.523 2.522A2.528 2.528 0 0 1 15.165 24a2.527 2.527 0 0 1-2.52-2.522v-2.522h2.52zm0-1.268a2.527 2.527 0 0 1-2.52-2.523 2.526 2.526 0 0 1 2.52-2.52h6.313A2.527 2.527 0 0 1 24 15.165a2.528 2.528 0 0 1-2.522 2.523h-6.313z"/></svg>`;
const NOTION_SVG = `<svg viewBox="0 0 100 100" width="18" height="18" fill="currentColor"><path d="M6.017 4.313l55.333-4.087c6.797-.583 8.543-.19 12.817 2.917l17.663 12.443c2.913 2.14 3.883 2.723 3.883 5.053v68.243c0 4.277-1.553 6.807-6.99 7.193L24.467 99.967c-4.08.193-6.023-.39-8.16-3.113L3.3 79.94c-2.333-3.113-3.3-5.443-3.3-8.167V11.113c0-3.497 1.553-6.413 6.017-6.8z" fill="#fff"/><path d="M61.35.227l-55.333 4.087C1.553 4.7 0 7.617 0 11.113v60.66c0 2.723.967 5.053 3.3 8.167l13.007 16.913c2.137 2.723 4.08 3.307 8.16 3.113l64.257-3.89c5.433-.387 6.99-2.917 6.99-7.193V20.64c0-2.21-.81-2.76-3.088-4.587L75.983 3.523C71.71.607 69.96.22 63.163.803L61.35.227z" fill="#000"/><path d="M26.395 18.768c-5.433.39-6.675.477-9.768-1.753L7.997 10.527c-1.163-.913-1.55-1.94-1.55-3.113.39-2.53 1.94-4.47 7.377-4.86l53.39-3.89c4.47-.39 6.603 1.553 8.157 2.723l10.133 7.577c.39.193 1.553 1.553 0 1.553l-55.14 3.11v5.14z" fill="#fff"/><path d="M19.018 88.4V30.173c0-2.527.78-3.697 3.113-3.89l57.277-3.307c2.14-.193 3.113 1.167 3.113 3.693V85.09c0 2.527-.39 4.667-3.887 4.86l-54.943 3.113c-3.5.193-4.673-1.003-4.673-4.663zm54.167-55.13c.39 1.75 0 3.5-1.75 3.697l-2.527.39v40.257c-2.14 1.163-4.277 1.75-5.833 1.75-2.723 0-3.5-.583-5.443-3.113L38.468 45.948V74.7l5.247 1.163s0 3.5-4.86 3.5l-13.393.78c-.39-.78 0-2.723 1.36-3.113l3.497-.97V38.33l-4.86-.39c-.39-1.75.583-4.277 3.307-4.473l14.363-.97 20.603 31.46V35.077l-4.47-.39c-.39-2.14 1.163-3.697 3.113-3.89l14.003-.527z" fill="#fff"/></svg>`;
const LINEAR_SVG = `<svg viewBox="0 0 100 100" width="18" height="18"><path d="M2.76 62.7a50.1 50.1 0 0 1-1.52-4.44L62.7 2.76a50.1 50.1 0 0 0-4.44-1.52L2.76 62.7zm7.66 12.48a50 50 0 0 1-3.54-4.3L75.18 4.58a50 50 0 0 0-4.3-3.54L10.42 75.18zm11.44 8.96a50 50 0 0 1-4.82-4.1L83.14 13.94a50 50 0 0 0-4.1-4.82L21.86 84.14zM0 50a49.9 49.9 0 0 0 .26 5L55 .26A50 50 0 1 0 0 50zm35.42 36.64a50 50 0 0 1-5.36-3.72L86.92 16.64a50 50 0 0 0-3.72-5.36L35.42 86.64z" fill="#5E6AD2"/></svg>`;
const SAVE_SVG = `<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>`;
const WEBHOOK_SVG = `<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71"/></svg>`;

const PROVIDER_META = {
  openai:    { img: 'assets/openai.svg',    filter: 'invert(1)',                                                          bgColor: 'rgba(16,163,127,0.15)' },
  google:    { img: 'assets/gemini.svg',    filter: 'invert(48%) sepia(90%) saturate(400%) hue-rotate(190deg)',           bgColor: 'rgba(66,133,244,0.15)' },
  anthropic: { img: 'assets/anthropic.svg', filter: 'invert(55%) sepia(80%) saturate(500%) hue-rotate(10deg)',            bgColor: 'rgba(217,119,6,0.15)'  },
  local:     { img: 'assets/local-llm.svg', filter: 'invert(68%) sepia(60%) saturate(400%) hue-rotate(220deg)',           bgColor: 'rgba(139,92,246,0.15)' },
};

const CLOUD_MODELS = {
  openai:    ['gpt-4.1', 'gpt-4.1-mini', 'gpt-4.1-nano', 'gpt-4o', 'gpt-4o-mini', 'o3-mini'],
  google:    ['gemini-2.5-pro', 'gemini-2.5-flash', 'gemini-2.0-flash', 'gemini-1.5-flash', 'gemini-1.5-pro'],
  anthropic: ['claude-sonnet-4-6', 'claude-opus-4-6', 'claude-haiku-4-5-20251001'],
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
  save:    { svg: SAVE_SVG,    textColor: '#10b981',         bgColor: 'rgba(16,185,129,0.15)' },
  slack:   { svg: SLACK_SVG,   textColor: '#fff',         bgColor: '#4A154B' },
  notion:  { svg: NOTION_SVG,  textColor: '#fff',            bgColor: '#2f2f2f' },
  webhook: { svg: WEBHOOK_SVG, textColor: '#60a5fa',          bgColor: 'rgba(59,130,246,0.2)' },
  linear:  { svg: LINEAR_SVG,  textColor: '#fff',            bgColor: '#5E6AD2' },
  mcp:     { abbr: 'MCP',  textColor: '#f59e0b',         bgColor: 'rgba(245,158,11,0.15)' },
};

// Shared pipeline flow renderer — used in both builder preview and recording detail/status views.
// opts.compact (bool): icon-only chips (for cards); false = icon+label chips (for status/builder preview)
// opts.statuses (object): { stepName: 'done'|'failed'|'running'|'skipped' }
function renderPipelineFlowHTML(steps, opts = {}) {
  const { compact = false, statuses = {} } = opts;
  const MIC_SVG = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" y1="19" x2="12" y2="23"/><line x1="8" y1="23" x2="16" y2="23"/></svg>`;
  const arrow = `<div class="pflow-arrow">›</div>`;

  let html = `<div class="pflow-chip pflow-chip--source" title="Transcript"><div class="pflow-chip-icon" style="background:var(--accent-soft);color:var(--accent);">${MIC_SVG}</div>${compact ? '' : '<span class="pflow-chip-label">Transcript</span>'}</div>`;

  for (const step of (steps || [])) {
    html += arrow;
    let meta = CONNECTOR_META[step.connector] || { abbr: step.connector.substring(0, 2).toUpperCase(), textColor: 'var(--text-primary)', bgColor: 'var(--bg-input)' };
    let iconContent = '';
    let bg = meta.bgColor;
    let fg = meta.textColor;

    if (step.connector === 'llm') {
      const provider = step.config?.provider || 'openai';
      const provMeta = PROVIDER_META[provider] || PROVIDER_META.openai;
      bg = provMeta.bgColor;
      iconContent = `<img src="${provMeta.img}" style="filter:${provMeta.filter};" alt="${provider}" />`;
    } else if (meta.svg) {
      iconContent = meta.svg;
    } else {
      iconContent = `<span style="font-size:7px;font-weight:800;color:${fg};">${meta.abbr}</span>`;
    }

    const st = statuses[step.name];
    const stClass = st ? ` pflow-chip--${st}` : '';
    const safeName = escapeHtml(step.name || '');

    html += `<div class="pflow-chip${stClass}" title="${safeName}"><div class="pflow-chip-icon" style="background:${bg};color:${fg};">${iconContent}</div>${compact ? '' : `<span class="pflow-chip-label">${safeName}</span>`}</div>`;
  }

  return `<div class="pflow${compact ? ' pflow--compact' : ''}">${html}</div>`;
}

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
    icon: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><polyline points="10 9 9 9 8 9"/></svg>',
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
    icon: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 11-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>',
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
    icon: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 4h2a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6a2 2 0 012-2h2"/><rect x="8" y="2" width="8" height="4" rx="1" ry="1"/></svg>',
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
    icon: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="6" width="20" height="12" rx="2"/><path d="M12 12h.01"/><path d="M17 12h.01"/><path d="M7 12h.01"/></svg>',
    step: {
      name: 'structure',
      connector: 'llm',
      input: 'transcript',
      config: { prompt_template: 'structure' },
      description: 'Organize content into sections'
    }
  },
  {
    label: 'MCP Tool',
    icon: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/></svg>',
    step: {
      name: 'mcp-tool',
      connector: 'mcp',
      input: 'transcript',
      config: { url: '', tool: '' },
      description: 'Call an MCP tool'
    }
  },
  {
    label: 'Custom Prompt',
    icon: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>',
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
      icon: NOTION_SVG,
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
      icon: LINEAR_SVG,
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
      icon: SAVE_SVG,
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
      icon: SLACK_SVG,
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
      icon: WEBHOOK_SVG,
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
  const needsConfig = ['slack', 'notion', 'linear', 'webhook', 'save', 'mcp'];
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
      showToast('Prompt text is required', 'error');
      return;
    }

    const saveAsTemplate = checkbox.checked;
    let stepConfig = {};

    if (saveAsTemplate) {
      const templateName = formEl.querySelector('.custom-prompt-name-input').value.trim();
      if (!templateName) {
        showToast('Template name is required when saving as template', 'error');
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
        showToast('Failed to save template: ' + err, 'error');
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
    const updated = p.updated_at ? new Date(p.updated_at).toLocaleDateString() : '';
    const flowHtml = renderPipelineFlowHTML(p.steps || [], { compact: true });
    const meta = [safeDesc, updated].filter(Boolean).join(' &middot; ');
    return `
    <div class="pipeline-def-item" data-name="${safeName}">
      <div class="pipeline-def-info">
        <div class="pipeline-def-name">${safeName}</div>
        <div class="pipeline-def-flow">${flowHtml}</div>
        ${meta ? `<div class="pipeline-def-desc">${meta}</div>` : ''}
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
}

function closePipelineEditor() {
  if (pipelineEditor) pipelineEditor.style.display = 'none';
  editingPipelineDef = null;
  editingStepIndex = null;
  pipelineEditorSteps = [];
  closeStepEditorPanel();
}

function closeStepEditorPanel() {
  if (stepEditorPanelEl) {
    stepEditorPanelEl.innerHTML = '';
    stepEditorPanelEl.style.display = 'none';
  }
}

function renderPipelineSteps() {
  if (!pipelineStepsListEl) return;

  const MIC_SVG = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" y1="19" x2="12" y2="23"/><line x1="8" y1="23" x2="16" y2="23"/></svg>`;

  // Source chip (always first, non-interactive)
  let html = `<div class="pflow-chip pflow-chip--source" title="Transcript">
    <div class="pflow-chip-icon" style="background:var(--accent-soft);color:var(--accent);">${MIC_SVG}</div>
    <span class="pflow-chip-label">Transcript</span>
  </div>`;

  for (let i = 0; i < pipelineEditorSteps.length; i++) {
    const step = pipelineEditorSteps[i];
    let meta = CONNECTOR_META[step.connector] || {
      abbr: step.connector.substring(0, 2).toUpperCase(),
      textColor: 'var(--text-primary)',
      bgColor: 'var(--bg-input)'
    };
    let iconContent = '';
    let bg = meta.bgColor;
    let fg = meta.textColor;
    let subText = escapeHtml(step.connector);

    if (step.connector === 'llm') {
      const provider = step.config?.provider || 'openai';
      const provMeta = PROVIDER_META[provider] || PROVIDER_META.openai;
      bg = provMeta.bgColor;
      iconContent = `<img src="${provMeta.img}" style="filter:${provMeta.filter};" alt="${provider}" />`;
      const model = step.config?.model || '';
      if (model) {
        let short;
        if (provider === 'local') {
          const localModel = (typeof llmModelsData !== 'undefined') ? llmModelsData.find(m => m.id === model) : null;
          short = localModel ? localModel.name : model;
        } else {
          short = trimModelName(model, provider);
        }
        subText = escapeHtml(short);
      } else {
        subText = escapeHtml(provider);
      }
    } else if (meta.svg) {
      iconContent = meta.svg;
    } else {
      iconContent = `<span style="font-size:7px;font-weight:800;color:${fg};">${meta.abbr}</span>`;
    }

    const safeName = escapeHtml(step.name || 'Unnamed');
    const isEditing = editingStepIndex === i;

    html += `<div class="pflow-chip${isEditing ? ' pflow-chip--editing' : ''}" data-index="${i}" title="${safeName}">
      <span class="pflow-chip-num">${i + 1}</span>
      <div class="pflow-chip-icon" style="background:${bg};color:${fg};">${iconContent}</div>
      <div class="pflow-chip-label-group">
        <span class="pflow-chip-label">${safeName}</span>
        <span class="pflow-chip-sub">${subText}</span>
      </div>
      <button class="pflow-chip-remove" data-index="${i}" title="Remove step" aria-label="Remove step">×</button>
    </div>`;
  }

  // Add step chip (dashed ghost)
  html += `<div class="pflow-chip pflow-chip--add" id="add-step-tile" title="Add step">
    <div class="pflow-chip-icon">+</div>
    <span class="pflow-chip-label">Add Step</span>
  </div>`;

  pipelineStepsListEl.innerHTML = `<div class="pflow pflow--builder">${html}</div>`;
  const pfFlowEl = pipelineStepsListEl.querySelector('.pflow--builder');

  // Wire: click step chip to open editor
  pfFlowEl.querySelectorAll('.pflow-chip[data-index]').forEach(chip => {
    chip.addEventListener('click', (e) => {
      if (e.target.closest('.pflow-chip-remove')) return;
      showStepEditor(parseInt(chip.dataset.index));
    });
  });

  // Wire: remove buttons
  pfFlowEl.querySelectorAll('.pflow-chip-remove').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const idx = parseInt(btn.dataset.index);
      const stepName = pipelineEditorSteps[idx]?.name || `Step ${idx + 1}`;
      const ok = await showConfirm('Remove Step?', `Remove step "${stepName}" from pipeline?`);
      if (!ok) return;
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

  // Wire: add step chip
  const addChip = document.getElementById('add-step-tile');
  if (addChip) {
    addChip.addEventListener('click', (e) => {
      e.stopPropagation();
      togglePicker();
    });
  }

  // Initialize Sortable.js for drag-and-drop reordering
  if (sortableInstance) {
    sortableInstance.destroy();
    sortableInstance = null;
  }
  if (typeof Sortable !== 'undefined' && pfFlowEl) {
    sortableInstance = Sortable.create(pfFlowEl, {
      draggable: '.pflow-chip[data-index]',
      filter: '.pflow-chip--source, .pflow-chip--add',
      ghostClass: 'sortable-ghost',
      chosenClass: 'sortable-chosen',
      dragClass: 'sortable-drag',
      animation: 150,
      onEnd(evt) {
        const movedChip = evt.item;
        const movedIdx = parseInt(movedChip.dataset.index);
        // Determine new semantic index from DOM order of [data-index] chips
        const allStepChips = [...pfFlowEl.querySelectorAll('.pflow-chip[data-index]')];
        const newIdx = allStepChips.indexOf(movedChip);
        if (movedIdx === newIdx || newIdx < 0) { renderPipelineSteps(); return; }
        const [moved] = pipelineEditorSteps.splice(movedIdx, 1);
        pipelineEditorSteps.splice(newIdx, 0, moved);
        if (editingStepIndex === movedIdx) editingStepIndex = newIdx;
        else if (editingStepIndex !== null) {
          if (movedIdx < editingStepIndex && newIdx >= editingStepIndex) editingStepIndex--;
          else if (movedIdx > editingStepIndex && newIdx <= editingStepIndex) editingStepIndex++;
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

function showStepEditor(index) {
  const step = pipelineEditorSteps[index];
  if (!step) return;

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
    const currentProvider = step.config?.provider || 'openai';
    const currentModel = step.config?.model || '';
    const providerModels = CLOUD_MODELS[currentProvider] || [];
    const modelOptions = currentProvider === 'local'
      ? (typeof llmModelsData !== 'undefined' ? llmModelsData.filter(m => m.downloaded).map(m =>
          `<option value="${escapeHtml(m.id)}" ${currentModel === m.id ? 'selected' : ''}>${escapeHtml(m.name)} (${escapeHtml(m.params)})</option>`
        ).join('') : '')
      : providerModels.map(m =>
          `<option value="${escapeHtml(m)}" ${currentModel === m ? 'selected' : ''}>${escapeHtml(m)}</option>`
        ).join('');
    configFields = `
      ${promptField}
      <div class="step-editor-row"><label>Provider</label><select data-field="provider" class="llm-provider-select">
        <option value="openai" ${currentProvider === 'openai' ? 'selected' : ''}>OpenAI</option>
        <option value="google" ${currentProvider === 'google' ? 'selected' : ''}>Google</option>
        <option value="anthropic" ${currentProvider === 'anthropic' ? 'selected' : ''}>Anthropic</option>
        <option value="local" ${currentProvider === 'local' ? 'selected' : ''}>Local LLM</option>
      </select></div>
      <div class="step-editor-row"><label>Model</label><select data-field="model" class="llm-model-select">
        ${modelOptions}
      </select></div>
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
      <div class="step-editor-row"><label>URL</label><input data-field="url" value="${escapeHtml(step.config?.url || '')}" placeholder="https://mcp.example.com" /></div>
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

  // Update editing index and re-render chips to show active state
  editingStepIndex = index;
  renderPipelineSteps();

  // Build editor into the panel below chips
  const editorEl = document.createElement('div');
  editorEl.className = 'step-editor';
  editorEl.innerHTML = `
    <div class="step-editor-header">
      <span class="step-editor-title">Step ${index + 1} — ${escapeHtml(step.name || 'Unnamed')}</span>
      <button class="step-editor-close" title="Close editor">×</button>
    </div>
    <div id="step-config-fields">
      <div class="step-editor-row"><label>Step Name</label><input class="step-name-input" value="${escapeHtml(step.name || '')}" placeholder="Step name" /></div>
      ${configFields}
      ${index > 0 ? (() => {
        const currentInput = step.input || 'transcript';
        const inputOptions = ['transcript', ...pipelineEditorSteps.slice(0, index).map(s => s.name).filter(Boolean)]
          .map(v => `<option value="${escapeHtml(v)}" ${currentInput === v ? 'selected' : ''}>${escapeHtml(v === 'transcript' ? 'Transcript' : v)}</option>`)
          .join('');
        return `<div class="step-editor-row"><label>Input</label><select class="step-input-select">${inputOptions}</select></div>`;
      })() : ''}
    </div>
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
          const prefix = ch.is_private ? '\uD83D\uDD12 ' : '#';
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

  // LLM provider change → update model dropdown
  const llmProviderSelect = editorEl.querySelector('.llm-provider-select');
  if (llmProviderSelect) {
    llmProviderSelect.addEventListener('change', () => {
      const newProvider = llmProviderSelect.value;
      const modelSelect = editorEl.querySelector('.llm-model-select');
      if (!modelSelect) return;
      let opts = '';
      if (newProvider === 'local') {
        const models = (typeof llmModelsData !== 'undefined') ? llmModelsData.filter(m => m.downloaded) : [];
        opts = models.map(m =>
          `<option value="${escapeHtml(m.id)}">${escapeHtml(m.name)} (${escapeHtml(m.params)})</option>`
        ).join('');
        if (models.length === 0) {
          opts = '<option value="" disabled>No local models downloaded</option>';
        }
      } else {
        const models = CLOUD_MODELS[newProvider] || [];
        opts = models.map(m => `<option value="${escapeHtml(m)}">${escapeHtml(m)}</option>`).join('');
      }
      modelSelect.innerHTML = opts;
    });
  }

  // Done button — save and collapse panel
  editorEl.querySelector('.step-editor-done').addEventListener('click', () => {
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
    // Use explicit step name from input, fall back to auto-derive for llm steps
    const nameInput = editorEl.querySelector('.step-name-input');
    const nameVal = nameInput ? nameInput.value.trim() : '';
    if (nameVal) {
      step.name = nameVal;
    } else if (step.connector === 'llm' && step.config.prompt_template) {
      step.name = step.config.prompt_template;
    }
    // Persist user-selected input source (transcript or previous step name)
    const inputSelect = editorEl.querySelector('.step-input-select');
    if (inputSelect) {
      step.input = inputSelect.value || 'transcript';
    }

    fixStepInputs();
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
    if (!name) { showToast('Pipeline name is required', 'error'); return; }

    // Validate step names
    for (let i = 0; i < pipelineEditorSteps.length; i++) {
      if (!pipelineEditorSteps[i].name.trim()) {
        showToast(`Step ${i + 1} needs a name`, 'error');
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
      showToast('Failed to save: ' + err, 'error');
    }
  });
}

if (deletePipelineDefBtn) {
  deletePipelineDefBtn.addEventListener('click', async () => {
    if (!editingPipelineDef) return;
    const ok = await showConfirm('Delete Pipeline?', `Delete pipeline "${editingPipelineDef}"? This cannot be undone.`);
    if (!ok) return;
    try {
      await invoke('delete_pipeline', { name: editingPipelineDef });
      closePipelineEditor();
      await loadPipelineDefs();
    } catch (err) {
      console.error('Failed to delete pipeline:', err);
      showToast('Failed to delete: ' + err, 'error');
    }
  });
}
