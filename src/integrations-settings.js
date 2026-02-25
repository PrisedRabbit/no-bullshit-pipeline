// integrations-settings.js — Integrations settings page (Phase 4)
// State-first full re-render pattern. This module owns the Connected/Available layout.

// Brand SVGs for integration icons
// Asset sources and licensing:
// - OpenAI logo: Official trademark, used with permission per OpenAI brand guidelines
// - Google/Gemini logo: Google brand assets, used per Google brand guidelines
// - Anthropic logo: Official trademark, used per Anthropic brand guidelines
// - Notion logo: Official trademark, used per Notion brand guidelines
// - Linear logo: Official trademark, used per Linear brand guidelines
// - Slack logo: Official trademark, used per Slack brand guidelines
// All icons displayed with brand-consistent colors: solid background with white (inverted) icon
const INT_NOTION_SVG = `<svg viewBox="0 0 100 100" width="18" height="18" fill="currentColor"><path d="M6.017 4.313l55.333-4.087c6.797-.583 8.543-.19 12.817 2.917l17.663 12.443c2.913 2.14 3.883 2.723 3.883 5.053v68.243c0 4.277-1.553 6.807-6.99 7.193L24.467 99.967c-4.08.193-6.023-.39-8.16-3.113L3.3 79.94c-2.333-3.113-3.3-5.443-3.3-8.167V11.113c0-3.497 1.553-6.413 6.017-6.8z" fill="#fff"/><path d="M61.35.227l-55.333 4.087C1.553 4.7 0 7.617 0 11.113v60.66c0 2.723.967 5.053 3.3 8.167l13.007 16.913c2.137 2.723 4.08 3.307 8.16 3.113l64.257-3.89c5.433-.387 6.99-2.917 6.99-7.193V20.64c0-2.21-.81-2.76-3.088-4.587L75.983 3.523C71.71.607 69.96.22 63.163.803L61.35.227z" fill="#000"/><path d="M26.395 18.768c-5.433.39-6.675.477-9.768-1.753L7.997 10.527c-1.163-.913-1.55-1.94-1.55-3.113.39-2.53 1.94-4.47 7.377-4.86l53.39-3.89c4.47-.39 6.603 1.553 8.157 2.723l10.133 7.577c.39.193 1.553 1.553 0 1.553l-55.14 3.11v5.14z" fill="#fff"/><path d="M19.018 88.4V30.173c0-2.527.78-3.697 3.113-3.89l57.277-3.307c2.14-.193 3.113 1.167 3.113 3.693V85.09c0 2.527-.39 4.667-3.887 4.86l-54.943 3.113c-3.5.193-4.673-1.003-4.673-4.663zm54.167-55.13c.39 1.75 0 3.5-1.75 3.697l-2.527.39v40.257c-2.14 1.163-4.277 1.75-5.833 1.75-2.723 0-3.5-.583-5.443-3.113L38.468 45.948V74.7l5.247 1.163s0 3.5-4.86 3.5l-13.393.78c-.39-.78 0-2.723 1.36-3.113l3.497-.97V38.33l-4.86-.39c-.39-2.14 1.163-3.697 3.307-4.473l14.363-.97 20.603 31.46V35.077l-4.47-.39c-.39-2.14 1.163-3.697 3.113-3.89l14.003-.527z" fill="#fff"/></svg>`;
const INT_LINEAR_SVG = `<svg viewBox="0 0 100 100" width="18" height="18"><path d="M2.76 62.7a50.1 50.1 0 0 1-1.52-4.44L62.7 2.76a50.1 50.1 0 0 0-4.44-1.52L2.76 62.7zm7.66 12.48a50 50 0 0 1-3.54-4.3L75.18 4.58a50 50 0 0 0-4.3-3.54L10.42 75.18zm11.44 8.96a50 50 0 0 1-4.82-4.1L83.14 13.94a50 50 0 0 0-4.1-4.82L21.86 84.14zM0 50a49.9 49.9 0 0 0 .26 5L55 .26A50 50 0 1 0 0 50zm35.42 36.64a50 50 0 0 1-5.36-3.72L86.92 16.64a50 50 0 0 0-3.72-5.36L35.42 86.64z" fill="#5E6AD2"/></svg>`;
const INT_SLACK_SVG = `<svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor"><path d="M5.042 15.165a2.528 2.528 0 0 1-2.52 2.523A2.528 2.528 0 0 1 0 15.165a2.527 2.527 0 0 1 2.522-2.52h2.52v2.52zm1.271 0a2.527 2.527 0 0 1 2.521-2.52 2.527 2.527 0 0 1 2.521 2.52v6.313A2.528 2.528 0 0 1 8.834 24a2.528 2.528 0 0 1-2.521-2.522v-6.313zm2.521-10.123a2.528 2.528 0 0 1-2.521-2.52A2.528 2.528 0 0 1 8.834 0a2.528 2.528 0 0 1 2.521 2.522v2.52H8.834zm0 1.271a2.528 2.528 0 0 1 2.521 2.521 2.528 2.528 0 0 1-2.521 2.521H2.522A2.528 2.528 0 0 1 0 8.834a2.528 2.528 0 0 1 2.522-2.521h6.312zm10.123 2.521a2.528 2.528 0 0 1 2.522-2.521A2.528 2.528 0 0 1 24 8.834a2.528 2.528 0 0 1-2.522 2.521h-2.522V8.834zm-1.268 0a2.528 2.528 0 0 1-2.523 2.521 2.527 2.527 0 0 1-2.52-2.521V2.522A2.527 2.527 0 0 1 15.165 0a2.528 2.528 0 0 1 2.523 2.522v6.312zm-2.523 10.123a2.528 2.528 0 0 1 2.523 2.522A2.528 2.528 0 0 1 15.165 24a2.527 2.527 0 0 1-2.52-2.522v-2.522h2.52zm0-1.268a2.527 2.527 0 0 1-2.52-2.523 2.526 2.526 0 0 1 2.52-2.52h6.313A2.527 2.527 0 0 1 24 15.165a2.528 2.528 0 0 1-2.522 2.523h-6.313z"/></svg>`;
const INT_FOLDER_SVG = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4l2 2h10a2 2 0 0 1 2 2z"></path></svg>`;
const INT_LINK_SVG = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path></svg>`;

// Module state — use `var` so these are on `window` and accessible from main.js
var notionProfiles = [];
var linearProfiles = [];
var savePathIntegrations = [];
var webhookProfiles = [];

const connectedListEl = () => document.getElementById('connected-integrations-list');
const availableListEl = () => document.getElementById('available-integrations-list');

// ===== LOAD ALL INTEGRATIONS =====
async function loadAllIntegrations() {
  await Promise.all([
    loadNotionProfiles(),
    loadLinearProfiles(),
    loadSlackForIntegrations(),
    loadSavePathIntegrations(),
    loadWebhookProfiles(),
  ]);
  renderModelsProviders();
  renderLocalLlmModels();
  renderConnectedIntegrations();
  renderAvailableIntegrations();
}

// ===== PROVIDER MODELS =====
var providerModels = {};
var providerModelsFetching = false;

async function fetchProviderModels(provider) {
  try {
    const models = await window.__TAURI__.core.invoke('fetch_provider_models', { provider });
    providerModels[provider] = { models, error: null };
  } catch (err) {
    providerModels[provider] = { models: [], error: String(err) };
  }
}

async function refreshAllProviderModels() {
  if (providerModelsFetching) return;
  providerModelsFetching = true;
  const apiKeys = (appSettings && appSettings.transcription && appSettings.transcription.api_keys) || {};
  const fetches = [];
  for (const provider of ['openai', 'google', 'anthropic']) {
    if (apiKeys[provider]) {
      fetches.push(fetchProviderModels(provider));
    }
  }
  // Ollama is local — no API key needed
  fetches.push(fetchProviderModels('ollama'));
  await Promise.all(fetches);
  renderModelsProviders();
  providerModelsFetching = false;
}

var providerModelsExpanded = {};

function renderProviderModelsList(providerId) {
  const data = providerModels[providerId];
  const expanded = providerModelsExpanded[providerId] || false;

  if (!data) {
    const storedModels = (appSettings && appSettings.providers && appSettings.providers[providerId] && appSettings.providers[providerId].models) || [];
    if (storedModels.length === 0) return '';
    const chips = storedModels.map(id => {
      return `<div class="provider-model-item">
      <span class="provider-model-id">${escapeHtml(id)}</span>
    </div>`;
    }).join('');
    return `<div class="provider-models-section">
    <div class="provider-models-header">
      <span class="provider-models-count">${storedModels.length} LLM${storedModels.length !== 1 ? 's' : ''} available</span>
    </div>
    <div class="provider-models-list">${chips}</div>
  </div>`;
  }

  if (data.error) {
    const storedModels = (appSettings && appSettings.providers && appSettings.providers[providerId] && appSettings.providers[providerId].models) || [];
    if (storedModels.length === 0) {
      return `<div class="provider-models-section"><span class="provider-models-error">Failed to load LLMs</span></div>`;
    }
    const chips = storedModels.map(id => {
      return `<div class="provider-model-item">
      <span class="provider-model-id">${escapeHtml(id)}</span>
    </div>`;
    }).join('');
    return `<div class="provider-models-section">
    <span class="provider-models-error">Failed to load LLMs</span>
    <div class="provider-models-header">
      <span class="provider-models-count">${storedModels.length} LLM${storedModels.length !== 1 ? 's' : ''} available</span>
    </div>
    <div class="provider-models-list">${chips}</div>
  </div>`;
  }

  if (data.models.length === 0) return '';

  const CAP_COLORS = {
    chat: 'rgba(59,130,246,0.2)',
    transcription: 'rgba(16,185,129,0.2)',
    embedding: 'rgba(168,85,247,0.2)',
    'text-to-speech': 'rgba(245,158,11,0.2)',
    image: 'rgba(239,68,68,0.2)',
  };

  const recommended = RECOMMENDED_MODELS[providerId] || [];
  const recommendedSet = new Set(recommended);
  const recommendedModels = data.models.filter(m => recommendedSet.has(m.id));
  const otherModels = data.models.filter(m => !recommendedSet.has(m.id) && !m.deprecated);

  let displayModels;
  let showToggle = false;
  const MAX_DEFAULT = 4;
  if (expanded) {
    displayModels = [...recommendedModels, ...otherModels];
  } else {
    displayModels = recommendedModels.length > 0 ? recommendedModels.slice(0, MAX_DEFAULT) : otherModels.slice(0, MAX_DEFAULT);
    showToggle = otherModels.length > 0 || recommendedModels.length > MAX_DEFAULT;
  }

  const chips = displayModels.map(m => {
    const capBadges = m.capabilities.map(c => {
      const bg = CAP_COLORS[c] || 'rgba(148,163,184,0.2)';
      return `<span class="provider-model-cap" style="background:${bg}">${escapeHtml(c)}</span>`;
    }).join('');
    const deprecatedBadge = m.deprecated
      ? '<span class="provider-model-cap" style="background:rgba(239,68,68,0.25);color:rgba(239,68,68,0.9)">deprecated</span>'
      : '';
    const displayName = m.name !== m.id ? escapeHtml(m.name) : '';
    const isRecommended = recommendedSet.has(m.id);
    const recBadge = isRecommended ? '<span class="provider-model-cap" style="background:rgba(34,197,94,0.2);color:rgba(34,197,94,0.9)">recommended</span>' : '';
    return `<div class="provider-model-item"${m.deprecated ? ' style="opacity:0.55"' : ''}>
      <span class="provider-model-id">${escapeHtml(m.id)}</span>
      ${displayName ? `<span class="provider-model-name">${displayName}</span>` : ''}
      ${recBadge}${capBadges}${deprecatedBadge}
    </div>`;
  }).join('');

  const toggleBtn = showToggle
    ? `<button class="provider-models-toggle" data-provider="${escapeHtml(providerId)}" style="font-size:0.68rem;color:var(--accent);background:none;border:none;cursor:pointer;padding:4px 0;">Show all ${data.models.length} LLMs</button>`
    : expanded && (otherModels.length > 0 || recommendedModels.length < data.models.length)
    ? `<button class="provider-models-toggle" data-provider="${escapeHtml(providerId)}" style="font-size:0.68rem;color:var(--text-secondary);background:none;border:none;cursor:pointer;padding:4px 0;">Show less</button>`
    : '';

  return `<div class="provider-models-section">
    <div class="provider-models-header">
      <span class="provider-models-count">${displayModels.length} of ${data.models.length} LLMs</span>
    </div>
    <div class="provider-models-list">${chips}</div>
    ${toggleBtn}
  </div>`;
}

function renderOllamaCard() {
  // Kept for backward compat — actual rendering handled by renderModelsProviders
  renderModelsProviders();
}

// ===== RENDER PROCESSING PROVIDERS =====
async function validateApiKey(provider, key) {
  try {
    if (provider === 'openai') {
      const r = await fetch('https://api.openai.com/v1/models', {
        headers: { 'Authorization': `Bearer ${key}` }
      });
      return r.ok;
    } else if (provider === 'google') {
      const r = await fetch(`https://generativelanguage.googleapis.com/v1beta/models?key=${encodeURIComponent(key)}`);
      return r.ok;
    } else if (provider === 'anthropic') {
      const r = await fetch('https://api.anthropic.com/v1/models', {
        headers: { 'x-api-key': key, 'anthropic-version': '2023-06-01' }
      });
      return r.ok;
    }
  } catch { return false; }
  return false;
}

