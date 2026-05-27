// Entry point — imports all modules, wires events, bootstraps the app.
import { invoke, listen } from './core/tauri.js';
import * as state from './core/state.js';
import { escapeHtml } from './core/utils.js';
import { on, emit } from './core/events.js';
import { trace } from './ignore-trace.js';
import { checkForUpdates } from './updater.js';

// UI
import './ui/confirm-modal.js';
import { showToast } from './ui/toast.js';
import { ViewManager } from './ui/view-manager.js';

// Settings
import { loadSettings, switchSettingsTab, initSettingsListeners } from './settings/settings.js';
import { initTranscriptionSettings } from './settings/transcription.js';
import { updatePermissionStatus, initPermissions } from './settings/permissions.js';
import { initModelVersion, refreshModelVersion } from './settings/model-version.js';

// Recording
import './recording/timer.js';
import { startWaveformAnimation, stopWaveformAnimation } from './recording/waveform.js';
import './recording/live-transcript.js';
import { toggleRecording, startRecording, setRecordingUI } from './recording/controls.js';
import { loadRecordings, renderRecordingsList } from './recording/list.js';
// renderRecordingsList is invoked from transcription_progress handler when
// only the "Transcribing…" status changes — no need to refetch all metadata.
import { showDetailView, hideDetailView } from './recording/detail.js';
import { autoTranscribeAndExecute } from './recording/auto-execute.js';

// Pipeline
import { loadPipelineDefs } from './pipeline/defs-list.js';
import { renderPipelineFlowHTML } from './pipeline/flow-renderer.js';
import { renderPipelineChips, startRecordingWithPipeline } from './pipeline/chips.js';
import { allPipelineDefs } from './pipeline/state.js';

// Prompts
import { loadPromptTemplates, initPromptTemplates } from './prompts/templates.js';

// Connections (unified flat-list — replaces the legacy Models + Integrations tabs).
import { initConnectionsTab } from './connections/index.js';

// Health
import { initHealthCheck, scheduleAudit } from './health/init.js';

// Expose globals needed by other modules and onclick handlers
window.showDetailView = showDetailView;
window.escapeHtml = escapeHtml;
window.__nbpLoadPipelineDefs = loadPipelineDefs;
window.__nbpRenderPipelineFlowHTML = renderPipelineFlowHTML;
window.__nbpLoadPromptTemplates = loadPromptTemplates;
window.__nbpSwitchSettingsTab = switchSettingsTab;

// Wire record button
const recordToggleBtn = document.getElementById('record-toggle-btn');
if (recordToggleBtn) recordToggleBtn.addEventListener('click', toggleRecording);

// Wire cross-module events
on('tab:pipelines', () => loadPipelineDefs());
on('tab:prompts', () => loadPromptTemplates());
on('recording:showDetail', (id) => showDetailView(id));
on('recording:hideDetail', () => hideDetailView());
on('recordings:reload', async () => { await loadRecordings(); renderRecordingsList(); });
on('pipelines:renderChips', () => renderPipelineChips());

// Load templates for detail view
async function loadTemplates() {
  const templateSelect = document.getElementById('template-select');
  if (!templateSelect) return;
  try {
    const templates = await invoke('list_templates');
    templateSelect.innerHTML = '<option value="">Select template...</option>' +
      templates.map(t => `<option value="${escapeHtml(t.name)}">${escapeHtml(t.description)}</option>`).join('');
  } catch (err) { console.error('Failed to load templates:', err); }
}

