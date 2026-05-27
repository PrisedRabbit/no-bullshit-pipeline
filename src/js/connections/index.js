// connections/index.js — Single source for the unified Connections tab.
//
// Backed by the Rust commands in `src-tauri/src/connections.rs`:
//   list_connections, save_connection, delete_connection, test_connection
//
// Each Connection is a flat, self-contained entry: type + non-secret config
// (+ a token in Keychain for types that want one). See
// `docs/connections-model.md` for the model.
//
// Per-type form schemas live in TYPE_SCHEMA below — the renderer is generic.

import { invoke } from '../core/tauri.js';
import { escapeHtml } from '../core/utils.js';
import { showToast } from '../ui/toast.js';
import { showConfirm } from '../ui/confirm-modal.js';
import { CLI_SVG, SLACK_SVG, SAVE_SVG, WEBHOOK_SVG } from '../pipeline/constants.js';

// Type metadata: order, role, label, icon SVG, tagline. Drives sectioning +
// the "+ Add" tile order. Hidden Llm type intentionally absent — code stays
// in Rust for forward-compat (see config.rs::ConnectionType doc).
const TYPES = [
  // Processing
  { key: 'cli_agent', role: 'processing', label: 'CLI agent',     icon: CLI_SVG,                      tagline: 'Claude / Codex / OpenCode running locally' },
  { key: 'shell',     role: 'processing', label: 'Shell script',  icon: shellIcon(),                  tagline: 'Pipe to a local command via stdin → stdout' },
  // Delivery
  { key: 'slack',     role: 'delivery',   label: 'Slack',         icon: SLACK_SVG,                    tagline: 'Post to a channel or DM' },
  // Notion intentionally hidden in v1 — the connector still depends on a
  // legacy `~/.nbp/integrations/notion-{id}.json` profile (database schema)
  // that the new Connection form doesn't produce. ConnectionType::Notion
  // stays in the Rust enum so a settings.json that references it survives
  // round-trip serde, but adding new ones from this UI is blocked until the
  // connector is rewritten to read database_id from connection.config.
  { key: 'telegram',  role: 'delivery',   label: 'Telegram',      icon: telegramIcon(),               tagline: 'Send to a chat via Bot API' },
  { key: 'webhook',   role: 'delivery',   label: 'Webhook',       icon: WEBHOOK_SVG,                  tagline: 'POST to any HTTP endpoint' },
  { key: 'save_local',role: 'delivery',   label: 'Save Local',    icon: SAVE_SVG,                     tagline: 'Write to a local file' },
];

function shellIcon() {
  return `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>`;
}
function telegramIcon() {
  // Telegram paper-plane mark; uses currentColor so it inherits card text colour.
  return `<svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor"><path d="M9.04 15.6 8.86 19c.43 0 .62-.18.85-.4l2.05-1.96 4.25 3.1c.78.43 1.34.2 1.55-.72l2.8-13.13c.27-1.16-.42-1.62-1.18-1.34L2.07 10.05c-1.13.44-1.11 1.07-.19 1.36l4.43 1.39 10.27-6.47c.48-.32.92-.14.56.18z"/></svg>`;
}