// ===== PROVIDER-FIRST MODELS UI =====

const CLOUD_PROVIDERS = [
  { id: 'openai',    name: 'OpenAI',    desc: 'GPT-4o, Whisper, real-time transcription', placeholder: 'sk-...',     icon: { img: 'assets/openai.svg',    filter: 'invert(1)', bg: '#000' } },
  { id: 'google',    name: 'Google AI',  desc: 'Gemini long-context processing',           placeholder: 'AIza...',    icon: { img: 'assets/gemini.svg',    filter: 'invert(1)', bg: '#4285F4' } },
  { id: 'anthropic', name: 'Anthropic',  desc: 'Claude structured extraction',             placeholder: 'sk-ant-...', icon: { img: 'assets/anthropic.svg', filter: 'invert(1)', bg: '#D97706' } },
];

const CAP_BADGE_COLORS = {
  'Transcription': 'rgba(16,185,129,0.18)',
  'Processing':    'rgba(59,130,246,0.18)',
  'Embedding':     'rgba(168,85,247,0.18)',
};

const RECOMMENDED_MODELS = {
  openai: ['gpt-4.1', 'gpt-4o', 'gpt-4o-mini', 'o4-mini'],
  anthropic: ['claude-sonnet-4-20250514', 'claude-3-5-sonnet-20241022', 'claude-3-5-haiku-20241022', 'claude-opus-4-20250514'],
  google: ['gemini-2.5-pro-preview-06-05', 'gemini-2.0-flash', 'gemini-1.5-pro', 'gemini-1.5-flash'],
  ollama: [],
};

function renderCapBadges(capabilities) {
  return capabilities.map(c => {
    const bg = CAP_BADGE_COLORS[c] || 'rgba(148,163,184,0.15)';
    return `<span class="provider-cap-badge" style="background:${bg}">${escapeHtml(c)}</span>`;
  }).join('');
}

function updateProviderKeyStatus(providerId, state) {
  const statusEl = document.getElementById(`key-status-${providerId}`);
  if (!statusEl) return;
  const STATUS_CONFIG = {
    missing: { class: 'key-missing', label: 'No key', ariaLabel: 'API key not configured', icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>` },
    saved: { class: 'key-saved', label: 'Saved', ariaLabel: 'API key saved (not yet verified)', icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/></svg>` },
    checking: { class: 'key-checking', label: 'Verifying', ariaLabel: 'Verifying API key', icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="2" x2="12" y2="6"/><line x1="12" y1="18" x2="12" y2="22"/><line x1="4.93" y1="4.93" x2="7.76" y2="7.76"/><line x1="16.24" y1="16.24" x2="19.07" y2="19.07"/><line x1="2" y1="12" x2="6" y2="12"/><line x1="18" y1="12" x2="22" y2="12"/><line x1="4.93" y1="19.07" x2="7.76" y2="16.24"/><line x1="16.24" y1="7.76" x2="19.07" y2="4.93"/></svg>` },
    valid: { class: 'key-valid', label: 'Verified', ariaLabel: 'API key verified successfully', icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>` },
    failed: { class: 'key-failed', label: 'Invalid', ariaLabel: 'API key verification failed', icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>` }
  };
  const config = STATUS_CONFIG[state] || STATUS_CONFIG.missing;
  statusEl.className = `provider-key-status ${config.class}`;
  statusEl.setAttribute('aria-label', config.ariaLabel);
  statusEl.innerHTML = `${config.icon}<span>${config.label}</span>`;
}

function renderProviderKeyStatus(state, providerId) {
  const STATUS_CONFIG = {
    missing: {
      class: 'key-missing',
      label: 'No key',
      ariaLabel: 'API key not configured',
      icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>`
    },
    saved: {
      class: 'key-saved',
      label: 'Saved',
      ariaLabel: 'API key saved (not yet verified)',
      icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/></svg>`
    },
    checking: {
      class: 'key-checking',
      label: 'Verifying',
      ariaLabel: 'Verifying API key',
      icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="2" x2="12" y2="6"/><line x1="12" y1="18" x2="12" y2="22"/><line x1="4.93" y1="4.93" x2="7.76" y2="7.76"/><line x1="16.24" y1="16.24" x2="19.07" y2="19.07"/><line x1="2" y1="12" x2="6" y2="12"/><line x1="18" y1="12" x2="22" y2="12"/><line x1="4.93" y1="19.07" x2="7.76" y2="16.24"/><line x1="16.24" y1="7.76" x2="19.07" y2="4.93"/></svg>`
    },
    valid: {
      class: 'key-valid',
      label: 'Verified',
      ariaLabel: 'API key verified successfully',
      icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>`
    },
    failed: {
      class: 'key-failed',
      label: 'Invalid',
      ariaLabel: 'API key verification failed',
      icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>`
    }
  };
  const config = STATUS_CONFIG[state] || STATUS_CONFIG.missing;
  return `<span class="provider-key-status ${config.class}" id="key-status-${providerId}" role="status" aria-label="${config.ariaLabel}">${config.icon}<span>${config.label}</span></span>`;
}

function renderModelsProviders() {
  const el = document.getElementById('models-providers-list');
  if (!el) return;

  const apiKeys = (appSettings && appSettings.transcription && appSettings.transcription.api_keys) || {};
  const providerConfigs = (appSettings && appSettings.providers) || {};

  const sections = [];

  // Cloud provider sections
  for (const p of CLOUD_PROVIDERS) {
    const config = providerConfigs[p.id] || {};
    const caps = config.capabilities || [];
    const key = apiKeys[p.id] || '';
    const hasKey = !!key;
    const displayValue = hasKey ? maskApiKey(key) : '';
    const validatedKeys = window.__nbpValidatedKeys || {};
    const failedKeys = window.__nbpFailedKeys || {};
    const isValidated = validatedKeys[p.id] === key && !!key;
    const isFailed = failedKeys[p.id] === key;
    const keyStatus = !hasKey ? 'missing' : isFailed ? 'failed' : isValidated ? 'valid' : 'saved';
    const statusHtml = renderProviderKeyStatus(keyStatus, p.id);
    const modelsHtml = renderProviderModelsList(p.id);

    sections.push(`
      <div class="connections-group" data-provider-section="${escapeHtml(p.id)}">
        <div class="connections-group-header">
          <div class="provider-card-icon-wrapper">
            <div class="provider-card-icon ${escapeHtml(p.id)}" style="background:${p.icon.bg};display:flex;align-items:center;justify-content:center;">
              <img src="${escapeHtml(p.icon.img)}" style="width:20px;height:20px;filter:${p.icon.filter}" />
            </div>
            <span class="provider-card-icon-label">${escapeHtml(p.name)}</span>
          </div>
          <div class="group-info">
            <div class="group-label">${escapeHtml(p.name)} ${renderCapBadges(caps)}</div>
            <div class="group-desc">${escapeHtml(p.desc)}</div>
          </div>
        </div>
        <div class="provider-card" data-provider="${escapeHtml(p.id)}" style="border:none;padding:0;background:none;">
          <div class="provider-card-input" style="width:100%;justify-content:flex-start;gap:8px;">
            <input
              id="settings-api-key-${escapeHtml(p.id)}"
              type="password"
              placeholder="${escapeHtml(p.placeholder)}"
              class="settings-input-text"
              value="${escapeHtml(displayValue)}"
              data-original-key="${escapeHtml(key)}"
              style="width:200px;"
            />
            <button class="mini-action-btn provider-save-btn" data-provider="${escapeHtml(p.id)}">Save</button>
            ${statusHtml}
          </div>
        </div>
        ${modelsHtml ? `<div class="provider-card-wrapper">${modelsHtml}</div>` : ''}
        <button class="mini-action-btn refresh-provider-btn" data-provider="${escapeHtml(p.id)}" style="align-self:flex-start;margin-top:4px;font-size:0.75rem;">Refresh LLMs</button>
      </div>
    `);
  }

  // Local / Ollama section
  const localConfig = providerConfigs['local'] || {};
  const ollamaConfig = providerConfigs['ollama'] || {};
  const localCaps = [...new Set([...(localConfig.capabilities || []), ...(ollamaConfig.capabilities || [])])];
  const ollamaModelsHtml = renderProviderModelsList('ollama');

  sections.push(`
    <div class="connections-group" data-provider-section="local">
      <div class="connections-group-header">
        <div class="provider-card-icon-wrapper">
          <div class="provider-card-icon ollama" style="background:#000;display:flex;align-items:center;justify-content:center;">
            <img src="assets/ollama.svg" style="width:20px;height:20px;filter:invert(1)" />
          </div>
          <span class="provider-card-icon-label">Ollama</span>
        </div>
        <div class="group-info">
          <div class="group-label">Local / Ollama ${renderCapBadges(localCaps)}</div>
          <div class="group-desc">On-device AI processing — no API keys needed</div>
        </div>
      </div>
      ${ollamaModelsHtml ? `<div id="ollama-provider-container"><div class="provider-card-wrapper">${ollamaModelsHtml}</div></div>` : '<div id="ollama-provider-container"></div>'}
      <div id="local-llm-models-list" style="display:flex;flex-direction:column;gap:8px;"></div>
      <div id="llm-freshness-actions" style="margin-top:6px;display:flex;align-items:center;gap:8px;">
        <button id="llm-check-freshness-btn" class="mini-action-btn" style="font-size:0.75rem;">Check for Updates</button>
        <span id="llm-freshness-status" style="font-size:0.7rem;color:var(--text-secondary);"></span>
      </div>
      <div style="margin-top:4px;">
        <p style="font-size:0.75rem;color:var(--text-secondary);opacity:0.7;margin:0;">
          Location: <span class="mono-font">~/.nbp/models/llm/</span>
        </p>
      </div>
    </div>
  `);

  el.innerHTML = sections.join('');

  // Wire per-provider Refresh buttons
  el.querySelectorAll('.refresh-provider-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const providerId = btn.dataset.provider;
      btn.disabled = true;
      btn.textContent = 'Refreshing...';
      try {
        await fetchProviderModels(providerId);
        renderModelsProviders();
      } finally {
        btn.disabled = false;
        btn.textContent = 'Refresh LLMs';
      }
    });
  });

  // Wire Save buttons
  el.querySelectorAll('.provider-save-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const providerId = btn.dataset.provider;
      const input = document.getElementById(`settings-api-key-${providerId}`);
      if (!input) return;

      const value = input.value.trim();
      if (isKeyMasked(value)) return;

      if (!appSettings.transcription) appSettings.transcription = {};
      if (!appSettings.transcription.api_keys) appSettings.transcription.api_keys = {};
      appSettings.transcription.api_keys[providerId] = value || null;
      if (!appSettings.providers) appSettings.providers = {};
      if (!appSettings.providers[providerId]) appSettings.providers[providerId] = {};
      appSettings.providers[providerId].api_key = value || null;

      if (!value) {
        delete providerModels[providerId];
      }

      btn.disabled = true;
      btn.textContent = '...';
      try {
        await window.__TAURI__.core.invoke('save_settings', { settings: appSettings });
        if (typeof updateTranscriptionKeyStatusDot === 'function') updateTranscriptionKeyStatusDot();

        if (value) {
          updateProviderKeyStatus(providerId, 'checking');
          btn.textContent = 'Checking...';
          const valid = await validateApiKey(providerId, value);
          if (!window.__nbpValidatedKeys) window.__nbpValidatedKeys = {};
          if (!window.__nbpFailedKeys) window.__nbpFailedKeys = {};
          if (valid) {
            window.__nbpValidatedKeys[providerId] = value;
            delete window.__nbpFailedKeys[providerId];
            fetchProviderModels(providerId).then(() => renderModelsProviders());
          } else {
            delete window.__nbpValidatedKeys[providerId];
            window.__nbpFailedKeys[providerId] = value;
            delete providerModels[providerId];
            showToast(`${providerId} key verification failed`, 'error');
          }
        }
        renderModelsProviders();
      } catch (err) {
        showToast('Failed to save: ' + err, 'error');
      } finally {
        btn.disabled = false;
        btn.textContent = 'Save';
      }
    });
  });

  // Wire Show All / Show Less toggle buttons
  el.querySelectorAll('.provider-models-toggle').forEach(btn => {
    btn.addEventListener('click', () => {
      const providerId = btn.dataset.provider;
      providerModelsExpanded[providerId] = !providerModelsExpanded[providerId];
      renderModelsProviders();
    });
  });

  // Wire freshness check button
  const freshnessBtn = el.querySelector('#llm-check-freshness-btn');
  if (freshnessBtn) {
    freshnessBtn.addEventListener('click', async () => {
      const invoke = window.__TAURI__.core.invoke;
      const statusEl = document.getElementById('llm-freshness-status');
      freshnessBtn.disabled = true;
      freshnessBtn.innerHTML = '<span class="btn-spinner"></span> Checking…';
      freshnessCheckRunning = true;
      if (statusEl) statusEl.textContent = '';
      try {
        const report = await invoke('check_all_llm_freshness');
        llmFreshnessData = report.models || {};
        const updateCount = Object.values(llmFreshnessData).filter(v => v.status === 'update_available').length;
        if (report.failed > 0 && report.checked === 0) {
          showToast('Could not check for updates — network error', 'error');
          if (statusEl) statusEl.textContent = `${report.failed} LLM${report.failed !== 1 ? 's' : ''} could not be checked`;
        } else if (report.failed > 0) {
          const msg = updateCount > 0
            ? `${updateCount} update(s) available, ${report.failed} could not be checked`
            : `${report.checked} checked, ${report.failed} could not be checked`;
          if (statusEl) statusEl.textContent = msg;
        } else if (updateCount > 0) {
          if (statusEl) statusEl.textContent = `${updateCount} update(s) available`;
        } else {
          showToast('All LLMs are up to date', 'success');
          if (statusEl) statusEl.textContent = 'All up to date';
        }
        renderLocalLlmModelsInner();
      } catch (err) {
        if (String(err).includes('cancelled')) {
          if (statusEl) statusEl.textContent = '';
        } else {
          showToast('Freshness check failed: ' + err, 'error');
        }
      } finally {
        freshnessCheckRunning = false;
        freshnessBtn.disabled = false;
        freshnessBtn.textContent = 'Check for Updates';
      }
    });
  }

  // Fill local LLM models
  renderLocalLlmModelsInner();
}

