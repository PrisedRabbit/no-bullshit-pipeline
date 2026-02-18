// integrations-settings.js — Integrations settings page (Phase 4)
// State-first full re-render pattern. This module owns the Connected/Available layout.

// Module state — use `var` so these are on `window` and accessible from main.js
var notionProfiles = [];
var _slackIntegrations = {}; // shadow — main.js still owns slackIntegrations

const connectedListEl = () => document.getElementById('connected-integrations-list');
const availableListEl = () => document.getElementById('available-integrations-list');

// ===== LOAD ALL INTEGRATIONS =====
async function loadAllIntegrations() {
  await Promise.all([
    loadNotionProfiles(),
    loadSlackForIntegrations(),
  ]);
  renderConnectedIntegrations();
  renderAvailableIntegrations();
}

async function loadNotionProfiles() {
  try {
    notionProfiles = await window.__TAURI__.core.invoke('list_notion_profiles');
  } catch (err) {
    console.error('Failed to load Notion profiles:', err);
    notionProfiles = [];
  }
}

async function loadSlackForIntegrations() {
  try {
    _slackIntegrations = await window.__TAURI__.core.invoke('list_slack_integrations');
  } catch (err) {
    console.error('Failed to load Slack integrations:', err);
    _slackIntegrations = {};
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
    cards.push(`
      <div class="integration-card" data-type="notion" data-id="${escapeHtml(profile.id)}">
        <div class="integration-card-icon notion">N</div>
        <div class="integration-card-info">
          <div class="integration-card-name">${safeName}</div>
          <div class="integration-card-detail">${safeDb} · Synced ${syncedAt}</div>
        </div>
        <div class="integration-card-actions">
          <button class="mini-action-btn test-notion-btn" data-id="${escapeHtml(profile.id)}">Test</button>
          <button class="mini-action-btn danger remove-notion-btn" data-id="${escapeHtml(profile.id)}">Remove</button>
        </div>
      </div>
    `);
  }

  // Slack cards
  for (const [id, data] of Object.entries(_slackIntegrations)) {
    const safeName = escapeHtml(data.name);
    const safeWorkspace = escapeHtml(data.workspace_name || 'Unknown workspace');
    cards.push(`
      <div class="integration-card" data-type="slack" data-id="${escapeHtml(id)}">
        <div class="integration-card-icon slack">S</div>
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
        alert('Notion: ' + result);
      } catch (err) {
        alert('Notion test failed: ' + err);
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
      if (!confirm(`Remove Notion integration "${profile ? profile.name : id}"?`)) return;
      try {
        await window.__TAURI__.core.invoke('remove_notion_integration', { integrationId: id });
        await loadAllIntegrations();
      } catch (err) {
        alert('Failed to remove: ' + err);
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
        alert('Connected to: ' + workspaceName);
      } catch (err) {
        alert('Connection failed: ' + err);
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
      const data = _slackIntegrations[id];
      if (!confirm(`Remove Slack workspace "${data ? data.name : id}"?`)) return;
      try {
        await window.__TAURI__.core.invoke('remove_slack_integration', { id });
        await loadAllIntegrations();
        // Also refresh main.js slackIntegrations if the function exists
        if (typeof loadSlackIntegrations === 'function') loadSlackIntegrations();
      } catch (err) {
        alert('Failed to remove: ' + err);
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
      <div class="integration-card-icon notion">N</div>
      <div class="integration-card-info">
        <div class="integration-card-name">Notion</div>
        <div class="integration-card-detail">Connect a Notion database for automatic page creation</div>
      </div>
      <span class="available-add-label">+ Add</span>
    </div>
  `);

  // Slack is always available to add
  available.push(`
    <div class="available-integration-card" data-type="slack" id="add-slack-integration-btn">
      <div class="integration-card-icon slack">S</div>
      <div class="integration-card-info">
        <div class="integration-card-name">Slack</div>
        <div class="integration-card-detail">Send pipeline output to Slack channels or DMs</div>
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
        alert('Notion setup wizard not yet available');
      }
    });
  }

  // Slack add → reuse existing add-slack-modal from main.js
  const addSlackIntBtn = document.getElementById('add-slack-integration-btn');
  if (addSlackIntBtn) {
    addSlackIntBtn.addEventListener('click', () => {
      const modal = document.getElementById('add-slack-modal');
      const nameInput = document.getElementById('slack-name-input');
      const tokenInput = document.getElementById('slack-token-input');
      if (modal) {
        if (nameInput) nameInput.value = '';
        if (tokenInput) tokenInput.value = '';
        modal.style.display = 'flex';
      }
    });
  }
}

// ===== INITIALIZATION =====
// Load integrations when the integrations tab is shown.
// The tab switcher in main.js calls switchSettingsTab() — we hook into it by
// observing the integrations tab becoming active.
function initIntegrationsSettings() {
  const observer = new MutationObserver(() => {
    const intTab = document.querySelector('.settings-tab-content[data-tab="integrations"]');
    if (intTab && intTab.classList.contains('active')) {
      loadAllIntegrations();
    }
  });

  const settingsContainer = document.querySelector('.settings-tabs-container');
  if (settingsContainer) {
    // Observe the integrations tab for class changes
    const intTab = document.querySelector('.settings-tab-content[data-tab="integrations"]');
    if (intTab) {
      observer.observe(intTab, { attributes: true, attributeFilter: ['class'] });
    }
  }
}

// Auto-init when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initIntegrationsSettings);
} else {
  initIntegrationsSettings();
}