// Form schema per type. Each field:
//   { key, label, hint?, type: 'text'|'password'|'textarea'|'select'|'cli-detect',
//     options?, required? }
// `secret: true` on a field means it routes to Keychain via save_connection's
// `token` arg instead of being stored in connection.config.
// `cli-detect` is special: the renderer fetches `check_cli_availability` at
// open time and shows only installed CLIs (with install hints when nothing
// is installed). One CLI per row, install command copy-paste-ready.
const TYPE_SCHEMA = {
  cli_agent: {
    fields: [
      { key: 'cli',           label: 'CLI',            type: 'cli-detect', required: true, hint: 'Only CLIs currently on your PATH show up. Install one to unlock it.' },
      { key: 'model',         label: 'Model',          type: 'text',   hint: 'Optional. Free-text — CLI validates at runtime. Examples: claude → sonnet · codex → o3 · opencode → openai/gpt-4o · agy → gemini-3.1-pro.' },
      { key: 'timeout_secs',  label: 'Timeout (sec)',  type: 'number', hint: 'Default 300. The whole subprocess is killed at this limit.' },
    ],
  },
  shell: {
    fields: [
      // Script-mode: the Connection is just an *environment* (cwd / shell /
      // env / timeout). The actual script lives at the pipeline-step level
      // (step.template). No `command` / `args` here — the user writes
      // arbitrary shell at step time.
      { key: 'cwd',           label: 'Working dir',    type: 'text', required: true, hint: 'Required. Where the script runs. `~` expanded. Example: ~/work/notes' },
      { key: 'shell',         label: 'Shell',          type: 'text', hint: 'Binary that accepts -c <script>. Default /bin/bash. Use /bin/zsh, /usr/bin/env fish, etc. if you prefer.' },
      { key: 'env',           label: 'Env vars (KEY=value per line)', type: 'textarea', hint: 'Optional. Merged on top of NBP_TRANSCRIPT / NBP_APP / NBP_PROCESSING_RESULT (so you CAN override them — rare).' },
      { key: 'timeout_secs',  label: 'Timeout (sec)',  type: 'number', hint: 'Default 120. Kills the subprocess if exceeded.' },
    ],
  },
  slack: {
    fields: [
      { key: 'token',         label: 'Bot token',      type: 'password', secret: true, required: true, hint: 'xoxb-… token from your Slack app. Stored in Keychain.' },
      { key: 'target',        label: 'Channel or user',type: 'text', required: true, hint: '#channel, @user, email, or a C/D/G/U id. The bot must be in the channel.' },
      { key: 'thread_ts',     label: 'Thread (optional)', type: 'text', hint: 'Reply to an existing thread by ts. Leave blank for a fresh message.' },
    ],
  },
  notion: {
    fields: [
      { key: 'token',         label: 'Integration token', type: 'password', secret: true, required: true, hint: 'Internal Integration token from notion.so/profile/integrations. Stored in Keychain.' },
      { key: 'database_id',   label: 'Database id',    type: 'text', required: true, hint: '32-char id from the database URL.' },
    ],
  },
  telegram: {
    fields: [
      { key: 'token',         label: 'Bot token',      type: 'password', secret: true, required: true, hint: 'Token from @BotFather. Stored in Keychain.' },
      { key: 'chat_id',       label: 'Chat id',        type: 'text', required: true, hint: 'Numeric id (e.g. -100123...) or @channelname.' },
      { key: 'parse_mode',    label: 'Parse mode',     type: 'select', options: ['', 'MarkdownV2', 'HTML'], hint: 'Leave blank for plain text.' },
    ],
  },
  webhook: {
    fields: [
      { key: 'url',           label: 'URL',            type: 'text', required: true, hint: 'Must start with http:// or https://.' },
      { key: 'method',        label: 'Method',         type: 'select', options: ['POST', 'PUT', 'PATCH'] },
      { key: 'body_format',   label: 'Body format',    type: 'select', options: ['text', 'json'], hint: 'json wraps the content as {"content": "…"}; text sends it as the body.' },
    ],
  },
  save_local: {
    fields: [
      { key: 'folder_path',   label: 'Folder',         type: 'text', required: true, hint: 'Where the file lands. `~` expanded. Example: ~/Documents/Meetings' },
    ],
  },
};

// Local state (loaded once per tab activation, refreshed on mutate).
let connections = [];
let editingType = null;        // when a form is open: type key
let editingExisting = null;    // when editing an existing entry: connection object

// `check_cli_availability` result cache. Refreshed when the CLI Agent form
// opens — cheap (`which X` per supported CLI) and means newly-installed
// agents show up without an app restart.
let cliAvailabilityCache = [];

async function refreshCliAvailability() {
  try {
    cliAvailabilityCache = await invoke('check_cli_availability');
  } catch (err) {
    console.error('check_cli_availability failed:', err);
    cliAvailabilityCache = [];
  }
}