// Backward-compat aliases
function renderProcessingProviders() { renderModelsProviders(); }

// ===== LOCAL LLM MODELS =====
var llmModelsData = [];
var llmFreshnessData = {};

async function renderLocalLlmModels() {
  const invoke = window.__TAURI__.core.invoke;
  try {
    llmModelsData = await invoke('get_llm_models_info');
  } catch (err) {
    console.error('Failed to load LLM models:', err);
    llmModelsData = [];
  }
  // Load cached freshness results from last auto-check (full snapshot replaces existing data)
  try {
    const cached = await invoke('get_cached_freshness_results');
    if (cached && typeof cached === 'object') {
      llmFreshnessData = {};
      for (const [modelId, hasUpdate] of Object.entries(cached)) {
        llmFreshnessData[modelId] = { status: hasUpdate ? 'update_available' : 'up_to_date' };
      }
    }
  } catch (_) { /* ignore — cached results are optional */ }
  renderLocalLlmModelsInner();
}

function renderLocalLlmModelsInner() {
  const el = document.getElementById('local-llm-models-list');
  if (!el) return;

  const invoke = window.__TAURI__.core.invoke;
  const selectedId = appSettings?.local_llm?.model_id || null;

  el.innerHTML = llmModelsData.map(m => {
    const isSelected = m.id === selectedId;
    const sizeStr = m.size_mb >= 1000 ? `${(m.size_mb / 1000).toFixed(1)} GB` : `${m.size_mb} MB`;
    const freshness = llmFreshnessData[m.id];
    const hasUpdate = freshness?.status === 'update_available';
    const statusBadge = m.downloaded
      ? `<span class="llm-status-badge llm-status-downloaded">Downloaded</span>`
      : `<span class="llm-status-badge llm-status-not-downloaded">Not downloaded</span>`;

    return `
      <div class="provider-card${isSelected ? ' llm-selected' : ''}" data-llm-id="${escapeHtml(m.id)}" style="cursor:pointer;flex-wrap:wrap;">
        <div class="provider-card-icon" style="background:rgba(139,92,246,0.15);display:flex;align-items:center;justify-content:center;font-size:14px;font-weight:700;color:var(--accent-color);">
          ${escapeHtml(m.params)}
        </div>
        <div class="provider-card-info" style="flex:1;">
          <div class="provider-card-name">
            ${escapeHtml(m.name)}
            ${statusBadge}
            ${isSelected ? '<span class="llm-status-badge llm-status-active">Active</span>' : ''}
            ${hasUpdate ? '<span class="llm-status-badge llm-status-update">Update available</span>' : ''}
          </div>
          <div class="provider-card-detail">${escapeHtml(m.desc)}</div>
          <div class="provider-card-detail" style="opacity:0.6;font-size:0.65rem;">${sizeStr} • Q4_K_M</div>
        </div>
        <div class="provider-card-input" style="gap:6px;">
          ${m.downloaded
            ? `<button class="mini-action-btn llm-select-btn${isSelected ? ' active' : ''}" data-llm-id="${escapeHtml(m.id)}" style="font-size:0.75rem;">${isSelected ? 'Active' : 'Select'}</button>
               <button class="mini-action-btn llm-update-btn${hasUpdate ? ' update-available' : ''}" data-llm-id="${escapeHtml(m.id)}" title="${hasUpdate ? 'Update available — download latest version' : 'Re-download latest version'}" style="width:34px;height:34px;padding:0;display:flex;align-items:center;justify-content:center;">
                 <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
               </button>
               <button class="mini-action-btn llm-delete-btn" data-llm-id="${escapeHtml(m.id)}" title="Delete LLM" style="width:34px;height:34px;padding:0;display:flex;align-items:center;justify-content:center;">
                 <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="m19 6-.867 12.14A2 2 0 0 1 16.138 20H7.862a2 2 0 0 1-1.995-1.86L5 6m5 0V4a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v2"/></svg>
               </button>`
            : `<button class="mini-action-btn llm-download-btn" data-llm-id="${escapeHtml(m.id)}" style="font-size:0.75rem;">Download</button>`
          }
        </div>
        <div id="llm-progress-${escapeHtml(m.id)}" class="llm-progress-region" style="display:none;flex-basis:100%;padding:8px 0 0;">
          <div style="height:4px;background:var(--border-color);border-radius:2px;overflow:hidden;">
            <div class="llm-progress-fill" style="height:100%;width:0%;background:var(--accent-color);transition:width 0.2s;border-radius:2px;"></div>
          </div>
          <div class="llm-progress-text" style="font-size:0.7rem;color:var(--text-secondary);margin-top:4px;"></div>
        </div>
      </div>
    `;
  }).join('');

  // Wire download buttons
  el.querySelectorAll('.llm-download-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const modelId = btn.dataset.llmId;
      btn.style.display = 'none';

      const progressEl = document.getElementById(`llm-progress-${modelId}`);
      if (progressEl) progressEl.style.display = 'block';

      try {
        await invoke('download_llm_model', { modelId });
        await renderLocalLlmModels();
      } catch (err) {
        showToast('Download failed: ' + err, 'error');
        btn.style.display = '';
        if (progressEl) progressEl.style.display = 'none';
      }
    });
  });

  // Wire select buttons
  el.querySelectorAll('.llm-select-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const modelId = btn.dataset.llmId;
      if (!appSettings.local_llm) appSettings.local_llm = {};
      appSettings.local_llm.model_id = modelId;
      appSettings.local_llm.enabled = true;
      await invoke('save_settings', { settings: appSettings });
      await renderLocalLlmModels();
    });
  });

  // Wire update buttons (delete + re-download)
  el.querySelectorAll('.llm-update-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const modelId = btn.dataset.llmId;
      const model = llmModelsData.find(m => m.id === modelId);
      const hasUpdate = llmFreshnessData[modelId]?.status === 'update_available';
      const title = hasUpdate ? 'Update LLM?' : 'Re-download LLM?';
      const msg = hasUpdate
        ? `A newer version of ${model?.name || modelId} is available. Download it now?`
        : `Re-download ${model?.name || modelId}? This will replace the current file.`;
      const ok = await showConfirm(title, msg);
      if (!ok) return;

      const card = btn.closest('.provider-card');
      if (card) card.querySelectorAll('.provider-card-input button').forEach(b => { b.style.display = 'none'; });
      const progressEl = document.getElementById(`llm-progress-${modelId}`);
      if (progressEl) progressEl.style.display = 'block';

      try {
        await invoke('delete_llm_model', { modelId });
        delete llmFreshnessData[modelId];
        await invoke('download_llm_model', { modelId });
        await renderLocalLlmModels();
      } catch (err) {
        showToast('Update failed: ' + err, 'error');
        if (progressEl) progressEl.style.display = 'none';
        await renderLocalLlmModels();
      }
    });
  });

  // Wire delete buttons
  el.querySelectorAll('.llm-delete-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const modelId = btn.dataset.llmId;
      const model = llmModelsData.find(m => m.id === modelId);
      const ok2 = await showConfirm('Delete LLM?', `Delete ${model?.name || modelId}? The LLM file will be removed.`);
      if (!ok2) return;
      try {
        await invoke('delete_llm_model', { modelId });
        delete llmFreshnessData[modelId];
        appSettings = await invoke('load_settings');
        await renderLocalLlmModels();
      } catch (err) {
        showToast('Delete failed: ' + err, 'error');
      }
    });
  });
}

// Listen for LLM download progress events
if (window.__TAURI__?.event?.listen) {
  window.__TAURI__.event.listen('llm_download_progress', (event) => {
    const { model_id, downloaded, total, percent } = event.payload;
    const progressEl = document.getElementById(`llm-progress-${model_id}`);
    if (!progressEl) return;
    const fill = progressEl.querySelector('.llm-progress-fill');
    const text = progressEl.querySelector('.llm-progress-text');
    if (fill) fill.style.width = `${percent.toFixed(1)}%`;
    const dlMB = (downloaded / 1024 / 1024).toFixed(0);
    const totalMB = (total / 1024 / 1024).toFixed(0);
    if (text) text.textContent = `${dlMB} / ${totalMB} MB (${percent.toFixed(1)}%)`;
  });
}

// Listen for LLM freshness check progress events
if (window.__TAURI__?.event?.listen) {
  window.__TAURI__.event.listen('llm_freshness_progress', (event) => {
    const { model_name, current, total } = event.payload;
    const statusEl = document.getElementById('llm-freshness-status');
    if (statusEl) statusEl.textContent = `Checking ${model_name} (${current}/${total})…`;
  });

  // Listen for auto-check results from app launch freshness check (full snapshot replacement)
  window.__TAURI__.event.listen('model_freshness_auto_result', (event) => {
    const results = event.payload;
    if (!Array.isArray(results)) return;
    llmFreshnessData = {};
    for (const info of results) {
      llmFreshnessData[info.model_id] = { status: info.update_available ? 'update_available' : 'up_to_date' };
    }
    renderLocalLlmModelsInner();
  });
}

