// People editor: voices collected reactively by diarization. Naming a voice
// labels its whole history (backend retro-applies) and all future recordings.
import { invoke, listen } from '../core/tauri.js';
import { escapeHtml } from '../core/utils.js';
import { showToast } from '../ui/toast.js';
import { showConfirm } from '../ui/confirm-modal.js';

/// Keep a <datalist> of known people for name inputs (rename suggestions).
export function ensureNamesDatalist(profiles) {
  let dl = document.getElementById('people-names');
  if (!dl) {
    dl = document.createElement('datalist');
    dl.id = 'people-names';
    document.body.appendChild(dl);
  }
  const names = [...new Set(profiles.filter(p => p.name).map(p => p.name))];
  dl.innerHTML = names.map(n => `<option value="${escapeHtml(n)}"></option>`).join('');
}

/// Commit a typed name for a speaker: if it matches an existing person, ask —
/// picking them is an EXPLICIT identity merge (assign_speaker_profile);
/// otherwise it's a plain label (rename_speaker; duplicates allowed).
export async function commitSpeakerName(recordingId, speaker, name) {
  let profiles = [];
  try { profiles = (await invoke('list_speaker_profiles')) || []; } catch { profiles = []; }
  const match = profiles.find(p => p.name && p.name.toLowerCase() === name.toLowerCase());
  if (match) {
    const same = await showConfirm(
      `Same person as “${match.name}”?`,
      `They appear in ${match.recordings} recording${match.recordings === 1 ? '' : 's'}. Linking merges this voice into that person and labels their whole history.`,
      'Yes, same person'
    );
    if (same) {
      await invoke('assign_speaker_profile', { recordingId, speaker, profileUid: match.uid });
      return;
    }
  }
  await invoke('rename_speaker', { recordingId, speaker, name });
}

// Reactive: refresh the list/badge whenever a diarization job finishes while
// the settings view is open.
let listenerStarted = false;
function ensurePeopleListener() {
  if (listenerStarted) return;
  listenerStarted = true;
  listen('diarization_progress', (event) => {
    const p = event.payload || {};
    if (p.status === 'done') renderPeople();
  });
}

const MIN_SECONDS = 60; // show candidates with enough voice to judge…
const MIN_RECORDINGS = 2; // …or seen in more than one recording

function fmtDur(s) {
  const m = Math.round(s / 60);
  return m >= 1 ? `${m} min` : `${Math.round(s)}s`;
}

export async function renderPeople() {
  ensurePeopleListener();
  const list = document.getElementById('people-list');
  const badge = document.getElementById('people-badge');
  if (!list) return;

  let profiles = [];
  try { profiles = (await invoke('list_speaker_profiles')) || []; } catch { profiles = []; }
  ensureNamesDatalist(profiles);

  const named = profiles.filter(p => p.name);
  const candidates = profiles.filter(
    p => !p.name && (p.total_seconds >= MIN_SECONDS || p.recordings >= MIN_RECORDINGS)
  );

  if (badge) {
    if (candidates.length > 0) {
      badge.textContent = candidates.length;
      badge.style.display = '';
    } else {
      badge.style.display = 'none';
    }
  }

  if (named.length === 0 && candidates.length === 0) {
    list.innerHTML = '<p class="settings-hint">No voices yet — diarize a recording to start collecting.</p>';
    return;
  }

  const row = (p, isCandidate) => `
    <div class="people-row" data-uid="${p.uid}">
      ${p.has_sample ? `<button class="people-play mini-action-btn" data-rec="${escapeHtml(p.sample_recording_id)}" data-spk="${p.sample_local_id}" title="Listen">&#9654;</button>` : ''}
      <div class="people-info">
        <div class="people-title">${p.name ? escapeHtml(p.name) : 'Unknown voice'} <span class="people-meta">${fmtDur(p.total_seconds)} · ${p.recordings} rec</span></div>
        ${p.preview ? `<div class="people-quote">«${escapeHtml(p.preview)}»</div>` : ''}
      </div>
      ${isCandidate
        ? `<input class="people-name-input" type="text" list="people-names" placeholder="Name…" data-rec="${escapeHtml(p.sample_recording_id)}" data-spk="${p.sample_local_id}" />`
        : ''}
    </div>`;

  list.innerHTML =
    candidates.map(p => row(p, true)).join('') +
    (named.length ? `<div class="people-known-sep">Known people</div>` + named.map(p => row(p, false)).join('') : '');

  list.querySelectorAll('.people-play').forEach(btn => {
    btn.addEventListener('click', async () => {
      try {
        const b64 = await invoke('get_voice_sample', {
          recordingId: btn.dataset.rec,
          speaker: parseInt(btn.dataset.spk, 10),
        });
        new Audio('data:audio/wav;base64,' + b64).play();
      } catch (e) {
        showToast('No sample: ' + e, 'error');
      }
    });
  });

  list.querySelectorAll('.people-name-input').forEach(input => {
    input.addEventListener('keydown', async (ev) => {
      if (ev.key !== 'Enter') return;
      const name = input.value.trim();
      if (!name) return;
      input.disabled = true;
      try {
        // Existing person → explicit merge (with confirm); new name → label.
        await commitSpeakerName(input.dataset.rec, parseInt(input.dataset.spk, 10), name);
        showToast(`Named: ${name}`, 'success');
        renderPeople();
      } catch (e) {
        showToast('Failed: ' + e, 'error');
        input.disabled = false;
      }
    });
  });
}