const connectedById = (id) => connections.find(c => c.id === id);
const typeMeta = (key) => TYPES.find(t => t.key === key);

function fieldsOf(typeKey) {
  return (TYPE_SCHEMA[typeKey]?.fields) || [];
}

// --- Entry points ---------------------------------------------------------

export async function initConnectionsTab() {
  // MutationObserver pattern matches the legacy integrations tab — render
  // when the Connections tab becomes active so we hit the backend only when
  // the user actually looks at the tab.
  const tabEl = document.querySelector('.settings-tab-content[data-tab="connections"]');
  if (!tabEl) return;
  const observer = new MutationObserver(() => {
    if (tabEl.classList.contains('active')) loadConnections();
  });
  observer.observe(tabEl, { attributes: true, attributeFilter: ['class'] });
  if (tabEl.classList.contains('active')) loadConnections();
}

export async function loadConnections() {
  try {
    connections = await invoke('list_connections');
  } catch (err) {
    console.error('list_connections failed:', err);
    connections = [];
    showToast('Failed to load connections: ' + err, 'error');
  }
  renderAll();
}

// --- Rendering ------------------------------------------------------------

function renderAll() {
  renderConnectedList('processing');
  renderConnectedList('delivery');
  renderAvailableList('processing');
  renderAvailableList('delivery');
}

function renderConnectedList(role) {
  const el = document.getElementById(`connections-${role}-list`);
  if (!el) return;
  const items = connections.filter(c => typeMeta(c.connection_type || c.type)?.role === role);
  if (items.length === 0) {
    el.innerHTML = `<div style="text-align:center;padding:24px 0;color:var(--text-secondary);opacity:0.6;">No ${role} connections yet</div>`;
    return;
  }
  el.innerHTML = items.map(renderCard).join('');
  wireCardHandlers(el);
}

function renderCard(c) {
  const typeKey = c.connection_type || c.type;
  const meta = typeMeta(typeKey) || { label: typeKey, icon: '', tagline: '' };
  const detail = summariseConfig(typeKey, c.config || {});
  return `
    <div class="integration-card" data-id="${escapeHtml(c.id)}" data-type="${escapeHtml(typeKey)}">
      <div class="integration-card-icon ${escapeHtml(typeKey)}">${meta.icon}</div>
      <div class="integration-card-info">
        <div class="integration-card-name">${escapeHtml(c.name)}</div>
        <div class="integration-card-detail">${meta.label}${detail ? ' · ' + escapeHtml(detail) : ''}</div>
      </div>
      <div class="integration-card-actions">
        <button class="mini-action-btn conn-test-btn" data-id="${escapeHtml(c.id)}">Test</button>
        <button class="mini-action-btn conn-edit-btn" data-id="${escapeHtml(c.id)}">Edit</button>
        <button class="mini-action-btn danger conn-delete-btn" data-id="${escapeHtml(c.id)}">Remove</button>
      </div>
    </div>
  `;
}

// One-line "what's distinctive about this entry" string — shown on the card
// so users with two Slacks / three folders can tell them apart at a glance.
function summariseConfig(typeKey, cfg) {
  switch (typeKey) {
    case 'cli_agent':  return [cfg.cli, cfg.model].filter(Boolean).join(' · ');
    case 'shell':      return cfg.command || '';
    case 'slack':      return cfg.target || '';
    case 'notion':     return cfg.database_id ? `db ${String(cfg.database_id).slice(0, 8)}…` : '';
    case 'telegram':   return cfg.chat_id || '';
    case 'webhook':    return [cfg.method || 'POST', cfg.url].filter(Boolean).join(' ');
    case 'save_local': return cfg.folder_path || '';
    default:           return '';
  }
}