// Wire "Check for Updates" button with spinner and cancellation
let freshnessCheckRunning = false;

function cancelFreshnessCheck() {
  if (!freshnessCheckRunning) return;
  window.__TAURI__.core.invoke('cancel_llm_freshness');
}

// Cancel freshness check when navigating away from settings
new MutationObserver(() => {
  if (!document.body.classList.contains('settings-open')) {
    cancelFreshnessCheck();
  }
}).observe(document.body, { attributes: true, attributeFilter: ['class'] });

// Freshness check button wired inside renderModelsProviders()

async function loadNotionProfiles() {
  try {
    notionProfiles = await window.__TAURI__.core.invoke('list_notion_profiles');
  } catch (err) {
    console.error('Failed to load Notion profiles:', err);
    notionProfiles = [];
  }
}

async function loadLinearProfiles() {
  try {
    linearProfiles = await window.__TAURI__.core.invoke('list_linear_profiles');
  } catch (err) {
    console.error('Failed to load Linear profiles:', err);
    linearProfiles = [];
  }
}

async function loadSlackForIntegrations() {
  if (typeof loadSlackIntegrations === 'function') {
    await loadSlackIntegrations();
  }
}

async function loadSavePathIntegrations() {
  try {
    savePathIntegrations = await window.__TAURI__.core.invoke('list_save_path_integrations');
  } catch (err) {
    console.error('Failed to load save path integrations:', err);
    savePathIntegrations = [];
  }
}

async function loadWebhookProfiles() {
  try {
    webhookProfiles = await window.__TAURI__.core.invoke('list_webhook_profiles');
  } catch (err) {
    console.error('Failed to load webhook profiles:', err);
    webhookProfiles = [];
  }
}

// ===== RENDER CONNECTED =====
function renderConnectedIntegrations() {
  const el = connectedListEl();
  if (!el) return;

  const cards = [];

  // Notion cards
  for (const profile of notionProfiles) {
    const safeName = escapeHtml(profile.name);
    const safeDb = escapeHtml(profile.database_name || 'No database selected');
    const syncedAt = profile.synced_at
      ? new Date(profile.synced_at).toLocaleDateString()
      : 'Never synced';
    const daysSinceSync = profile.synced_at
      ? Math.floor((Date.now() - new Date(profile.synced_at).getTime()) / (1000 * 60 * 60 * 24))
      : null;
    const isStale = !profile.synced_at || daysSinceSync > 7;
    let cardDetail;
    if (!profile.synced_at) {
      cardDetail = `${safeDb} · <span style="color: #e6a700; font-weight: 500;">Never synced — sync schema in wizard</span>`;
    } else if (isStale) {
      cardDetail = `${safeDb} · Synced ${syncedAt} · <span style="color: #e6a700; font-weight: 500;">Schema may be outdated — re-sync recommended</span>`;
    } else {
      cardDetail = `${safeDb} · Synced ${syncedAt}`;
    }
    cards.push(`
      <div class="integration-card" data-type="notion" data-id="${escapeHtml(profile.id)}">
        <div class="integration-card-icon-wrapper">
          ${profile.icon_url
            ? `<img class="integration-card-icon notion" src="${escapeHtml(profile.icon_url)}" alt="N" style="object-fit: cover;" />`
            : `<div class="integration-card-icon notion">${INT_NOTION_SVG}</div>`}
          <span class="integration-card-icon-label">Notion</span>
        </div>
        <div class="integration-card-info">
          <div class="integration-card-name">${safeName}</div>
          <div class="integration-card-detail">${cardDetail}</div>
        </div>
        <div class="integration-card-actions">
          <button class="mini-action-btn test-notion-btn" data-id="${escapeHtml(profile.id)}">Test</button>
          <button class="mini-action-btn danger remove-notion-btn" data-id="${escapeHtml(profile.id)}">Remove</button>
        </div>
      </div>
    `);
  }

  // Linear cards
  for (const profile of linearProfiles) {
    const safeName = escapeHtml(profile.name);
    const safeTeam = escapeHtml(profile.team_name || 'No team selected');
    const syncedAt = profile.synced_at
      ? new Date(profile.synced_at).toLocaleDateString()
      : 'Never synced';
    const daysSinceSync = profile.synced_at
      ? Math.floor((Date.now() - new Date(profile.synced_at).getTime()) / (1000 * 60 * 60 * 24))
      : null;
    const isStale = !profile.synced_at || daysSinceSync > 7;
    let cardDetail;
    if (!profile.synced_at) {
      cardDetail = `${safeTeam} · <span style="color: #e6a700; font-weight: 500;">Never synced — sync schema in wizard</span>`;
    } else if (isStale) {
      cardDetail = `${safeTeam} · Synced ${syncedAt} · <span style="color: #e6a700; font-weight: 500;">Schema may be outdated — re-sync recommended</span>`;
    } else {
      cardDetail = `${safeTeam} · Synced ${syncedAt}`;
    }
    cards.push(`
      <div class="integration-card" data-type="linear" data-id="${escapeHtml(profile.id)}">
        <div class="integration-card-icon-wrapper">
          <div class="integration-card-icon linear">${INT_LINEAR_SVG}</div>
          <span class="integration-card-icon-label">Linear</span>
        </div>
        <div class="integration-card-info">
          <div class="integration-card-name">${safeName}</div>
          <div class="integration-card-detail">${cardDetail}</div>
        </div>
        <div class="integration-card-actions">
          <button class="mini-action-btn test-linear-btn" data-id="${escapeHtml(profile.id)}">Test</button>
          <button class="mini-action-btn resync-linear-btn" data-id="${escapeHtml(profile.id)}">Re-sync</button>
          <button class="mini-action-btn danger remove-linear-btn" data-id="${escapeHtml(profile.id)}">Remove</button>
        </div>
      </div>
    `);
  }

  // Slack cards
  for (const [id, data] of Object.entries(slackIntegrations)) {
    const safeName = escapeHtml(data.name);
    const safeWorkspace = escapeHtml(data.workspace_name || 'Unknown workspace');
    cards.push(`
      <div class="integration-card" data-type="slack" data-id="${escapeHtml(id)}">
        <div class="integration-card-icon-wrapper">
          ${data.icon_url
            ? `<img class="integration-card-icon slack" src="${escapeHtml(data.icon_url)}" alt="S" style="object-fit: cover;" />`
            : `<div class="integration-card-icon slack">${INT_SLACK_SVG}</div>`}
          <span class="integration-card-icon-label">Slack</span>
        </div>
        <div class="integration-card-info">
          <div class="integration-card-name">${safeName}</div>
          <div class="integration-card-detail">${safeWorkspace}</div>
        </div>
        <div class="integration-card-actions">
          <button class="mini-action-btn test-slack-int-btn" data-id="${escapeHtml(id)}">Test</button>
          <button class="mini-action-btn danger remove-slack-int-btn" data-id="${escapeHtml(id)}">Remove</button>
        </div>
      </div>
    `);
  }

  // Save path cards
  for (const sp of savePathIntegrations) {
    const safeName = escapeHtml(sp.name);
    const safePath = escapeHtml(sp.path);
    cards.push(`
      <div class="integration-card" data-type="save-path" data-id="${escapeHtml(sp.id)}">
        <div class="integration-card-icon-wrapper">
          <div class="integration-card-icon save-path">${INT_FOLDER_SVG}</div>
          <span class="integration-card-icon-label">Folder</span>
        </div>
        <div class="integration-card-info">
          <div class="integration-card-name">${safeName}</div>
          <div class="integration-card-detail">${safePath}</div>
        </div>
        <div class="integration-card-actions">
          <button class="mini-action-btn edit-save-path-btn" data-id="${escapeHtml(sp.id)}">Edit</button>
          <button class="mini-action-btn danger remove-save-path-btn" data-id="${escapeHtml(sp.id)}">Remove</button>
        </div>
      </div>
    `);
  }

  // Webhook cards
  for (const wh of webhookProfiles) {
    const safeName = escapeHtml(wh.name);
    const safeUrl = escapeHtml(wh.url);
    const safeMethod = escapeHtml(wh.method || 'POST');
    cards.push(`
      <div class="integration-card" data-type="webhook" data-id="${escapeHtml(wh.id)}">
        <div class="integration-card-icon-wrapper">
          <div class="integration-card-icon webhook">${INT_LINK_SVG}</div>
          <span class="integration-card-icon-label">Webhook</span>
        </div>
        <div class="integration-card-info">
          <div class="integration-card-name">${safeName}</div>
          <div class="integration-card-detail">${safeMethod} ${safeUrl}</div>
        </div>
        <div class="integration-card-actions">
          <button class="mini-action-btn test-webhook-btn" data-id="${escapeHtml(wh.id)}">Test</button>
          <button class="mini-action-btn danger remove-webhook-btn" data-id="${escapeHtml(wh.id)}">Remove</button>
        </div>
      </div>
    `);
  }

  if (cards.length === 0) {
    el.innerHTML = '<div style="text-align: center; padding: 40px 0; color: var(--text-secondary); opacity: 0.6;">No integrations connected yet</div>';
  } else {
    el.innerHTML = cards.join('');
  }

  // Attach Notion handlers
  el.querySelectorAll('.test-notion-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      btn.disabled = true;
      btn.textContent = 'Testing...';
      try {
        const result = await window.__TAURI__.core.invoke('test_notion_integration', { integrationId: id });
        showToast('Notion: ' + result, 'success');
      } catch (err) {
        showToast('Notion test failed: ' + err, 'error');
      } finally {
        btn.disabled = false;
        btn.textContent = 'Test';
      }
    });
  });

  el.querySelectorAll('.remove-notion-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      const profile = notionProfiles.find(p => p.id === id);
      const ok = await showConfirm('Remove Integration?', `Remove Notion integration "${profile ? profile.name : id}"?`);
      if (!ok) return;
      try {
        await window.__TAURI__.core.invoke('remove_notion_integration', { integrationId: id });
        await loadAllIntegrations();
      } catch (err) {
        showToast('Failed to remove: ' + err, 'error');
      }
    });
  });

  // Attach Linear handlers
  el.querySelectorAll('.test-linear-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      btn.disabled = true;
      btn.textContent = 'Testing...';
      try {
        const result = await window.__TAURI__.core.invoke('test_linear_integration', { integrationId: id });
        showToast('Linear: ' + result, 'success');
      } catch (err) {
        showToast('Linear test failed: ' + err, 'error');
      } finally {
        btn.disabled = false;
        btn.textContent = 'Test';
      }
    });
  });

  el.querySelectorAll('.remove-linear-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      const profile = linearProfiles.find(p => p.id === id);
      const ok = await showConfirm('Remove Integration?', `Remove Linear integration "${profile ? profile.name : id}"?`);
      if (!ok) return;
      try {
        await window.__TAURI__.core.invoke('remove_linear_integration', { integrationId: id });
        await loadAllIntegrations();
      } catch (err) {
        showToast('Failed to remove: ' + err, 'error');
      }
    });
  });

  el.querySelectorAll('.resync-linear-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      const profile = linearProfiles.find(p => p.id === id);
      if (!profile || !profile.team_id) {
        showToast('No team synced. Open the wizard to set up.', 'error');
        return;
      }

      btn.disabled = true;
      btn.textContent = 'Syncing...';
      try {
        const updatedProfile = await window.__TAURI__.core.invoke('sync_linear_schema', {
          integrationId: id,
          teamId: profile.team_id,
          teamName: profile.team_name,
        });
        // Update global
        const idx = linearProfiles.findIndex(p => p.id === id);
        if (idx >= 0) linearProfiles[idx] = updatedProfile;
        // Re-render to show updated sync timestamp
        renderConnectedIntegrations();
      } catch (err) {
        showToast('Re-sync failed: ' + err, 'error');
        btn.disabled = false;
        btn.textContent = 'Re-sync';
      }
    });
  });

  // Attach Slack handlers
  el.querySelectorAll('.test-slack-int-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      btn.disabled = true;
      btn.textContent = 'Testing...';
      try {
        const workspaceName = await window.__TAURI__.core.invoke('test_slack_integration', { id });
        showToast('Connected to: ' + workspaceName, 'success');
      } catch (err) {
        showToast('Connection failed: ' + err, 'error');
      } finally {
        btn.disabled = false;
        btn.textContent = 'Test';
      }
    });
  });

  el.querySelectorAll('.remove-slack-int-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      const data = slackIntegrations[id];
      const ok = await showConfirm('Remove Integration?', `Remove Slack workspace "${data ? data.name : id}"?`);
      if (!ok) return;
      try {
        await window.__TAURI__.core.invoke('remove_slack_integration', { id });
        await loadAllIntegrations();
      } catch (err) {
        showToast('Failed to remove: ' + err, 'error');
      }
    });
  });

  // Attach Save Path handlers
  el.querySelectorAll('.edit-save-path-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      const sp = savePathIntegrations.find(p => p.id === id);
      if (!sp) return;
      // Replace the card with an inline editor
      const card = btn.closest('.integration-card');
      if (!card) return;
      const safeId = escapeHtml(sp.id);
      card.outerHTML = `
        <div class="integration-card save-path-editor" data-id="${safeId}">
          <div class="integration-card-icon-wrapper">
            <div class="integration-card-icon save-path">${INT_FOLDER_SVG}</div>
            <span class="integration-card-icon-label">Folder</span>
          </div>
          <div class="integration-card-info" style="flex: 1; gap: 6px; display: flex; flex-direction: column;">
            <input id="edit-sp-name-${safeId}" type="text" value="${escapeHtml(sp.name)}" placeholder="Folder name" style="width: 100%;" />
            <div style="display: flex; align-items: center; gap: 8px;">
              <span id="edit-sp-path-display-${safeId}" style="flex: 1; font-size: 0.8rem; color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${escapeHtml(sp.path)}</span>
              <button id="edit-sp-browse-${safeId}" class="mini-action-btn">Browse</button>
            </div>
          </div>
          <div class="integration-card-actions">
            <button class="mini-action-btn save-sp-edit-btn" data-id="${safeId}">Save</button>
            <button class="mini-action-btn cancel-sp-edit-btn" data-id="${safeId}">Cancel</button>
          </div>
        </div>
      `;
      // Store the current path in a closure variable
      let selectedPath = sp.path;
      const pathDisplay = document.getElementById(`edit-sp-path-display-${sp.id}`);
      const browseBtn = document.getElementById(`edit-sp-browse-${sp.id}`);
      if (browseBtn) {
        browseBtn.addEventListener('click', async () => {
          try {
            const result = await window.__TAURI__.dialog.open({ directory: true, multiple: false });
            if (result) {
              selectedPath = result;
              if (pathDisplay) pathDisplay.textContent = result;
            }
          } catch (err) {
            console.error('Folder picker error:', err);
          }
        });
      }
      const saveBtn = el.querySelector(`.save-sp-edit-btn[data-id="${sp.id}"]`);
      if (saveBtn) {
        saveBtn.addEventListener('click', async () => {
          const nameInput = document.getElementById(`edit-sp-name-${sp.id}`);
          const name = nameInput ? nameInput.value.trim() : '';
          if (!name) { showToast('Name cannot be empty', 'error'); return; }
          if (!selectedPath) { showToast('Please select a folder', 'error'); return; }
          saveBtn.disabled = true;
          try {
            await window.__TAURI__.core.invoke('update_save_path_integration', { id: sp.id, name, path: selectedPath });
            await loadAllIntegrations();
          } catch (err) {
            showToast('Failed to update: ' + err, 'error');
            saveBtn.disabled = false;
          }
        });
      }
      const cancelBtn = el.querySelector(`.cancel-sp-edit-btn[data-id="${sp.id}"]`);
      if (cancelBtn) {
        cancelBtn.addEventListener('click', () => {
          renderConnectedIntegrations();
        });
      }
    });
  });

  el.querySelectorAll('.remove-save-path-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      const sp = savePathIntegrations.find(p => p.id === id);
      const ok = await showConfirm('Remove Save Path?', `Remove save path "${sp ? sp.name : id}"?`);
      if (!ok) return;
      try {
        await window.__TAURI__.core.invoke('remove_save_path_integration', { id });
        await loadAllIntegrations();
      } catch (err) {
        showToast('Failed to remove: ' + err, 'error');
      }
    });
  });

  // Attach Webhook handlers
  el.querySelectorAll('.test-webhook-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      btn.disabled = true;
      btn.textContent = 'Testing...';
      try {
        const result = await window.__TAURI__.core.invoke('test_webhook_integration', { id });
        showToast('Webhook: ' + result, 'success');
      } catch (err) {
        showToast('Webhook test failed: ' + err, 'error');
      } finally {
        btn.disabled = false;
        btn.textContent = 'Test';
      }
    });
  });

  el.querySelectorAll('.remove-webhook-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      const wh = webhookProfiles.find(p => p.id === id);
      const ok = await showConfirm('Remove Webhook?', `Remove webhook "${wh ? wh.name : id}"?`);
      if (!ok) return;
      try {
        await window.__TAURI__.core.invoke('remove_webhook_integration', { id });
        await loadAllIntegrations();
      } catch (err) {
        showToast('Failed to remove: ' + err, 'error');
      }
    });
  });
}

