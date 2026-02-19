// integrations-settings.js — Integrations settings page (Phase 4)
// State-first full re-render pattern. This module owns the Connected/Available layout.

// Module state — use `var` so these are on `window` and accessible from main.js
var notionProfiles = [];
var savePathIntegrations = [];

const connectedListEl = () => document.getElementById('connected-integrations-list');
const availableListEl = () => document.getElementById('available-integrations-list');

// ===== LOAD ALL INTEGRATIONS =====
async function loadAllIntegrations() {
  await Promise.all([
    loadNotionProfiles(),
    loadSlackForIntegrations(),
    loadSavePathIntegrations(),
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
  for (const [id, data] of Object.entries(slackIntegrations)) {
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

  // Save path cards
  for (const sp of savePathIntegrations) {
    const safeName = escapeHtml(sp.name);
    const safePath = escapeHtml(sp.path);
    cards.push(`
      <div class="integration-card" data-type="save-path" data-id="${escapeHtml(sp.id)}">
        <div class="integration-card-icon save-path">&#128193;</div>
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
      const data = slackIntegrations[id];
      if (!confirm(`Remove Slack workspace "${data ? data.name : id}"?`)) return;
      try {
        await window.__TAURI__.core.invoke('remove_slack_integration', { id });
        await loadAllIntegrations();
      } catch (err) {
        alert('Failed to remove: ' + err);
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
          <div class="integration-card-icon save-path">&#128193;</div>
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
          if (!name) { alert('Name cannot be empty.'); return; }
          if (!selectedPath) { alert('Please select a folder.'); return; }
          saveBtn.disabled = true;
          try {
            await window.__TAURI__.core.invoke('update_save_path_integration', { id: sp.id, name, path: selectedPath });
            await loadAllIntegrations();
          } catch (err) {
            alert('Failed to update: ' + err);
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
      if (!confirm(`Remove save path "${sp ? sp.name : id}"?`)) return;
      try {
        await window.__TAURI__.core.invoke('remove_save_path_integration', { id });
        await loadAllIntegrations();
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

  // Save Path is always available to add (multiple can exist)
  available.push(`
    <div class="available-integration-card" data-type="save-path" id="add-save-path-btn">
      <div class="integration-card-icon save-path">&#128193;</div>
      <div class="integration-card-info">
        <div class="integration-card-name">Save Path</div>
        <div class="integration-card-detail">Save pipeline output to a named folder location</div>
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

  // Save Path add → inline form replaces the card
  const addSavePathBtn = document.getElementById('add-save-path-btn');
  if (addSavePathBtn) {
    addSavePathBtn.addEventListener('click', () => {
      let selectedPath = '';
      addSavePathBtn.outerHTML = `
        <div class="available-integration-card save-path-add-form" id="add-save-path-form">
          <div class="integration-card-icon save-path">&#128193;</div>
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
          if (!name) { alert('Please enter a name for the save path.'); return; }
          if (!selectedPath) { alert('Please select a folder.'); return; }
          saveBtn.disabled = true;
          try {
            await window.__TAURI__.core.invoke('add_save_path_integration', { name, path: selectedPath });
            await loadAllIntegrations();
          } catch (err) {
            alert('Failed to add save path: ' + err);
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
        <label for="wizard-notion-name">Integration Name</label>
        <input id="wizard-notion-name" type="text" placeholder="Notion" value="Notion" autocomplete="off" />
      </div>
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
    const name = (document.getElementById('wizard-notion-name').value || 'Notion').trim();
    const apiKey = (document.getElementById('wizard-notion-apikey').value || '').trim();
    if (!apiKey) {
      notionWizardState.error = 'Please enter an API key.';
      renderWizardStep();
      return;
    }
    freshNext.disabled = true;
    freshNext.textContent = '...';
    try {
      const result = await window.__TAURI__.core.invoke('add_notion_integration', { name, apiKey });
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

  const intTab = document.querySelector('.settings-tab-content[data-tab="integrations"]');
  if (intTab) {
    observer.observe(intTab, { attributes: true, attributeFilter: ['class'] });
  }
}

// Auto-init when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initIntegrationsSettings);
} else {
  initIntegrationsSettings();
}