function renderAvailableList(role) {
  const el = document.getElementById(`connections-${role}-add`);
  if (!el) return;
  const types = TYPES.filter(t => t.role === role);
  el.innerHTML = types.map(t => `
    <div class="available-integration-card" data-add-type="${escapeHtml(t.key)}">
      <div class="integration-card-icon ${escapeHtml(t.key)}">${t.icon}</div>
      <div class="integration-card-info">
        <div class="integration-card-name">${escapeHtml(t.label)}</div>
        <div class="integration-card-detail">${escapeHtml(t.tagline)}</div>
      </div>
      <span class="available-add-label">+ Add</span>
    </div>
  `).join('');
  el.querySelectorAll('[data-add-type]').forEach(card => {
    card.addEventListener('click', () => openFormFor(card.dataset.addType, null));
  });
}

function wireCardHandlers(scopeEl) {
  scopeEl.querySelectorAll('.conn-edit-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const c = connectedById(btn.dataset.id);
      if (c) openFormFor(c.connection_type || c.type, c);
    });
  });
  scopeEl.querySelectorAll('.conn-test-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      btn.disabled = true;
      const original = btn.textContent;
      btn.textContent = 'Testing…';
      try {
        const msg = await invoke('test_connection', { id: btn.dataset.id });
        showToast(msg, 'success');
      } catch (err) {
        showToast('Test failed: ' + err, 'error');
      } finally {
        btn.disabled = false;
        btn.textContent = original;
      }
    });
  });
  scopeEl.querySelectorAll('.conn-delete-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const c = connectedById(btn.dataset.id);
      if (!c) return;
      await handleDelete(c);
    });
  });
}

// --- Per-type form (inline modal) -----------------------------------------

async function openFormFor(typeKey, existing) {
  editingType = typeKey;
  editingExisting = existing;
  const meta = typeMeta(typeKey);
  if (!meta) return;

  const fields = fieldsOf(typeKey);
  // Refresh CLI installation status before rendering — so the form shows
  // newly-installed agents without an app restart.
  if (fields.some(f => f.type === 'cli-detect')) {
    await refreshCliAvailability();
  }
  const initial = existing ? { name: existing.name, ...(existing.config || {}) } : {};
  // Shell env: stored as { KEY: value }, rendered as KEY=value lines for the
  // textarea. Round-tripped in submitForm.
  if (typeKey === 'shell' && initial.env && typeof initial.env === 'object' && !Array.isArray(initial.env)) {
    initial.env = Object.entries(initial.env)
      .map(([k, v]) => `${k}=${v}`)
      .join('\n');
  }
  const formHTML = `
    <div class="modal-overlay" id="connection-form-modal" style="display:flex;">
      <div class="modal-card" style="max-width:520px;text-align:left;">
        <h3 style="margin:0 0 12px;">${existing ? 'Edit' : 'Add'} ${escapeHtml(meta.label)}</h3>
        <div style="display:flex;flex-direction:column;gap:12px;">
          <label style="display:flex;flex-direction:column;gap:4px;">
            <span style="font-size:0.8rem;color:var(--text-secondary);">Name</span>
            <input id="conn-form-name" type="text" placeholder="Memorable label (e.g. Work Slack)" value="${escapeHtml(initial.name || '')}" />
          </label>
          ${fields.map(f => renderField(f, initial, !!existing)).join('')}
        </div>
        <div class="modal-actions" style="display:flex;justify-content:flex-end;gap:8px;margin-top:16px;">
          <button class="modal-btn secondary" id="conn-form-cancel">Cancel</button>
          <button class="modal-btn primary" id="conn-form-save">${existing ? 'Save' : 'Add'}</button>
        </div>
      </div>
    </div>
  `;
  const host = document.createElement('div');
  host.innerHTML = formHTML;
  document.body.appendChild(host.firstElementChild);

  document.getElementById('conn-form-cancel').addEventListener('click', closeForm);
  document.getElementById('conn-form-save').addEventListener('click', () => submitForm(typeKey, existing));
  const modalEl = document.getElementById('connection-form-modal');
  modalEl.addEventListener('click', (e) => { if (e.target === modalEl) closeForm(); });
}