// ===== RENDER AVAILABLE =====
function renderAvailableIntegrations() {
  const el = availableListEl();
  if (!el) return;

  const available = [];

  // Notion is always available to add (user can have multiple databases)
  available.push(`
    <div class="available-integration-card" data-type="notion" id="add-notion-integration-btn">
      <div class="integration-card-icon-wrapper">
        <div class="integration-card-icon notion">${INT_NOTION_SVG}</div>
        <span class="integration-card-icon-label">Notion</span>
      </div>
      <div class="integration-card-info">
        <div class="integration-card-name">Notion</div>
        <div class="integration-card-detail">Connect a Notion database for automatic page creation</div>
      </div>
      <span class="available-add-label">+ Add</span>
    </div>
  `);

  // Linear is always available to add (user can have multiple team connections)
  available.push(`
    <div class="available-integration-card" data-type="linear" id="add-linear-integration-btn">
      <div class="integration-card-icon-wrapper">
        <div class="integration-card-icon linear">${INT_LINEAR_SVG}</div>
        <span class="integration-card-icon-label">Linear</span>
      </div>
      <div class="integration-card-info">
        <div class="integration-card-name">Linear</div>
        <div class="integration-card-detail">Connect Linear to create issues from pipeline output</div>
      </div>
      <span class="available-add-label">+ Add</span>
    </div>
  `);

  // Slack is always available to add
  available.push(`
    <div class="available-integration-card" data-type="slack" id="add-slack-integration-btn">
      <div class="integration-card-icon-wrapper">
        <div class="integration-card-icon slack">${INT_SLACK_SVG}</div>
        <span class="integration-card-icon-label">Slack</span>
      </div>
      <div class="integration-card-info">
        <div class="integration-card-name">Slack</div>
        <div class="integration-card-detail">Send pipeline output to Slack channels or DMs</div>
      </div>
      <span class="available-add-label">+ Add</span>
    </div>
  `);

  // Save Path is always available to add (multiple can exist)
  available.push(`
    <div class="available-integration-card" data-type="save-path" id="add-save-path-btn">
      <div class="integration-card-icon-wrapper">
        <div class="integration-card-icon save-path">${INT_FOLDER_SVG}</div>
        <span class="integration-card-icon-label">Folder</span>
      </div>
      <div class="integration-card-info">
        <div class="integration-card-name">Save Path</div>
        <div class="integration-card-detail">Save pipeline output to a named folder location</div>
      </div>
      <span class="available-add-label">+ Add</span>
    </div>
  `);

  // Webhook is always available to add
  available.push(`
    <div class="available-integration-card" data-type="webhook" id="add-webhook-btn">
      <div class="integration-card-icon-wrapper">
        <div class="integration-card-icon webhook">${INT_LINK_SVG}</div>
        <span class="integration-card-icon-label">Webhook</span>
      </div>
      <div class="integration-card-info">
        <div class="integration-card-name">Webhook</div>
        <div class="integration-card-detail">Send pipeline output to any HTTP endpoint</div>
      </div>
      <span class="available-add-label">+ Add</span>
    </div>
  `);

  el.innerHTML = available.join('');

  // Notion add → opens wizard (wired in 04-02, placeholder here)
  const addNotionBtn = document.getElementById('add-notion-integration-btn');
  if (addNotionBtn) {
    addNotionBtn.addEventListener('click', () => {
      if (typeof openNotionWizard === 'function') {
        openNotionWizard();
      } else {
        showToast('Notion setup wizard not yet available', 'info');
      }
    });
  }

  // Linear add → opens wizard
  const addLinearBtn = document.getElementById('add-linear-integration-btn');
  if (addLinearBtn) {
    addLinearBtn.addEventListener('click', () => {
      if (typeof openLinearWizard === 'function') {
        openLinearWizard();
      } else {
        showToast('Linear setup wizard not yet available', 'info');
      }
    });
  }

  // Slack add → reuse existing add-slack-modal from main.js
  const addSlackIntBtn = document.getElementById('add-slack-integration-btn');
  if (addSlackIntBtn) {
    addSlackIntBtn.addEventListener('click', () => {
      const modal = document.getElementById('add-slack-modal');
      const tokenInput = document.getElementById('slack-token-input');
      if (modal) {
        if (tokenInput) tokenInput.value = '';
        modal.style.display = 'flex';
      }
    });
  }

  // Save Path add → inline form replaces the card
  const addSavePathBtn = document.getElementById('add-save-path-btn');
  if (addSavePathBtn) {
    addSavePathBtn.addEventListener('click', () => {
      let selectedPath = '';
      addSavePathBtn.outerHTML = `
        <div class="available-integration-card save-path-add-form" id="add-save-path-form">
          <div class="integration-card-icon-wrapper">
            <div class="integration-card-icon save-path">${INT_FOLDER_SVG}</div>
            <span class="integration-card-icon-label">Folder</span>
          </div>
          <div class="integration-card-info" style="flex: 1; gap: 6px; display: flex; flex-direction: column;">
            <input id="new-sp-name" type="text" placeholder="Folder name (e.g. Notes)" style="width: 100%;" />
            <div style="display: flex; align-items: center; gap: 8px;">
              <span id="new-sp-path-display" style="flex: 1; font-size: 0.8rem; color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">No folder selected</span>
              <button id="new-sp-browse-btn" class="mini-action-btn">Browse</button>
            </div>
          </div>
          <div class="integration-card-actions">
            <button id="new-sp-save-btn" class="mini-action-btn">Save</button>
            <button id="new-sp-cancel-btn" class="mini-action-btn">Cancel</button>
          </div>
        </div>
      `;

      const browseBtn = document.getElementById('new-sp-browse-btn');
      if (browseBtn) {
        browseBtn.addEventListener('click', async () => {
          try {
            const result = await window.__TAURI__.dialog.open({ directory: true, multiple: false });
            if (result) {
              selectedPath = result;
              const display = document.getElementById('new-sp-path-display');
              if (display) display.textContent = result;
            }
          } catch (err) {
            console.error('Folder picker error:', err);
          }
        });
      }

      const saveBtn = document.getElementById('new-sp-save-btn');
      if (saveBtn) {
        saveBtn.addEventListener('click', async () => {
          const nameInput = document.getElementById('new-sp-name');
          const name = nameInput ? nameInput.value.trim() : '';
          if (!name) { showToast('Please enter a name', 'error'); return; }
          if (!selectedPath) { showToast('Please select a folder', 'error'); return; }
          saveBtn.disabled = true;
          try {
            await window.__TAURI__.core.invoke('add_save_path_integration', { name, path: selectedPath });
            await loadAllIntegrations();
          } catch (err) {
            showToast('Failed to add save path: ' + err, 'error');
            saveBtn.disabled = false;
          }
        });
      }

      const cancelBtn = document.getElementById('new-sp-cancel-btn');
      if (cancelBtn) {
        cancelBtn.addEventListener('click', () => {
          renderAvailableIntegrations();
        });
      }
    });
  }

  // Webhook add → inline form
  const addWebhookBtn = document.getElementById('add-webhook-btn');
  if (addWebhookBtn) {
    addWebhookBtn.addEventListener('click', () => {
      addWebhookBtn.outerHTML = `
        <div class="available-integration-card webhook-add-form" id="add-webhook-form">
          <div class="integration-card-icon-wrapper">
            <div class="integration-card-icon webhook">${INT_LINK_SVG}</div>
            <span class="integration-card-icon-label">Webhook</span>
          </div>
          <div class="integration-card-info" style="flex: 1; gap: 6px; display: flex; flex-direction: column;">
            <input id="new-wh-name" type="text" placeholder="Endpoint name (e.g. n8n Meetings)" style="width: 100%;" />
            <input id="new-wh-url" type="text" placeholder="https://hooks.example.com/..." style="width: 100%;" />
            <select id="new-wh-method" style="width: 100px;">
              <option value="POST">POST</option>
              <option value="PUT">PUT</option>
              <option value="PATCH">PATCH</option>
            </select>
          </div>
          <div class="integration-card-actions">
            <button id="new-wh-save-btn" class="mini-action-btn">Save</button>
            <button id="new-wh-cancel-btn" class="mini-action-btn">Cancel</button>
          </div>
        </div>
      `;

      const saveBtn = document.getElementById('new-wh-save-btn');
      if (saveBtn) {
        saveBtn.addEventListener('click', async () => {
          const nameInput = document.getElementById('new-wh-name');
          const urlInput = document.getElementById('new-wh-url');
          const methodSelect = document.getElementById('new-wh-method');
          const name = nameInput ? nameInput.value.trim() : '';
          const url = urlInput ? urlInput.value.trim() : '';
          const method = methodSelect ? methodSelect.value : 'POST';
          if (!name) { showToast('Please enter a name', 'error'); return; }
          if (!url) { showToast('Please enter the webhook URL', 'error'); return; }
          if (!url.startsWith('http://') && !url.startsWith('https://')) {
            showToast('URL must start with http:// or https://', 'error');
            return;
          }
          saveBtn.disabled = true;
          try {
            await window.__TAURI__.core.invoke('add_webhook_integration', { name, url, method });
            await loadAllIntegrations();
          } catch (err) {
            showToast('Failed to add webhook: ' + err, 'error');
            saveBtn.disabled = false;
          }
        });
      }

      const cancelBtn = document.getElementById('new-wh-cancel-btn');
      if (cancelBtn) {
        cancelBtn.addEventListener('click', () => {
          renderAvailableIntegrations();
        });
      }
    });
  }
}

