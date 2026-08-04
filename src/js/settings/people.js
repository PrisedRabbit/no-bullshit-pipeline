// People editor: voices collected reactively by diarization. Naming a voice
// labels its whole history (backend retro-applies) and all future recordings.
import { invoke } from '../core/tauri.js';
import { escapeHtml } from '../core/utils.js';
import { showToast } from '../ui/toast.js';

const MIN_SECONDS = 60; // show candidates with enough voice to judge…
const MIN_RECORDINGS = 2; // …or seen in more than one recording

function fmtDur(s) {
  const m = Math.round(s / 60);
  return m >= 1 ? `${m} min` : `${Math.round(s)}s`;
}

export async function renderPeople() {
  const list = document.getElementById('people-list');
  const badge = document.getElementById('people-badge');
  if (!list) return;

  let profiles = [];
  try { profiles = (await invoke('list_speaker_profiles')) || []; } catch { profiles = []; }

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
      <button class="people-play mini-action-btn" data-rec="${escapeHtml(p.sample_recording_id)}" data-spk="${p.sample_local_id}" title="Listen">&#9654;</button>
      <div class="people-info">
        <div class="people-title">${p.name ? escapeHtml(p.name) : 'Unknown voice'} <span class="people-meta">${fmtDur(p.total_seconds)} · ${p.recordings} rec</span></div>
        ${p.preview ? `<div class="people-quote">«${escapeHtml(p.preview)}»</div>` : ''}
      </div>
      ${isCandidate
        ? `<input class="people-name-input" type="text" placeholder="Name…" data-rec="${escapeHtml(p.sample_recording_id)}" data-spk="${p.sample_local_id}" />`
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
        // rename_speaker on the best appearance names the profile and
        // retro-applies across every recording this voice appeared in.
        await invoke('rename_speaker', {
          recordingId: input.dataset.rec,
          speaker: parseInt(input.dataset.spk, 10),
          name,
        });
        showToast(`Named: ${name}`, 'success');
        renderPeople();
      } catch (e) {
        showToast('Failed: ' + e, 'error');
        input.disabled = false;
      }
    });
  });
}