function renderField(f, initial, isEdit) {
  // When editing, secret fields stay blank — backend keeps the existing
  // token on null/empty (see save_connection in Rust). Surface that to the
  // user with a placeholder so they don't think the password was lost.
  const value = initial[f.key];
  const placeholder = f.secret && isEdit ? '(unchanged — leave blank to keep)' : '';
  const hintHTML = f.hint
    ? `<span style="font-size:0.72rem;color:var(--text-secondary);opacity:0.75;">${escapeHtml(f.hint)}</span>`
    : '';
  const required = f.required && !(f.secret && isEdit) ? ' required' : '';

  if (f.type === 'cli-detect') {
    return renderCliDetectField(f, value, hintHTML);
  }
  if (f.type === 'select') {
    const opts = (f.options || []).map(o => {
      const sel = String(value || '') === String(o) ? ' selected' : '';
      return `<option value="${escapeHtml(o)}"${sel}>${o === '' ? '— none —' : escapeHtml(o)}</option>`;
    }).join('');
    return `
      <label style="display:flex;flex-direction:column;gap:4px;">
        <span style="font-size:0.8rem;color:var(--text-secondary);">${escapeHtml(f.label)}</span>
        <select data-field="${escapeHtml(f.key)}">${opts}</select>
        ${hintHTML}
      </label>
    `;
  }
  if (f.type === 'textarea') {
    return `
      <label style="display:flex;flex-direction:column;gap:4px;">
        <span style="font-size:0.8rem;color:var(--text-secondary);">${escapeHtml(f.label)}</span>
        <textarea data-field="${escapeHtml(f.key)}" rows="3" placeholder="${escapeHtml(placeholder)}"${required}>${escapeHtml(value || '')}</textarea>
        ${hintHTML}
      </label>
    `;
  }
  const inputType = f.type === 'password' ? 'password' : (f.type === 'number' ? 'number' : 'text');
  return `
    <label style="display:flex;flex-direction:column;gap:4px;">
      <span style="font-size:0.8rem;color:var(--text-secondary);">${escapeHtml(f.label)}</span>
      <input data-field="${escapeHtml(f.key)}" type="${inputType}" placeholder="${escapeHtml(placeholder)}" value="${escapeHtml(value == null ? '' : String(value))}"${required} />
      ${hintHTML}
    </label>
  `;
}

