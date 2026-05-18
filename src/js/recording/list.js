import { invoke } from '../core/tauri.js';
import * as state from '../core/state.js';
import { setAllRecordings } from '../core/state.js';
import { escapeHtml, formatDuration, getDuration } from '../core/utils.js';
import { emit } from '../core/events.js';
import { showConfirm } from '../ui/confirm-modal.js';
import { showToast } from '../ui/toast.js';
import { allPipelineDefs } from '../pipeline/state.js';
import { renderPipelineFlowHTML } from '../pipeline/flow-renderer.js';

const dateOptions = {
  month: 'short',
  day: 'numeric',
  year: 'numeric',
  hour: 'numeric',
  minute: '2-digit'
};

// id → { element, signature } — preserves DOM nodes across polls so scroll
// and selection survive when nothing (or only some rows) changed.
const renderedRows = new Map();

export async function loadRecordings() {
  try {
    const recordings = await invoke('list_recordings');
    setAllRecordings(recordings || []);
    renderRecordingsList();
  } catch (error) {
    console.error('Failed to load recordings:', error);
  }
}

function buildRowHtml(rec) {
  const isProcessing = rec.status === 'processing';
  const isCurrentlyRecording = state.isRecording && state.selectedRecordingId === rec.id;
  const metaText = isProcessing
    ? '<span style="color:var(--accent)">Processing...</span>'
    : formatDuration(getDuration(rec));

  const hasIssues = rec.health && rec.health.status !== 'ok';
  const healthIcon = hasIssues
    ? '<span class="health-warning" title="Issues occurred during recording"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--warning, #f59e0b)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg></span>'
    : '';

  const safeTitle = escapeHtml(rec.title || 'Untitled');
  const safeId = escapeHtml(rec.id);

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

  return `<div class="recording-item ${isCurrentlyRecording ? 'recording-active' : ''}" data-id="${safeId}" onclick="showDetailView(this.dataset.id)">
        <div class="recording-item-header">
          <div class="recording-title">${healthIcon}${safeTitle}${isCurrentlyRecording ? ' <span style="color:var(--accent)">●</span>' : ''}</div>
          <div class="recording-meta">
            <span>${new Date(rec.created_at).toLocaleString(undefined, dateOptions)}</span>
            <span>·</span>
            <span>${metaText}</span>
          </div>
        </div>
        ${pipelineTags ? `<div class="recording-pipeline-tags">${pipelineTags}</div>` : ''}
        ${deleteBtnHtml}
      </div>`;
}

function htmlToElement(html) {
  const tmp = document.createElement('div');
  tmp.innerHTML = html.trim();
  return tmp.firstElementChild;
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
    if (renderedRows.size > 0 || recordingsListEl.firstChild) {
      recordingsListEl.innerHTML = '';
      renderedRows.clear();
    }
    if (emptyStateEl) emptyStateEl.style.display = 'block';
    return;
  }
  if (emptyStateEl) emptyStateEl.style.display = 'none';

  // Remove rows no longer present
  const currentIds = new Set();
  for (const rec of state.allRecordings) currentIds.add(rec.id);
  for (const [id, info] of renderedRows) {
    if (!currentIds.has(id)) {
      info.element.remove();
      renderedRows.delete(id);
    }
  }

  // Walk recordings in order; insert/update/move per-row as needed
  let prevNode = null;
  for (const rec of state.allRecordings) {
    const html = buildRowHtml(rec);
    let info = renderedRows.get(rec.id);
    if (info) {
      if (info.signature !== html) {
        const newEl = htmlToElement(html);
        info.element.replaceWith(newEl);
        wireRowDelete(newEl);
        info.element = newEl;
        info.signature = html;
      }
    } else {
      const newEl = htmlToElement(html);
      info = { element: newEl, signature: html };
      renderedRows.set(rec.id, info);
      wireRowDelete(newEl);
    }

    const expectedAtPos = prevNode ? prevNode.nextSibling : recordingsListEl.firstChild;
    if (info.element !== expectedAtPos) {
      recordingsListEl.insertBefore(info.element, expectedAtPos);
    }
    prevNode = info.element;
  }
}