// ===== NOTION WIZARD =====

let notionWizardState = {
  step: 0,            // 0=api-key, 1=share-instruction, 2=db-picker, 3=schema, 4=people-mapping
  integrationId: null,
  databases: [],
  selectedDbId: null,
  selectedDbName: null,
  profile: null,      // Full NotionIntegrationProfile after sync
  mappings: [],       // [{alias, notionUserId, displayName}]
  error: null,
};

function resetNotionWizardState() {
  notionWizardState = {
    step: 0,
    integrationId: null,
    databases: [],
    selectedDbId: null,
    selectedDbName: null,
    profile: null,
    mappings: [],
    error: null,
  };
}

function closeNotionWizard() {
  resetNotionWizardState();
  const modal = document.getElementById('notion-wizard-modal');
  if (modal) modal.style.display = 'none';
}

function openNotionWizard() {
  resetNotionWizardState();
  const modal = document.getElementById('notion-wizard-modal');
  if (!modal) return;
  modal.style.display = 'flex';

  // Wire cancel button once (not on each render)
  const cancelBtn = document.getElementById('notion-wizard-cancel');
  if (cancelBtn) {
    // Remove previous listener to avoid stacking
    cancelBtn.replaceWith(cancelBtn.cloneNode(true));
    const freshCancel = document.getElementById('notion-wizard-cancel');
    freshCancel.addEventListener('click', async () => {
      if (notionWizardState.integrationId) {
        try {
          await window.__TAURI__.core.invoke('remove_notion_integration', {
            integrationId: notionWizardState.integrationId,
          });
        } catch (err) {
          console.error('Failed to clean up partial integration on cancel:', err);
        }
      }
      closeNotionWizard();
    });
  }

  renderWizardStep();
}

// Progress percentages per step
const WIZARD_STEP_PROGRESS = ['20%', '40%', '60%', '80%', '100%'];

async function renderWizardStep() {
  const body = document.getElementById('notion-wizard-body');
  const progressBar = document.getElementById('notion-wizard-progress');
  const nextBtn = document.getElementById('notion-wizard-next');
  if (!body || !nextBtn) return;

  // Update progress bar
  if (progressBar) {
    progressBar.style.width = WIZARD_STEP_PROGRESS[notionWizardState.step] || '20%';
  }

  // Update Next button label
  nextBtn.textContent = notionWizardState.step === 4 ? 'Finish' : 'Next';
  nextBtn.disabled = false;

  // Render step body
  switch (notionWizardState.step) {
    case 0: renderStep0(body, nextBtn); break;
    case 1: renderStep1(body, nextBtn); break;
    case 2: await renderStep2(body, nextBtn); break;
    case 3: renderStep3(body, nextBtn); break;
    case 4: renderStep4(body, nextBtn); break;
  }
}

// Step 0: API Key Entry
function renderStep0(body, nextBtn) {
  body.innerHTML = `
    <div class="wizard-step-title">Enter Notion API Key</div>
    <p class="wizard-step-description">Create an internal integration at notion.so/my-integrations, then paste the API key below.</p>
    <div class="wizard-input-group">
      <div>
        <label for="wizard-notion-apikey">API Key</label>
        <input id="wizard-notion-apikey" type="password" placeholder="ntn_..." autocomplete="off" spellcheck="false"
          style="font-family: 'SF Mono', monospace; font-size: 0.85rem;" />
      </div>
    </div>
    ${notionWizardState.error ? `<div class="wizard-error">${escapeHtml(notionWizardState.error)}</div>` : ''}
  `;

  // Remove previous next handler and attach fresh one
  const freshNext = replaceNextBtn();
  freshNext.addEventListener('click', async () => {
    const apiKey = (document.getElementById('wizard-notion-apikey').value || '').trim();
    if (!apiKey) {
      notionWizardState.error = 'Please enter an API key.';
      renderWizardStep();
      return;
    }
    freshNext.disabled = true;
    freshNext.textContent = '...';
    try {
      const result = await window.__TAURI__.core.invoke('add_notion_integration', { apiKey });
      notionWizardState.integrationId = result.id || result;
      notionWizardState.error = null;
      notionWizardState.step = 1;
      renderWizardStep();
    } catch (err) {
      notionWizardState.error = String(err);
      freshNext.disabled = false;
      freshNext.textContent = 'Next';
      renderWizardStep();
    }
  });
}

// Step 1: Share Instruction (mandatory)
function renderStep1(body, nextBtn) {
  body.innerHTML = `
    <div class="wizard-step-title">Share Your Database</div>
    <div class="wizard-info-box">
      <strong>Before selecting a database, you must share it with your integration:</strong>
      <ol>
        <li>Open your Notion database in the browser</li>
        <li>Click the "..." menu in the top-right corner</li>
        <li>Go to "Connections" (or "Add connections")</li>
        <li>Find and add your integration by name</li>
      </ol>
    </div>
    <p class="wizard-step-description" style="margin-top: 12px;">After sharing, click Next to continue.</p>
  `;

  const freshNext = replaceNextBtn();
  freshNext.textContent = 'Next';
  freshNext.addEventListener('click', () => {
    notionWizardState.step = 2;
    renderWizardStep();
  });
}

// Step 2: Database Picker
async function renderStep2(body, nextBtn) {
  body.innerHTML = `<div style="color: var(--text-secondary); font-size: 0.85rem;">Loading databases...</div>`;
  nextBtn.disabled = true;

  try {
    const databases = await window.__TAURI__.core.invoke('list_notion_databases', {
      integrationId: notionWizardState.integrationId,
    });
    notionWizardState.databases = databases;
    notionWizardState.error = null;
    renderStep2Databases(body, nextBtn);
  } catch (err) {
    renderStep2Error(body, nextBtn, String(err));
  }
}

function renderStep2Databases(body, nextBtn) {
  const { databases, selectedDbId } = notionWizardState;

  if (!databases || databases.length === 0) {
    renderStep2Error(body, nextBtn, 'No databases found. Make sure you shared your database with the integration (see previous step).');
    return;
  }

  const items = databases.map(db => {
    const isSelected = db.id === selectedDbId;
    return `<div class="wizard-db-item${isSelected ? ' selected' : ''}" data-db-id="${escapeHtml(db.id)}" data-db-name="${escapeHtml(db.name)}">${escapeHtml(db.name)}</div>`;
  }).join('');

  body.innerHTML = `
    <div class="wizard-step-title">Select Database</div>
    <div class="wizard-db-list">${items}</div>
    ${notionWizardState.error ? `<div class="wizard-error">${escapeHtml(notionWizardState.error)}</div>` : ''}
  `;

  body.querySelectorAll('.wizard-db-item').forEach(item => {
    item.addEventListener('click', () => {
      body.querySelectorAll('.wizard-db-item').forEach(i => i.classList.remove('selected'));
      item.classList.add('selected');
      notionWizardState.selectedDbId = item.dataset.dbId;
      notionWizardState.selectedDbName = item.dataset.dbName;
      freshNext.disabled = false;
    });
  });

  const freshNext = replaceNextBtn();
  freshNext.disabled = !selectedDbId;
  freshNext.textContent = 'Next';
  freshNext.addEventListener('click', async () => {
    if (!notionWizardState.selectedDbId) return;
    freshNext.disabled = true;
    freshNext.textContent = '...';
    try {
      const profile = await window.__TAURI__.core.invoke('sync_notion_schema', {
        integrationId: notionWizardState.integrationId,
        databaseId: notionWizardState.selectedDbId,
        databaseName: notionWizardState.selectedDbName,
      });
      notionWizardState.profile = profile;
      notionWizardState.error = null;
      notionWizardState.step = 3;
      renderWizardStep();
    } catch (err) {
      notionWizardState.error = String(err);
      freshNext.disabled = false;
      freshNext.textContent = 'Next';
      // Re-render databases with error shown
      renderStep2Databases(body, freshNext);
    }
  });
}

function renderStep2Error(body, nextBtn, errorMsg) {
  body.innerHTML = `
    <div class="wizard-step-title">Select Database</div>
    <div class="wizard-info-box">
      <strong>No databases found. Please share your database first:</strong>
      <ol>
        <li>Open your Notion database in the browser</li>
        <li>Click the "..." menu in the top-right corner</li>
        <li>Go to "Connections" (or "Add connections")</li>
        <li>Find and add your integration by name</li>
      </ol>
    </div>
    <div class="wizard-error" style="margin-top: 8px;">${escapeHtml(errorMsg)}</div>
    <button id="wizard-retry-btn" class="mini-action-btn" style="margin-top: 12px;">Retry</button>
  `;

  const retryBtn = document.getElementById('wizard-retry-btn');
  if (retryBtn) {
    retryBtn.addEventListener('click', async () => {
      await renderStep2(body, nextBtn);
    });
  }

  nextBtn.disabled = true;
}

