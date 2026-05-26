import { invoke } from '../core/tauri.js';
import * as state from '../core/state.js';
import { setAllRecordings } from '../core/state.js';
import { escapeHtml, formatDuration, getDuration } from '../core/utils.js';
import { emit } from '../core/events.js';
import { trace } from '../ignore-trace.js';
import { showConfirm } from '../ui/confirm-modal.js';
import { showToast } from '../ui/toast.js';
import { allPipelineDefs } from '../pipeline/state.js';
import { renderPipelineFlowHTML } from '../pipeline/flow-renderer.js';

// Stable per-day key for grouping recordings into date sections.
function dateSectionKey(createdAt) {
  const d = new Date(createdAt);
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

// Human label for a date section: "Today" / "Yesterday" / "MAY 15, 2026".
function dateSectionLabel(createdAt) {
  const d = new Date(createdAt);
  const startOfDay = (x) => new Date(x.getFullYear(), x.getMonth(), x.getDate());
  const diffDays = Math.round((startOfDay(new Date()) - startOfDay(d)) / 86400000);
  if (diffDays === 0) return 'Today';
  if (diffDays === 1) return 'Yesterday';
  return d
    .toLocaleDateString(undefined, { month: 'long', day: 'numeric', year: 'numeric' })
    .toUpperCase();
}

// key → { element, signature } — preserves DOM nodes across polls so scroll
// and selection survive when nothing (or only some nodes) changed. Keys are
// `sec:<dayKey>` for section headers and the recording id for rows.
const renderedNodes = new Map();

// Live timer for any row in `status: "recording"`. Ticks once per second
// and patches just the meta span of those rows — avoids full list re-render.
// Started/stopped by `syncActiveRecordingTimers()` based on whether any
// recording is currently in the active state.
let activeTimerInterval = null;

function formatElapsedSince(createdAt) {
  const start = new Date(createdAt).getTime();
  if (!Number.isFinite(start)) return '';
  const elapsedSec = Math.max(0, Math.floor((Date.now() - start) / 1000));
  const h = Math.floor(elapsedSec / 3600);
  const m = Math.floor((elapsedSec % 3600) / 60);
  const s = elapsedSec % 60;
  const pad = (n) => String(n).padStart(2, '0');
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

function recordingStatusHtml(rec) {
  const elapsed = formatElapsedSince(rec.created_at);
  return `<span class="recording-text">Recording ${elapsed}</span>`;
}

function tickActiveRecordingRows() {
  for (const rec of state.allRecordings) {
    if (rec.status !== 'recording') continue;
    const row = document.querySelector(`.recording-item[data-id="${CSS.escape(rec.id)}"]`);
    if (!row) continue;
    const metaSpans = row.querySelectorAll('.recording-meta > span');
    if (metaSpans.length === 0) continue;
    metaSpans[metaSpans.length - 1].outerHTML = recordingStatusHtml(rec);
  }
}

function syncActiveRecordingTimers() {
  const anyActive = state.allRecordings.some((r) => r.status === 'recording');
  if (anyActive && activeTimerInterval == null) {
    activeTimerInterval = setInterval(tickActiveRecordingRows, 1000);
  } else if (!anyActive && activeTimerInterval != null) {
    clearInterval(activeTimerInterval);
    activeTimerInterval = null;
  }
}

let __loadSeq = 0;
export async function loadRecordings() {
  const seq = ++__loadSeq;
  try {
    const t0 = performance.now();
    const recordings = await invoke('list_recordings');
    const dt = (performance.now() - t0).toFixed(1);
    const active = (recordings || [])
      .filter(r => r.status === 'recording' || r.status === 'processing')
      .map(r => `${r.id.slice(0,8)}=${r.status}`);
    trace(`JS loadRecordings#${seq} resolved in ${dt}ms, n=${(recordings||[]).length}, active=`, active);
    setAllRecordings(recordings || []);
    renderRecordingsList();
  } catch (error) {
    trace('JS loadRecordings failed:', String(error));
  }
}

// Neutral, unobtrusive fallback shown while an icon resolves or when the
// bundle id can't be resolved to an installed app (muted rounded-square glyph).
const DEFAULT_APP_ICON_SVG =
  '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="4"/></svg>';

// NBP's own mark for manual recordings — inline waveform so it uses
// currentColor and stays visible in both light and dark themes (the app icon
// PNG is black-on-transparent and would vanish on a dark background).
const NBP_WAVEFORM_SVG =
  '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="4" y1="10" x2="4" y2="14"/><line x1="8.5" y1="5" x2="8.5" y2="19"/><line x1="13" y1="2.5" x2="13" y2="21.5"/><line x1="17.5" y1="7" x2="17.5" y2="17"/><line x1="21" y1="10.5" x2="21" y2="13.5"/></svg>';

// bundle_id → data URL (or null when unresolved). Avoids re-invoking the Rust
// command on every list re-render; the backend also caches on disk.
const appIconCache = new Map();

function applyResolvedIcon(el, url) {
  if (url) {
    el.innerHTML = `<img src="${url}" alt="" width="16" height="16" />`;
    el.classList.add('app-icon-resolved');
  }
}

// Fill in app icons after the rows are in the DOM. Marked rows are skipped so
// repeated renders don't re-process; rebuilt rows (new signature) come back
// without the marker and get re-hydrated from the cache (cheap, no FFI).
async function hydrateAppIcons() {
  const els = document.querySelectorAll('.app-icon[data-bundle]:not([data-hydrated])');
  for (const el of els) {
    el.dataset.hydrated = '1';
    const bundle = el.dataset.bundle;
    if (!bundle) continue;
    if (appIconCache.has(bundle)) {
      applyResolvedIcon(el, appIconCache.get(bundle));
      continue;
    }
    try {
      const url = await invoke('get_app_icon', { bundleId: bundle });
      appIconCache.set(bundle, url || null);
      applyResolvedIcon(el, url || null);
    } catch {
      appIconCache.set(bundle, null);
    }
  }
}

function buildRowHtml(rec) {
  const isProcessing = rec.status === 'processing';
  const isRecordingStatus = rec.status === 'recording';
  const isCurrentlyRecording = state.isRecording && state.selectedRecordingId === rec.id;
  const isTranscribing = state.transcribingIds && state.transcribingIds.has(rec.id);

  // Status text takes priority over duration when an action is in flight.
  // Order: recording > processing > transcribing > duration (idle).
  let metaText;
  if (isRecordingStatus) {
    metaText = recordingStatusHtml(rec);
  } else if (isProcessing) {
    metaText = '<span style="color:var(--accent)">Processing…</span>';
  } else if (isTranscribing) {
    metaText = '<span style="color:var(--accent)">Transcribing…</span>';
  } else {
    metaText = formatDuration(getDuration(rec));
  }

  // Transcript preview — populated by `transcription` after the run lands.
  // Tiny 1-3 line snippet so the user can tell what the meeting was about
  // without opening the recording. Falls back to absent for un-transcribed
  // or empty-transcript recordings.
  const previewHtml = rec.transcript_preview
    ? `<div class="recording-preview">${escapeHtml(rec.transcript_preview)}</div>`
    : '';

  const hasIssues = rec.health && rec.health.status !== 'ok';
  const healthIcon = hasIssues
    ? '<span class="health-warning" title="Issues occurred during recording"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--warning, #f59e0b)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg></span>'
    : '';

  const safeTitle = escapeHtml(rec.title || 'Untitled');
  const safeId = escapeHtml(rec.id);

  // App icon: call recordings resolve their app's icon async from the bundle
  // id; manual recordings (New Record / pipeline) have no bundle id and show
  // our own app icon instead.
  const appIconHtml = rec.app_bundle_id
    ? `<span class="app-icon" data-bundle="${escapeHtml(rec.app_bundle_id)}">${DEFAULT_APP_ICON_SVG}</span>`
    : `<span class="app-icon app-icon-resolved">${NBP_WAVEFORM_SVG}</span>`;

  // Pipeline tags with step chips
  const pipelineTags = (rec.pipelines || []).map(p => {
    const statusClass = p.status === 'Done' ? 'tag-done' : p.status === 'Partial' ? 'tag-partial' : p.status === 'Running' ? 'tag-running' : 'tag-waiting';
    const def = allPipelineDefs ? allPipelineDefs.find(d => d.name === p.name) : null;
    const flowHtml = def && def.steps && def.steps.length > 0
      ? renderPipelineFlowHTML(def.steps, { compact: true })
      : '';
    return `<div class="recording-pipeline-entry ${statusClass}"><span class="recording-pipeline-name">${escapeHtml(p.name)}</span>${flowHtml}</div>`;
  }).join('');

  const deleteDisabled = isCurrentlyRecording || isProcessing;
  const deleteBtnHtml = deleteDisabled
    ? ''
    : `<button class="recording-item-delete" data-id="${safeId}" title="Delete recording"><span class="icon-trash"></span></button>`;

  // Copy-transcript shown only when there's a transcript to copy (preview is
  // the cheapest signal). Click fetches the full text via get_transcript so we
  // copy what the user would see in the detail view, not just the snippet.
  const copyBtnHtml = rec.transcript_preview
    ? `<button class="recording-item-copy" data-id="${safeId}" title="Copy transcript"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg></button>`
    : '';

  return `<div class="recording-item ${isCurrentlyRecording ? 'recording-active' : ''}" data-id="${safeId}" onclick="showDetailView(this.dataset.id)">
        <div class="recording-item-header">
          <div class="recording-title">${healthIcon}${appIconHtml}${safeTitle}${isCurrentlyRecording ? ' <span style="color:var(--accent)">●</span>' : ''}</div>
          <div class="recording-meta">
            <span>${metaText}</span>
          </div>
        </div>
        ${previewHtml}
        ${pipelineTags ? `<div class="recording-pipeline-tags">${pipelineTags}</div>` : ''}
        ${copyBtnHtml}
        ${deleteBtnHtml}
      </div>`;
}

function htmlToElement(html) {
  const tmp = document.createElement('div');
  tmp.innerHTML = html.trim();
  return tmp.firstElementChild;
}

function wireRowCopy(rowEl) {
  const btn = rowEl.querySelector('.recording-item-copy');
  if (!btn) return;
  btn.addEventListener('click', async (e) => {
    e.stopPropagation(); // don't open the detail view on copy click
    const recordingId = btn.dataset.id;
    try {
      const text = await invoke('get_transcript', { recordingId });
      const trimmed = (text || '').trim();
      if (!trimmed) {
        showToast('No transcript to copy', 'info');
        return;
      }
      await navigator.clipboard.writeText(trimmed);
      showToast('Transcript copied', 'success');
    } catch (err) {
      console.error('copy transcript failed:', err);
      showToast('Copy failed', 'error');
    }
  });
}

function wireRowDelete(rowEl) {
  const btn = rowEl.querySelector('.recording-item-delete');
  if (!btn) return;
  btn.addEventListener('click', async (e) => {
    e.stopPropagation();
    const recordingId = btn.dataset.id;
    const ok = await showConfirm('Delete Recording?', 'This action cannot be undone.');
    if (!ok) return;
    try {
      await invoke('delete_recording', { recordingId });
      if (state.selectedRecordingId === recordingId) emit('recording:hideDetail');
      await loadRecordings();
    } catch (err) {
      console.error('Delete failed:', err);
      if (err && typeof err === 'string' && err.includes('finalized')) {
        showToast('Recording is still being finalized. Please wait a moment and try again.', 'info');
      } else {
        showToast('Delete failed: ' + err, 'error');
      }
    }
  });
}

export function renderRecordingsList() {
  const recordingsListEl = document.getElementById('recordings-list');
  const emptyStateEl = document.getElementById('empty-state');
  if (!recordingsListEl) return;

  if (state.allRecordings.length === 0) {
    if (renderedNodes.size > 0 || recordingsListEl.firstChild) {
      recordingsListEl.innerHTML = '';
      renderedNodes.clear();
    }
    if (emptyStateEl) emptyStateEl.style.display = 'block';
    return;
  }
  if (emptyStateEl) emptyStateEl.style.display = 'none';

  // Build the ordered node list: a date-section header before each new day,
  // then the rows for that day. `allRecordings` is already sorted newest-first
  // so days fall out in order.
  const nodes = [];
  let lastSectionKey = null;
  for (const rec of state.allRecordings) {
    const secKey = dateSectionKey(rec.created_at);
    if (secKey !== lastSectionKey) {
      lastSectionKey = secKey;
      nodes.push({
        key: `sec:${secKey}`,
        html: `<div class="recordings-section-header">${escapeHtml(dateSectionLabel(rec.created_at))}</div>`,
        isRow: false,
      });
    }
    nodes.push({ key: rec.id, html: buildRowHtml(rec), isRow: true });
  }

  // Remove nodes no longer present (rows and now-empty section headers).
  const currentKeys = new Set(nodes.map((n) => n.key));
  for (const [key, info] of renderedNodes) {
    if (!currentKeys.has(key)) {
      info.element.remove();
      renderedNodes.delete(key);
    }
  }

  // Walk nodes in order; insert/update/move per-node as needed.
  let prevNode = null;
  for (const node of nodes) {
    let info = renderedNodes.get(node.key);
    if (info) {
      if (info.signature !== node.html) {
        const newEl = htmlToElement(node.html);
        info.element.replaceWith(newEl);
        if (node.isRow) { wireRowDelete(newEl); wireRowCopy(newEl); }
        info.element = newEl;
        info.signature = node.html;
      }
    } else {
      const newEl = htmlToElement(node.html);
      info = { element: newEl, signature: node.html };
      renderedNodes.set(node.key, info);
      if (node.isRow) { wireRowDelete(newEl); wireRowCopy(newEl); }
    }

    const expectedAtPos = prevNode ? prevNode.nextSibling : recordingsListEl.firstChild;
    if (info.element !== expectedAtPos) {
      recordingsListEl.insertBefore(info.element, expectedAtPos);
    }
    prevNode = info.element;
  }

  // Drive the per-second tick for any in-progress recording rows so the
  // "Recording 0:42" stays current. Started here (not on every loadRecordings
  // call) so it reflects whatever's in state.allRecordings right now and
  // self-stops when the last active recording finalizes.
  syncActiveRecordingTimers();

  // Resolve + swap in real app icons once rows are in the DOM.
  hydrateAppIcons();
}
