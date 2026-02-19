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

  // Walkthrough navigation buttons
  const prevBtn = document.getElementById('walkthrough-prev');
  if (prevBtn) {
    prevBtn.addEventListener('click', function() {
      if (walkthroughStep > 0) {
        showWalkthroughStep(--walkthroughStep);
      }
    });
  }

  const nextBtn = document.getElementById('walkthrough-next');
  if (nextBtn) {
    nextBtn.addEventListener('click', function() {
      if (walkthroughStep >= WALKTHROUGH_STEPS.length - 1) {
        finishWalkthrough();
      } else {
        showWalkthroughStep(++walkthroughStep);
      }
    });
  }

  const skipBtn = document.getElementById('walkthrough-skip');
  if (skipBtn) {
    skipBtn.addEventListener('click', function() {
      finishWalkthrough();
    });
  }

  const startBtn = document.getElementById('start-walkthrough-btn');
  if (startBtn) {
    startBtn.addEventListener('click', function() {
      startWalkthrough();
    });
  }
}

// ===== WALKTHROUGH ENGINE =====

const WALKTHROUGH_STEPS = [
  { selector: '#pipeline-chip-bar',     title: 'Pipeline Chips',    desc: 'Click any chip to instantly start recording with that pipeline pre-assigned. No menus, no navigation.' },
  { selector: '#record-toggle-btn',     title: 'Record Button',     desc: 'Start or stop recording. When a recording is selected, this plays it back instead.' },
  { selector: '#sidebar-pipelines-btn', title: 'Pipelines',         desc: 'Open Settings > Pipelines to create multi-step AI processing pipelines with drag-and-drop.' },
  { selector: '#sidebar-templates-btn', title: 'Templates',         desc: 'Open Settings > Templates to create reusable AI prompt templates.' },
  { selector: '#settings-btn',          title: 'Settings',          desc: 'Configure audio, integrations (Notion, Slack), pipelines, and appearance.' },
  { selector: '#recordings-list',       title: 'Recordings',        desc: 'All your recordings appear here. Click any recording to open its detail view with transcript and pipeline status.' },
  { selector: '#health-badge',          title: 'Health Badge',      desc: 'This badge confirms all UI elements loaded correctly. Green means healthy. Red means something is missing — click it for details.' },
];

let walkthroughStep = 0;

function startWalkthrough() {
  walkthroughStep = 0;
  const overlay = document.getElementById('walkthrough-overlay');
  if (overlay) overlay.style.display = 'flex';
  showWalkthroughStep(0);
}

function showWalkthroughStep(stepIndex) {
  const step = WALKTHROUGH_STEPS[stepIndex];
  if (!step) return;

  const target = document.querySelector(step.selector);
  const spotlight = document.getElementById('walkthrough-spotlight');
  const titleEl = document.getElementById('walkthrough-title');
  const descEl = document.getElementById('walkthrough-desc');
  const stepCounter = document.getElementById('walkthrough-step');
  const prevBtn = document.getElementById('walkthrough-prev');
  const nextBtn = document.getElementById('walkthrough-next');
  const card = document.getElementById('walkthrough-card');

  if (titleEl) titleEl.textContent = step.title;
  if (descEl) descEl.textContent = step.desc;
  if (stepCounter) stepCounter.textContent = (stepIndex + 1) + ' / ' + WALKTHROUGH_STEPS.length;
  if (prevBtn) prevBtn.style.display = stepIndex === 0 ? 'none' : '';
  if (nextBtn) nextBtn.textContent = stepIndex === WALKTHROUGH_STEPS.length - 1 ? 'Done' : 'Next';

  if (target && card && spotlight) {
    const rect = target.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      const pad = 8;
      spotlight.style.position = 'fixed';
      spotlight.style.top = (rect.top - pad) + 'px';
      spotlight.style.left = (rect.left - pad) + 'px';
      spotlight.style.width = (rect.width + pad * 2) + 'px';
      spotlight.style.height = (rect.height + pad * 2) + 'px';
      spotlight.style.boxShadow = '0 0 0 9999px rgba(0,0,0,0.6)';
      spotlight.style.display = 'block';

      // Position card below spotlight, or above if near bottom of viewport
      const spotlightBottom = rect.bottom + pad + 12;
      const cardHeight = 180; // estimated
      card.style.position = 'fixed';
      card.style.left = Math.max(8, rect.left - pad) + 'px';
      if (spotlightBottom + cardHeight > window.innerHeight) {
        card.style.top = (rect.top - pad - cardHeight - 12) + 'px';
      } else {
        card.style.top = spotlightBottom + 'px';
      }
      return;
    }
  }

  // Target not found or zero dimensions — hide spotlight and center the card
  if (spotlight) spotlight.style.display = 'none';
  if (card) {
    card.style.position = 'fixed';
    card.style.top = '50%';
    card.style.left = '50%';
    card.style.transform = 'translate(-50%, -50%)';
  }
}

async function finishWalkthrough() {
  const overlay = document.getElementById('walkthrough-overlay');
  if (overlay) overlay.style.display = 'none';

  if (typeof appSettings !== 'undefined' && appSettings) {
    appSettings.walkthrough_completed = true;
  }

  try {
    await invoke('save_settings', { settings: appSettings });
  } catch (e) {
    console.error('Failed to save walkthrough_completed:', e);
  }
}