// Step 3: Schema Display
function renderStep3(body, nextBtn) {
  const profile = notionWizardState.profile;
  if (!profile) {
    body.innerHTML = '<div class="wizard-error">No schema loaded.</div>';
    return;
  }

  const properties = profile.properties || [];
  const syncedAt = profile.synced_at
    ? new Date(profile.synced_at).toLocaleString()
    : 'Unknown';

  const rows = properties.map(prop => {
    const options = (prop.type === 'select' || prop.type === 'multi_select')
      ? escapeHtml((prop.select_options || []).join(', ') || '—')
      : '—';
    return `<tr>
      <td>${escapeHtml(prop.name)}</td>
      <td>${escapeHtml(prop.type)}</td>
      <td>${options}</td>
    </tr>`;
  }).join('');

  body.innerHTML = `
    <div class="wizard-step-title">Database Schema</div>
    <div style="max-height: 260px; overflow-y: auto;">
      <table class="wizard-schema-table">
        <thead>
          <tr>
            <th>Property Name</th>
            <th>Type</th>
            <th>Options</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    </div>
    <div class="wizard-schema-synced">Last synced: ${escapeHtml(syncedAt)}</div>
    <button id="wizard-resync-btn" class="mini-action-btn" style="margin-top: 10px;">Re-sync Schema</button>
    ${notionWizardState.error ? `<div class="wizard-error">${escapeHtml(notionWizardState.error)}</div>` : ''}
  `;

  const resyncBtn = document.getElementById('wizard-resync-btn');
  if (resyncBtn) {
    resyncBtn.addEventListener('click', async () => {
      resyncBtn.disabled = true;
      resyncBtn.textContent = '...';
      try {
        const profile = await window.__TAURI__.core.invoke('sync_notion_schema', {
          integrationId: notionWizardState.integrationId,
          databaseId: notionWizardState.selectedDbId,
          databaseName: notionWizardState.selectedDbName,
        });
        notionWizardState.profile = profile;
        notionWizardState.error = null;
        renderWizardStep();
      } catch (err) {
        notionWizardState.error = String(err);
        renderWizardStep();
      }
    });
  }

  const freshNext = replaceNextBtn();
  freshNext.textContent = 'Next';
  freshNext.addEventListener('click', () => {
    // Pre-populate mappings from people-type properties
    const peopleProps = (profile.properties || []).filter(p => p.type === 'people');
    if (peopleProps.length > 0) {
      notionWizardState.mappings = peopleProps.map(p => ({ alias: p.name, notionUserId: '', displayName: '' }));
    } else if (notionWizardState.mappings.length === 0) {
      notionWizardState.mappings = [{ alias: '', notionUserId: '', displayName: '' }];
    }
    notionWizardState.step = 4;
    renderWizardStep();
  });
}

// Step 4: People Mapping
function renderStep4(body, nextBtn) {
  const profile = notionWizardState.profile;
  const workspaceUsers = (profile && profile.workspace_users) ? profile.workspace_users : [];

  function renderMappingRows() {
    const rowsEl = document.getElementById('wizard-mapping-rows');
    if (!rowsEl) return;

    const userOptions = workspaceUsers.map(u =>
      `<option value="${escapeHtml(u.id)}" data-name="${escapeHtml(u.name)}">${escapeHtml(u.name)}</option>`
    ).join('');

    rowsEl.innerHTML = notionWizardState.mappings.map((m, idx) => `
      <div class="wizard-mapping-row" data-mapping-idx="${idx}">
        <input type="text" class="wizard-mapping-alias" placeholder="Alias (e.g. me)" value="${escapeHtml(m.alias)}" />
        <select class="wizard-mapping-user">
          <option value="">Select user...</option>
          ${userOptions}
        </select>
        <button class="wizard-mapping-remove" title="Remove">x</button>
      </div>
    `).join('');

    // Restore selected user values
    rowsEl.querySelectorAll('.wizard-mapping-row').forEach((row, idx) => {
      const select = row.querySelector('.wizard-mapping-user');
      if (select && notionWizardState.mappings[idx].notionUserId) {
        select.value = notionWizardState.mappings[idx].notionUserId;
      }

      // Alias change handler
      const aliasInput = row.querySelector('.wizard-mapping-alias');
      aliasInput.addEventListener('input', () => {
        notionWizardState.mappings[idx].alias = aliasInput.value;
      });

      // User change handler
      select.addEventListener('change', () => {
        const selectedOpt = select.options[select.selectedIndex];
        notionWizardState.mappings[idx].notionUserId = select.value;
        notionWizardState.mappings[idx].displayName = selectedOpt ? (selectedOpt.dataset.name || '') : '';
      });

      // Remove handler
      const removeBtn = row.querySelector('.wizard-mapping-remove');
      removeBtn.addEventListener('click', () => {
        notionWizardState.mappings.splice(idx, 1);
        renderMappingRows();
      });
    });
  }

  body.innerHTML = `
    <div class="wizard-step-title">People Mapping</div>
    <p class="wizard-step-description">Map aliases (like 'me' or 'team') to Notion workspace users. These aliases can be used in AI output to assign people.</p>
    <div id="wizard-mapping-rows"></div>
    <button id="wizard-add-mapping-btn" class="mini-action-btn" style="margin-top: 4px;">+ Add mapping</button>
    ${notionWizardState.error ? `<div class="wizard-error" id="wizard-mapping-error">${escapeHtml(notionWizardState.error)}</div>` : ''}
  `;

  renderMappingRows();

  const addMappingBtn = document.getElementById('wizard-add-mapping-btn');
  if (addMappingBtn) {
    addMappingBtn.addEventListener('click', () => {
      notionWizardState.mappings.push({ alias: '', notionUserId: '', displayName: '' });
      renderMappingRows();
    });
  }

  const freshNext = replaceNextBtn();
  freshNext.textContent = 'Finish';
  freshNext.addEventListener('click', async () => {
    // Filter out incomplete rows
    const cleanMappings = notionWizardState.mappings.filter(m => m.alias.trim() && m.notionUserId);
    freshNext.disabled = true;
    freshNext.textContent = '...';

    try {
      if (cleanMappings.length > 0) {
        // Convert to snake_case for Rust deserialization
        const payload = cleanMappings.map(m => ({
          alias: m.alias.trim(),
          notion_user_id: m.notionUserId,
          display_name: m.displayName,
        }));
        await window.__TAURI__.core.invoke('update_notion_people_mappings', {
          integrationId: notionWizardState.integrationId,
          mappings: payload,
        });
      }
      closeNotionWizard();
      await loadAllIntegrations();
    } catch (err) {
      notionWizardState.error = String(err);
      freshNext.disabled = false;
      freshNext.textContent = 'Finish';
      const errEl = document.getElementById('wizard-mapping-error');
      if (errEl) {
        errEl.textContent = notionWizardState.error;
      } else {
        const errDiv = document.createElement('div');
        errDiv.id = 'wizard-mapping-error';
        errDiv.className = 'wizard-error';
        errDiv.textContent = notionWizardState.error;
        body.appendChild(errDiv);
      }
    }
  });
}

// Helper: Replace the Next button node to remove all stacked event listeners
function replaceNextBtn() {
  const btn = document.getElementById('notion-wizard-next');
  if (!btn) return btn;
  const clone = btn.cloneNode(true);
  btn.parentNode.replaceChild(clone, btn);
  return clone;
}

// ===== LINEAR WIZARD =====

let linearWizardState = {
  step: 0,            // 0=api-key, 1=team-picker, 2=schema, 3=member-alias
  integrationId: null,
  teams: [],
  selectedTeamId: null,
  selectedTeamName: null,
  profile: null,      // Full LinearIntegrationProfile after sync
  aliases: [],        // [{alias, memberId, displayName}]
  error: null,
};

function resetLinearWizardState() {
  linearWizardState = {
    step: 0,
    integrationId: null,
    teams: [],
    selectedTeamId: null,
    selectedTeamName: null,
    profile: null,
    aliases: [],
    error: null,
  };
}

function closeLinearWizard() {
  resetLinearWizardState();
  const modal = document.getElementById('linear-wizard-modal');
  if (modal) modal.style.display = 'none';
}

function openLinearWizard() {
  resetLinearWizardState();
  const modal = document.getElementById('linear-wizard-modal');
  if (!modal) return;
  modal.style.display = 'flex';

  // Wire cancel button once (not on each render)
  const cancelBtn = document.getElementById('linear-wizard-cancel');
  if (cancelBtn) {
    // Remove previous listener to avoid stacking
    cancelBtn.replaceWith(cancelBtn.cloneNode(true));
    const freshCancel = document.getElementById('linear-wizard-cancel');
    freshCancel.addEventListener('click', async () => {
      if (linearWizardState.integrationId) {
        try {
          await window.__TAURI__.core.invoke('remove_linear_integration', {
            integrationId: linearWizardState.integrationId,
          });
        } catch (err) {
          console.error('Failed to clean up partial Linear integration on cancel:', err);
        }
      }
      closeLinearWizard();
    });
  }

  renderLinearWizardStep();
}

// Progress percentages per step (4 steps: 0-3)
const LINEAR_WIZARD_STEP_PROGRESS = ['25%', '50%', '75%', '100%'];

async function renderLinearWizardStep() {
  const body = document.getElementById('linear-wizard-body');
  const progressBar = document.getElementById('linear-wizard-progress');
  const nextBtn = document.getElementById('linear-wizard-next');
  if (!body || !nextBtn) return;

  // Update progress bar
  if (progressBar) {
    progressBar.style.width = LINEAR_WIZARD_STEP_PROGRESS[linearWizardState.step] || '25%';
  }

  // Update Next button label
  nextBtn.textContent = linearWizardState.step === 3 ? 'Finish' : 'Next';
  nextBtn.disabled = false;

  // Render step body
  switch (linearWizardState.step) {
    case 0: renderLinearStep0(body, nextBtn); break;
    case 1: await renderLinearStep1(body, nextBtn); break;
    case 2: renderLinearStep2(body, nextBtn); break;
    case 3: renderLinearStep3(body, nextBtn); break;
  }
}

// Helper: Replace the Linear Next button node to remove all stacked event listeners
function replaceLinearNextBtn() {
  const btn = document.getElementById('linear-wizard-next');
  if (!btn) return btn;
  const clone = btn.cloneNode(true);
  btn.parentNode.replaceChild(clone, btn);
  return clone;
}

// Step 0: API Key Entry
function renderLinearStep0(body, nextBtn) {
  body.innerHTML = `
    <div class="wizard-step-title">Enter Linear API Key</div>
    <p class="wizard-step-description">Create a personal API key at linear.app/settings/api, then paste it below.</p>
    <div class="wizard-input-group">
      <div>
        <label for="wizard-linear-name">Integration Name</label>
        <input id="wizard-linear-name" type="text" placeholder="Linear" value="Linear" autocomplete="off" />
      </div>
      <div>
        <label for="wizard-linear-apikey">API Key</label>
        <input id="wizard-linear-apikey" type="password" placeholder="lin_api_..." autocomplete="off" spellcheck="false"
          style="font-family: 'SF Mono', monospace; font-size: 0.85rem;" />
      </div>
    </div>
    ${linearWizardState.error ? `<div class="wizard-error">${escapeHtml(linearWizardState.error)}</div>` : ''}
  `;

  const freshNext = replaceLinearNextBtn();
  freshNext.addEventListener('click', async () => {
    const name = (document.getElementById('wizard-linear-name').value || 'Linear').trim();
    const apiKey = (document.getElementById('wizard-linear-apikey').value || '').trim();
    if (!apiKey) {
      linearWizardState.error = 'Please enter an API key.';
      renderLinearWizardStep();
      return;
    }
    freshNext.disabled = true;
    freshNext.textContent = '...';
    try {
      const result = await window.__TAURI__.core.invoke('add_linear_integration', { name, apiKey });
      linearWizardState.integrationId = result;
      linearWizardState.error = null;
      linearWizardState.step = 1;
      renderLinearWizardStep();
    } catch (err) {
      linearWizardState.error = String(err);
      freshNext.disabled = false;
      freshNext.textContent = 'Next';
      renderLinearWizardStep();
    }
  });
}

// Step 1: Team Picker
async function renderLinearStep1(body, nextBtn) {
  body.innerHTML = `<div style="color: var(--text-secondary); font-size: 0.85rem;">Loading teams...</div>`;
  nextBtn.disabled = true;

  try {
    const teams = await window.__TAURI__.core.invoke('list_linear_teams', {
      integrationId: linearWizardState.integrationId,
    });
    linearWizardState.teams = teams;
    linearWizardState.error = null;
    renderLinearStep1Teams(body, nextBtn);
  } catch (err) {
    linearWizardState.error = String(err);
    body.innerHTML = `
      <div class="wizard-step-title">Select Team</div>
      <div class="wizard-error">${escapeHtml(String(err))}</div>
      <button id="linear-retry-teams-btn" class="mini-action-btn" style="margin-top: 12px;">Retry</button>
    `;
    const retryBtn = document.getElementById('linear-retry-teams-btn');
    if (retryBtn) {
      retryBtn.addEventListener('click', async () => {
        await renderLinearStep1(body, nextBtn);
      });
    }
    nextBtn.disabled = true;
  }
}