// Render the CLI dropdown using `check_cli_availability` cache. Only installed
// CLIs show as pickable options; un-installed ones list below with a
// copy-paste install command. If the connection being edited references a CLI
// that's currently missing (e.g. uninstalled since save), it's shown as a
// disabled-looking option flagged with ⚠ so the user notices.
function renderCliDetectField(f, currentValue, hintHTML) {
  const installed = cliAvailabilityCache.filter(c => c.installed);
  const missing = cliAvailabilityCache.filter(c => !c.installed);

  // Build options: installed CLIs in alphabetical-by-display-name order.
  const installedOpts = installed
    .map(c => {
      const sel = currentValue === c.id ? ' selected' : '';
      return `<option value="${escapeHtml(c.id)}"${sel}>${escapeHtml(c.name)} (${escapeHtml(c.id)})</option>`;
    })
    .join('');

  // If editing an entry whose CLI is no longer installed, preserve the value
  // with a warning so save doesn't silently switch it.
  const stale = currentValue && !installed.some(c => c.id === currentValue);
  const staleOpt = stale
    ? `<option value="${escapeHtml(currentValue)}" selected>⚠ ${escapeHtml(currentValue)} — not installed</option>`
    : '';

  const placeholderOpt = !currentValue && installed.length > 0
    ? '<option value="" disabled selected>— choose a CLI —</option>'
    : '';

  const noneInstalled = installed.length === 0 && !stale;

  const selectHTML = noneInstalled
    ? `<select data-field="${escapeHtml(f.key)}" disabled style="opacity:0.6;"><option>No CLI agents installed</option></select>`
    : `<select data-field="${escapeHtml(f.key)}">${placeholderOpt}${staleOpt}${installedOpts}</select>`;

  // Always show the install hints for missing CLIs — even when others ARE
  // installed. Users picking «which agent to use» benefit from seeing what
  // else they could enable with one command.
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
      <span style="font-size:0.8rem;color:var(--text-secondary);">${escapeHtml(f.label)}</span>
      ${selectHTML}
      ${hintHTML}
      ${missingHTML}
    </label>
  `;
}

function closeForm() {
  const modal = document.getElementById('connection-form-modal');
  if (modal) modal.remove();
  editingType = null;
  editingExisting = null;
}

async function submitForm(typeKey, existing) {
  const modal = document.getElementById('connection-form-modal');
  if (!modal) return;

  const name = (document.getElementById('conn-form-name')?.value || '').trim();
  if (!name) {
    showToast('Name is required', 'error');
    return;
  }

  const config = {};
  let secretToken = null;
  for (const f of fieldsOf(typeKey)) {
    const el = modal.querySelector(`[data-field="${f.key}"]`);
    if (!el) continue;
    let raw = (el.value || '').trim();
    if (f.secret) {
      if (raw) secretToken = raw;
      continue;
    }
    if (raw === '') continue;
    if (f.type === 'number') {
      const n = Number(raw);
      if (!Number.isFinite(n)) {
        showToast(`${f.label}: must be a number`, 'error');
        return;
      }
      config[f.key] = n;
    } else if (typeKey === 'shell' && f.key === 'env') {
      // KEY=value per line → { KEY: value } object. Skip blank lines + lines
      // missing `=`. Trim KEY but NOT value (whitespace can matter).
      const env = {};
      for (const line of raw.split(/\r?\n/)) {
        if (!line.trim() || !line.includes('=')) continue;
        const eq = line.indexOf('=');
        const key = line.slice(0, eq).trim();
        const val = line.slice(eq + 1);
        if (key) env[key] = val;
      }
      if (Object.keys(env).length > 0) config.env = env;
    } else {
      config[f.key] = raw;
    }
  }

  // Required-field check on non-secret fields. Secret + edit allows blank
  // (keeps existing token); secret + new requires the token.
  for (const f of fieldsOf(typeKey)) {
    if (!f.required) continue;
    if (f.secret) {
      if (!existing && !secretToken) {
        showToast(`${f.label} is required`, 'error');
        return;
      }
      continue;
    }
    if (config[f.key] == null || config[f.key] === '') {
      showToast(`${f.label} is required`, 'error');
      return;
    }
  }

  const payload = {
    id: existing?.id || '',
    name,
    type: typeKey, // serde rename "type" → connection_type in Rust
    config,
    created_at: existing?.created_at || '',
  };

  try {
    await invoke('save_connection', { connection: payload, token: secretToken });
    closeForm();
    await loadConnections();
    showToast(`${existing ? 'Updated' : 'Added'}: ${name}`, 'success');
  } catch (err) {
    showToast('Save failed: ' + err, 'error');
  }
}

async function handleDelete(c) {
  // First attempt without force — backend returns the list of referencing
  // pipelines if any, instead of deleting (see Rust delete_connection).
  let report;
  try {
    report = await invoke('delete_connection', { id: c.id, force: false });
  } catch (err) {
    showToast('Delete failed: ' + err, 'error');
    return;
  }

  if (report.referenced_by_pipelines && report.referenced_by_pipelines.length > 0) {
    const pipelineList = report.referenced_by_pipelines.join(', ');
    const ok = await showConfirm(
      'Connection is in use',
      `"${c.name}" is referenced by pipelines: ${pipelineList}.\n\nForce-delete will leave those pipelines pointing at a missing Connection — they will fail at run time with a clear error.`
    );
    if (!ok) return;
    try {
      await invoke('delete_connection', { id: c.id, force: true });
    } catch (err) {
      showToast('Delete failed: ' + err, 'error');
      return;
    }
  }

  await loadConnections();
  showToast(`Removed: ${c.name}`, 'success');
}

// Expose the type list so the step editor can build its type+connection picker
// from the same source of truth.
export function getConnectionTypes() {
  return TYPES.slice();
}
export function getLoadedConnections() {
  return connections.slice();
}
