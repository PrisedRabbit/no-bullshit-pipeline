// Shared pipeline flow renderer — used by builder preview and recording cards.
// Renders the linear chain `Transcript → step1 → step2 → ...` as type chips.
// Keyed on `step.step_type` (cli_agent | shell), with the step's inline config
// (CLI name / model, or shell binary) as the sub-label.

import { escapeHtml } from '../core/utils.js';
import { CONNECTOR_META } from './constants.js';

const MIC_SVG = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" y1="19" x2="12" y2="23"/><line x1="8" y1="23" x2="16" y2="23"/></svg>`;

// Friendly CLI display names for the chip label.
const CLI_NAMES = { claude: 'Claude Code', codex: 'Codex', opencode: 'OpenCode', agy: 'Antigravity' };

// Main chip label = the step TYPE (steps carry no user-facing name). For CLI
// this is which CLI (Claude Code / Codex / …); Shell and Save are fixed.
export function stepDisplayLabel(step) {
  const cfg = step.config || {};
  const type = step.step_type || step.connection_type || '';
  if (type === 'cli_agent') return CLI_NAMES[cfg.cli] || cfg.cli || 'CLI agent';
  if (type === 'shell') return 'Shell';
  if (type === 'save_local') return 'Save to folder';
  return step.name || 'Step';
}

// Short sub-label with the distinguishing detail, so two steps of the same
// type disambiguate at a glance (CLI model, shell binary, save folder).
export function stepSubLabel(step) {
  const cfg = step.config || {};
  const type = step.step_type || step.connection_type || '';
  if (type === 'cli_agent') {
    return cfg.model || ''; // CLI itself is the main label now
  }
  if (type === 'shell') {
    const sh = (cfg.shell || '').split('/').pop();
    return sh && sh !== 'bash' ? sh : '';
  }
  if (type === 'save_local') {
    const f = cfg.folder_path || '';
    return f ? (f.replace(/\/+$/, '').split('/').pop() || f) : '';
  }
  return '';
}

/**
 * Render a pipeline flow as HTML chips.
 * @param {Array} steps - pipeline steps (shape: { name, step_type, template, config })
 * @param {Object} opts - { compact: bool, statuses: { stepName: status } }
 */
export function renderPipelineFlowHTML(steps, opts = {}) {
  const { compact = false, statuses = {} } = opts;
  const arrow = `<div class="pflow-arrow">›</div>`;

  let html = `<div class="pflow-chip pflow-chip--source" title="Transcript"><div class="pflow-chip-icon" style="background:var(--accent-soft);color:var(--accent);">${MIC_SVG}</div>${compact ? '' : '<span class="pflow-chip-label">Transcript</span>'}</div>`;

  for (const step of (steps || [])) {
    html += arrow;
    // Defensive: pipelines saved in older shapes (or new ones with corrupted
    // data) might be missing step_type. Render a placeholder chip so the whole
    // list doesn't blow up — the user can edit the step to fix it.
    const typeKey = step.step_type || step.connection_type || '';
    const meta = CONNECTOR_META[typeKey] || {
      abbr: typeKey ? typeKey.substring(0, 2).toUpperCase() : '??',
      textColor: 'var(--text-primary)',
      bgColor: 'var(--bg-input)',
    };
    const bg = meta.bgColor;
    const fg = meta.textColor;
    const iconContent = meta.svg
      ? meta.svg
      : `<span style="font-size:7px;font-weight:800;color:${fg};">${meta.abbr}</span>`;

    const st = statuses[step.name];
    const stClass = st ? ` pflow-chip--${st}` : '';
    const safeName = escapeHtml(stepDisplayLabel(step));
    const subLabel = stepSubLabel(step);
    const subHtml = !compact && subLabel
      ? `<span class="pflow-chip-sub">${escapeHtml(subLabel)}</span>`
      : '';

    html += `<div class="pflow-chip${stClass}" title="${safeName}"><div class="pflow-chip-icon" style="background:${bg};color:${fg};">${iconContent}</div>${compact ? '' : `<div class="pflow-chip-label-group"><span class="pflow-chip-label">${safeName}</span>${subHtml}</div>`}</div>`;
  }

  return `<div class="pflow${compact ? ' pflow--compact' : ''}">${html}</div>`;
}