// ===== INIT =====
async function init() {
  await loadSettings();
  await loadRecordings();
  await loadPipelineDefs();
  renderRecordingsList();
  await loadTemplates();
  await loadPromptTemplates();

  try {
    const version = await invoke('get_app_version');
    const versionEl = document.getElementById('app-version');
    if (versionEl) versionEl.textContent = `v${version} `;
  } catch (err) { console.error('Failed to fetch version:', err); }

  // Background update check — silent on failure / no-newer-version.
  checkForUpdates();

  // ASR model version check for the active engine (server-side throttled to
  // daily; the banner only appears on a real, non-dismissed signal).
  refreshModelVersion(false);

  await updatePermissionStatus();

  // System audio warnings
  listen('recording_warning', (event) => {
    console.warn('Recording warning:', event.payload);
    showToast(event.payload, 'warning');
  });

  // Tray menu: start recording with preselected pipeline
  listen('tray-start-pipeline', async (event) => {
    const pipelineName = event.payload;
    if (!state.isRecording && !state.isRecordingBusy) {
      await startRecordingWithPipeline(pipelineName);
    }
  });

  // Tray menu: "New Record" — clean start, no pipeline preselection.
  // User picks pipelines later via chip bar (or doesn't).
  listen('tray-record-new', async () => {
    if (!state.isRecording && !state.isRecordingBusy) {
      await startRecording();
    }
  });

  // Auto-start recording when a call is detected
  listen('call-detected', async () => {
    if (!state.isRecording && !state.isRecordingBusy) {
      await startRecording();
    }
  });

  // Auto-transcribe + auto-execute on recording completion. Transcription is
  // always on (the toggle was removed) — FluidAudio runs on-device so it never
  // fails offline.
  listen('recording_complete', async (event) => {
    const recordingId = event.payload;
    trace('JS recording_complete:', recordingId);
    state.setIsRecording(false);
    setRecordingUI(false);
    stopWaveformAnimation();
    await loadRecordings();
    if (state.selectedRecordingId === recordingId) showDetailView(recordingId);

    // Pipelines to run: explicitly-assigned (pendingAutoExec) plus any pipeline
    // flagged auto_run — those fire after every recording. Deduped, assigned
    // ones first so their order is preserved.
    const assigned = state.pendingAutoExec.get(recordingId) || [];
    const autoRun = (allPipelineDefs || []).filter((p) => p.auto_run).map((p) => p.name);
    const pipelines = [...new Set([...assigned, ...autoRun])];
    state.pendingAutoExec.delete(recordingId);
    autoTranscribeAndExecute(recordingId, pipelines);
  });

  // Call-detector auto-started a recording. The main-window list refreshes
  // and we mirror server-side is_recording so the main Record button reflects
  // the running state. Default pipeline (if any) is queued for auto-execute
  // after recording_complete fires — same pendingAutoExec pathway the manual
  // flow uses on stop.
  listen('recording_started', async (event) => {
    const { id, pipelines, source } = event.payload || {};
    trace('JS recording_started:', { id, pipelines, source });
    if (!id) return;
    state.setIsRecording(true);
    // Reflect the active recording on the global top-bar control — works for
    // auto/call recordings too, not just the manual flow (controls.js).
    setRecordingUI(true);
    startWaveformAnimation();
    await loadRecordings();
    if (Array.isArray(pipelines) && pipelines.length > 0) {
      state.pendingAutoExec.set(id, pipelines);
    }
  });

  // Recording was discarded server-side (duration < auto_discard_seconds).
  // The row from the optimistic `recording_started` refresh needs to go;
  // recording_complete never fires for discarded sessions so this is the
  // only signal we get.
  listen('recording_discarded', async (event) => {
    const recordingId = typeof event.payload === 'string' ? event.payload : null;
    trace('JS recording_discarded:', recordingId);
    state.setIsRecording(false);
    setRecordingUI(false);
    stopWaveformAnimation();
    if (recordingId) state.pendingAutoExec.delete(recordingId);
    await loadRecordings();
  });

  // Track in-flight transcriptions so the recordings list can render a
  // "Transcribing…" status on the affected row instead of just a duration.
  // detail.js has its own per-recording progress bar; this is purely for
  // list visibility. On 'Done' we refresh the list to pick up the freshly-
  // written `transcript_preview`.
  listen('transcription_progress', async (event) => {
    const { recording_id, stage } = event.payload || {};
    if (!recording_id) return;
    if (stage === 'Done') {
      state.transcribingIds.delete(recording_id);
      await loadRecordings();
    } else {
      if (!state.transcribingIds.has(recording_id)) {
        state.transcribingIds.add(recording_id);
        renderRecordingsList();
      }
    }
  });

  // Show onboarding if never completed
  const onboardingOverlay = document.getElementById('onboarding-overlay');
  if (onboardingOverlay && !state.appSettings.onboarding_completed) {
    onboardingOverlay.style.display = 'flex';
  }
}

async function bootstrapApp() {
  // Wire all event listeners
  initSettingsListeners();
  initTranscriptionSettings();
  initPermissions();
  // controls.js and detail.js wire their own event listeners on import
  initPromptTemplates();
  initConnectionsTab();
  initModelVersion();

  await init().catch(e => console.error('Init failed:', e));
  initHealthCheck();
  scheduleAudit(state.appSettings);
}

if (document.readyState === 'complete') {
  setTimeout(bootstrapApp, 0);
} else {
  window.addEventListener('load', bootstrapApp, { once: true });
}
