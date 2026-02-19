// ui-health-check.js
// Loaded LAST (after pipeline-builder.js) so all globals are available:
// escapeHtml, invoke, appSettings
// All state is module-local — no var globals, no ES modules

const AUDIT_ELEMENTS = [
  // App bar
  { id: 'record-toggle-btn',         desc: 'Record button' },
  { id: 'pipeline-chip-bar',         desc: 'Pipeline chip bar container' },
  { id: 'status-indicator',          desc: 'Status indicator dot' },
  { id: 'timer',                     desc: 'Timer display' },
  { id: 'permission-warning',        desc: 'Permission warning banner' },
  // Sidebar
  { id: 'settings-btn',              desc: 'Settings button' },
  { id: 'sidebar-pipelines-btn',     desc: 'Pipelines sidebar nav' },
  { id: 'sidebar-templates-btn',     desc: 'Templates sidebar nav' },
  // Settings view
  { id: 'settings-view',             desc: 'Settings view section' },
  { id: 'settings-tabs',             desc: 'Settings tab bar' },
  { id: 'save-settings-btn',         desc: 'Save Settings button' },
  { id: 'settings-back-btn',         desc: 'Settings back button' },
  // Settings > Audio
  { id: 'settings-transcription-enabled', desc: 'Auto-transcribe toggle' },
  { id: 'settings-storage-path',     desc: 'Storage path input' },
  { id: 'browse-storage-btn',        desc: 'Browse storage button' },
  { id: 'settings-default-pipeline', desc: 'Default pipeline select' },
  // Settings > Pipelines
  { id: 'pipeline-defs-list',        desc: 'Pipeline definitions list' },
  { id: 'add-pipeline-def-btn',      desc: 'Add pipeline button' },
  { id: 'pipeline-editor',           desc: 'Pipeline editor (hidden)' },
  // Settings > Templates
  { id: 'prompt-templates-list',     desc: 'Prompt templates list' },
  { id: 'add-prompt-template-btn',   desc: 'Add template button' },
  // Settings > Integrations (containers only — content lazy-loaded on tab activation)
  { id: 'connected-integrations-list',  desc: 'Connected integrations container' },
  { id: 'available-integrations-list',  desc: 'Available integrations container' },
  // Settings > Theme
  { id: 'theme-purple-btn',          desc: 'Neon Purple theme button' },
  { id: 'theme-blue-btn',            desc: 'Deep Blue theme button' },
  { id: 'theme-light-btn',           desc: 'Light theme button' },
  // Detail view
  { id: 'detail-view',               desc: 'Detail view section' },
  { id: 'back-btn',                  desc: 'Back button in detail view' },
  { id: 'detail-title',              desc: 'Recording title input' },
  { id: 'process-btn',               desc: 'Transcribe button' },
  { id: 'detail-pipeline-select',    desc: 'Pipeline assignment select' },
  // Modals
  { id: 'delete-modal',              desc: 'Delete confirmation modal' },
  { id: 'add-slack-modal',           desc: 'Add Slack workspace modal' },
  { id: 'notion-wizard-modal',       desc: 'Notion setup wizard modal' },
  { id: 'onboarding-overlay',        desc: 'Onboarding overlay' },
];

// ===== AUDIT ENGINE =====

function runHealthAudit() {
  const issues = [];
  for (const spec of AUDIT_ELEMENTS) {
    const el = document.getElementById(spec.id);
    if (!el) {
      issues.push({
        element: spec.id,
        description: spec.desc + ' is missing from the DOM',
        fix: 'Check index.html for element with id="' + spec.id + '"'
      });
    }
  }
  const result = {
    passed: AUDIT_ELEMENTS.length - issues.length,
    failed: issues.length,
    issues
  };
  window._lastHealthResult = result;
  renderHealthBadge(result);
  return result;
}

// ===== BADGE RENDERER =====

function renderHealthBadge(result) {
  const badge = document.getElementById('health-badge');
  if (!badge) return;

  if (result.failed === 0) {
    badge.className = 'health-badge health-badge-ok';
    badge.textContent = '\u2713';
    badge.title = 'UI health: all ' + result.passed + ' elements verified';
    badge.style.cursor = 'default';
    badge.onclick = null;
  } else {
    badge.className = 'health-badge health-badge-fail';
    badge.textContent = '\u26a0 ' + result.failed;
    badge.title = result.failed + ' element' + (result.failed === 1 ? '' : 's') + ' failed health check — click for report';
    badge.style.cursor = 'pointer';
    badge.onclick = () => showHealthReport(result.issues);
  }

  badge.style.display = '';
}

// ===== REPORT MODAL =====

function showHealthReport(issues) {
  const modal = document.getElementById('health-report-modal');
  const body = document.getElementById('health-report-body');
  if (!modal || !body) return;

  body.innerHTML = issues.map(function(issue) {
    return '<div class="health-issue-row">' +
      '<div class="health-issue-id">' + escapeHtml(issue.element) + '</div>' +
      '<div class="health-issue-desc">' + escapeHtml(issue.description) + '</div>' +
      '<div class="health-issue-fix">Fix: ' + escapeHtml(issue.fix) + '</div>' +
      '</div>';
  }).join('');

  modal.style.display = 'flex';
}

// ===== INIT (wires modal button listeners ONCE) =====

function initHealthCheck() {
  // Close button hides modal
  const closeBtn = document.getElementById('health-report-close-btn');
  if (closeBtn) {
    closeBtn.addEventListener('click', function() {
      const modal = document.getElementById('health-report-modal');
      if (modal) modal.style.display = 'none';
    });
  }

  // Re-run audit button re-runs and hides modal if all pass
  const rerunBtn = document.getElementById('health-report-rerun-btn');
  if (rerunBtn) {
    rerunBtn.addEventListener('click', function() {
      const result = runHealthAudit();
      if (result.failed === 0) {
        const modal = document.getElementById('health-report-modal');
        if (modal) modal.style.display = 'none';
      }
    });
  }
}