function renderLinearStep1Teams(body, nextBtn) {
  const { teams, selectedTeamId } = linearWizardState;

  const items = teams.map(team => {
    const isSelected = team.id === selectedTeamId;
    return `<div class="wizard-db-item${isSelected ? ' selected' : ''}" data-team-id="${escapeHtml(team.id)}" data-team-name="${escapeHtml(team.name)}">${escapeHtml(team.name)}</div>`;
  }).join('');

  body.innerHTML = `
    <div class="wizard-step-title">Select Team</div>
    <div class="wizard-db-list">${items}</div>
    ${linearWizardState.error ? `<div class="wizard-error">${escapeHtml(linearWizardState.error)}</div>` : ''}
  `;

  const freshNext = replaceLinearNextBtn();
  freshNext.disabled = !selectedTeamId;
  freshNext.textContent = 'Next';

  body.querySelectorAll('.wizard-db-item').forEach(item => {
    item.addEventListener('click', () => {
      body.querySelectorAll('.wizard-db-item').forEach(i => i.classList.remove('selected'));
      item.classList.add('selected');
      linearWizardState.selectedTeamId = item.dataset.teamId;
      linearWizardState.selectedTeamName = item.dataset.teamName;
      freshNext.disabled = false;
    });
  });

  freshNext.addEventListener('click', async () => {
    if (!linearWizardState.selectedTeamId) return;
    freshNext.disabled = true;
    freshNext.textContent = '...';
    try {
      const profile = await window.__TAURI__.core.invoke('sync_linear_schema', {
        integrationId: linearWizardState.integrationId,
        teamId: linearWizardState.selectedTeamId,
        teamName: linearWizardState.selectedTeamName,
      });
      linearWizardState.profile = profile;
      linearWizardState.error = null;
      linearWizardState.step = 2;
      renderLinearWizardStep();
    } catch (err) {
      linearWizardState.error = String(err);
      freshNext.disabled = false;
      freshNext.textContent = 'Next';
      renderLinearStep1Teams(body, freshNext);
    }
  });
}

// Step 2: Schema Display
function renderLinearStep2(body, nextBtn) {
  const profile = linearWizardState.profile;
  if (!profile) {
    body.innerHTML = '<div class="wizard-error">No schema loaded.</div>';
    return;
  }

  const syncedAt = profile.synced_at
    ? new Date(profile.synced_at).toLocaleString()
    : 'Unknown';

  const stateRows = (profile.workflow_states || []).map(s =>
    `<tr><td>${escapeHtml(s.name)}</td><td>${escapeHtml(s.type_name)}</td></tr>`
  ).join('') || '<tr><td colspan="2" style="color: var(--text-secondary);">None</td></tr>';

  const labelRows = (profile.labels || []).map(l =>
    `<tr><td>${escapeHtml(l.name)}</td><td><span style="display:inline-block;width:10px;height:10px;border-radius:50%;background:${escapeHtml(l.color)};margin-right:4px;"></span>${escapeHtml(l.color)}</td></tr>`
  ).join('') || '<tr><td colspan="2" style="color: var(--text-secondary);">None</td></tr>';

  const memberRows = (profile.members || []).map(m =>
    `<tr><td>${escapeHtml(m.display_name)}</td><td>${escapeHtml(m.email || '—')}</td></tr>`
  ).join('') || '<tr><td colspan="2" style="color: var(--text-secondary);">None</td></tr>';

  const priorityRows = (profile.priorities || []).map(p =>
    `<tr><td>${escapeHtml(p.label)}</td><td>${p.priority}</td></tr>`
  ).join('') || '<tr><td colspan="2" style="color: var(--text-secondary);">None</td></tr>';

  body.innerHTML = `
    <div class="wizard-step-title">Team Schema: ${escapeHtml(profile.team_name)}</div>
    <div style="max-height: 280px; overflow-y: auto; font-size: 0.82rem;">
      <div style="margin-bottom: 10px;">
        <strong>Workflow States</strong>
        <table class="wizard-schema-table">
          <thead><tr><th>Name</th><th>Type</th></tr></thead>
          <tbody>${stateRows}</tbody>
        </table>
      </div>
      <div style="margin-bottom: 10px;">
        <strong>Labels</strong>
        <table class="wizard-schema-table">
          <thead><tr><th>Name</th><th>Color</th></tr></thead>
          <tbody>${labelRows}</tbody>
        </table>
      </div>
      <div style="margin-bottom: 10px;">
        <strong>Members</strong>
        <table class="wizard-schema-table">
          <thead><tr><th>Display Name</th><th>Email</th></tr></thead>
          <tbody>${memberRows}</tbody>
        </table>
      </div>
      <div style="margin-bottom: 10px;">
        <strong>Priorities</strong>
        <table class="wizard-schema-table">
          <thead><tr><th>Label</th><th>Value</th></tr></thead>
          <tbody>${priorityRows}</tbody>
        </table>
      </div>
    </div>
    <div class="wizard-schema-synced">Last synced: ${escapeHtml(syncedAt)}</div>
    <button id="linear-resync-btn" class="mini-action-btn" style="margin-top: 10px;">Re-sync Schema</button>
    ${linearWizardState.error ? `<div class="wizard-error">${escapeHtml(linearWizardState.error)}</div>` : ''}
  `;

  const resyncBtn = document.getElementById('linear-resync-btn');
  if (resyncBtn) {
    resyncBtn.addEventListener('click', async () => {
      resyncBtn.disabled = true;
      resyncBtn.textContent = '...';
      try {
        const profile = await window.__TAURI__.core.invoke('sync_linear_schema', {
          integrationId: linearWizardState.integrationId,
          teamId: linearWizardState.selectedTeamId,
          teamName: linearWizardState.selectedTeamName,
        });
        linearWizardState.profile = profile;
        linearWizardState.error = null;
        renderLinearWizardStep();
      } catch (err) {
        linearWizardState.error = String(err);
        renderLinearWizardStep();
      }
    });
  }

  const freshNext = replaceLinearNextBtn();
  freshNext.textContent = 'Next';
  freshNext.addEventListener('click', () => {
    // Pre-populate aliases with one empty row
    if (linearWizardState.aliases.length === 0) {
      linearWizardState.aliases = [{ alias: '', memberId: '', displayName: '' }];
    }
    linearWizardState.step = 3;
    renderLinearWizardStep();
  });
}

// Step 3: Member Alias Mapping
function renderLinearStep3(body, nextBtn) {
  const profile = linearWizardState.profile;
  const members = (profile && profile.members) ? profile.members : [];

  function renderAliasRows() {
    const rowsEl = document.getElementById('linear-alias-rows');
    if (!rowsEl) return;

    const memberOptions = members.map(m =>
      `<option value="${escapeHtml(m.id)}" data-name="${escapeHtml(m.display_name)}">${escapeHtml(m.display_name)}</option>`
    ).join('');

    rowsEl.innerHTML = linearWizardState.aliases.map((a, idx) => `
      <div class="wizard-mapping-row" data-alias-idx="${idx}">
        <input type="text" class="wizard-mapping-alias" placeholder="Alias (e.g. me)" value="${escapeHtml(a.alias)}" />
        <select class="wizard-mapping-user">
          <option value="">Select member...</option>
          ${memberOptions}
        </select>
        <button class="wizard-mapping-remove" title="Remove">x</button>
      </div>
    `).join('');

    // Restore selected member values and wire handlers
    rowsEl.querySelectorAll('.wizard-mapping-row').forEach((row, idx) => {
      const select = row.querySelector('.wizard-mapping-user');
      if (select && linearWizardState.aliases[idx].memberId) {
        select.value = linearWizardState.aliases[idx].memberId;
      }

      const aliasInput = row.querySelector('.wizard-mapping-alias');
      aliasInput.addEventListener('input', () => {
        linearWizardState.aliases[idx].alias = aliasInput.value;
      });

      select.addEventListener('change', () => {
        const selectedOpt = select.options[select.selectedIndex];
        linearWizardState.aliases[idx].memberId = select.value;
        linearWizardState.aliases[idx].displayName = selectedOpt ? (selectedOpt.dataset.name || '') : '';
      });

      const removeBtn = row.querySelector('.wizard-mapping-remove');
      removeBtn.addEventListener('click', () => {
        linearWizardState.aliases.splice(idx, 1);
        renderAliasRows();
      });
    });
  }

  body.innerHTML = `
    <div class="wizard-step-title">Member Alias Mapping</div>
    <p class="wizard-step-description">Map aliases (like 'me' or 'john') to Linear team members. These aliases can be used in AI output to assign issues.</p>
    <div id="linear-alias-rows"></div>
    <button id="linear-add-alias-btn" class="mini-action-btn" style="margin-top: 4px;">+ Add mapping</button>
    ${linearWizardState.error ? `<div class="wizard-error" id="linear-alias-error">${escapeHtml(linearWizardState.error)}</div>` : ''}
  `;

  renderAliasRows();

  const addAliasBtn = document.getElementById('linear-add-alias-btn');
  if (addAliasBtn) {
    addAliasBtn.addEventListener('click', () => {
      linearWizardState.aliases.push({ alias: '', memberId: '', displayName: '' });
      renderAliasRows();
    });
  }

  const freshNext = replaceLinearNextBtn();
  freshNext.textContent = 'Finish';
  freshNext.addEventListener('click', async () => {
    // Filter out incomplete rows
    const cleanAliases = linearWizardState.aliases.filter(a => a.alias.trim() && a.memberId);
    freshNext.disabled = true;
    freshNext.textContent = '...';

    try {
      if (cleanAliases.length > 0) {
        const payload = cleanAliases.map(a => ({
          alias: a.alias.trim(),
          member_id: a.memberId,
          display_name: a.displayName,
        }));
        await window.__TAURI__.core.invoke('update_linear_member_aliases', {
          integrationId: linearWizardState.integrationId,
          aliases: payload,
        });
      }
      closeLinearWizard();
      await loadAllIntegrations();
    } catch (err) {
      linearWizardState.error = String(err);
      freshNext.disabled = false;
      freshNext.textContent = 'Finish';
      const errEl = document.getElementById('linear-alias-error');
      if (errEl) {
        errEl.textContent = linearWizardState.error;
      } else {
        const errDiv = document.createElement('div');
        errDiv.id = 'linear-alias-error';
        errDiv.className = 'wizard-error';
        errDiv.textContent = linearWizardState.error;
        body.appendChild(errDiv);
      }
    }
  });
}

// ===== INITIALIZATION =====
// Load integrations when the integrations tab is shown.
// The tab switcher in main.js calls switchSettingsTab() — we hook into it by
// observing the integrations tab becoming active.
function initIntegrationsSettings() {
  const observer = new MutationObserver(() => {
    const modelsTab = document.querySelector('.settings-tab-content[data-tab="models"]');
    const intTab = document.querySelector('.settings-tab-content[data-tab="integrations"]');
    if ((modelsTab && modelsTab.classList.contains('active')) ||
        (intTab && intTab.classList.contains('active'))) {
      loadAllIntegrations();
    }
    // Auto-fetch provider models when models tab opens
    if (modelsTab && modelsTab.classList.contains('active')) {
      refreshAllProviderModels();
    }
  });

  const modelsTab = document.querySelector('.settings-tab-content[data-tab="models"]');
  const intTab = document.querySelector('.settings-tab-content[data-tab="integrations"]');
  if (modelsTab) observer.observe(modelsTab, { attributes: true, attributeFilter: ['class'] });
  if (intTab) observer.observe(intTab, { attributes: true, attributeFilter: ['class'] });
}

// Auto-init when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initIntegrationsSettings);
} else {
  initIntegrationsSettings();
}
