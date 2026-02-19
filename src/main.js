const { invoke } = window.__TAURI__.core;

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// ===== VIEW STATE MANAGER =====
const ViewManager = {
  closeAll() {
    document.body.classList.remove('detail-open', 'settings-open');
  },

  showRecordings() {
    this.closeAll();
  },

  showDetail() {
    this.closeAll();
    document.body.classList.add('detail-open');
  },

  showSettings() {
    this.closeAll();
    document.body.classList.add('settings-open');
  }
};

// ===== STATE =====
let timerInterval;
let startTime;
let isRecording = false;
let isRecordingBusy = false; // Guard against double-click during async start/stop

let allRecordings = [];
let selectedRecordingId = null;
let detailPipelineHandler = null; // Module-level ref to avoid stacking change listeners

let permissions = { mic: false, system_audio: false };
let appSettings = null;
let currentAssignedPipeline = null;
let pipelineProgressUnlisten = null;

// ===== DOM ELEMENTS =====
const statusIndicator = document.getElementById("status-indicator");
const timerDisplay = document.getElementById("timer");
const recordToggleBtn = document.getElementById("record-toggle-btn");

const recordingsListEl = document.getElementById("recordings-list");
const emptyStateEl = document.getElementById("empty-state");
const detailViewEl = document.getElementById("detail-view");
const appLayoutEl = document.querySelector(".app-layout");
const backBtn = document.getElementById("back-btn");

const detailTitleInput = document.getElementById("detail-title");
const detailMetaHeaderEl = document.getElementById("detail-meta-header");
const detailTranscriptEl = document.getElementById("transcript-content");
const detailStructuredEl = document.getElementById("structured-content");

const onboardingOverlay = document.getElementById("onboarding-overlay");
const requestMicBtn = document.getElementById("request-mic-btn");
const requestSysBtn = document.getElementById("request-sys-btn");
const onboardingContinueBtn = document.getElementById("onboarding-continue-btn");
const permissionWarning = document.getElementById("permission-warning");
const fixPermissionsBtn = document.getElementById("fix-permissions-btn");

const settingsViewEl = document.getElementById("settings-view");
const settingsBtn = document.getElementById("settings-btn");
const settingsBackBtn = document.getElementById("settings-back-btn");
const browseStorageBtn = document.getElementById("browse-storage-btn");
const saveSettingsBtn = document.getElementById("save-settings-btn");

const storagePathInput = document.getElementById("settings-storage-path");
const cleanupThresholdInput = document.getElementById("settings-cleanup-threshold");
const themeButtons = document.querySelectorAll(".theme-btn");

const detailControlsEl = document.getElementById("detail-controls");
const captureSectionEl = document.getElementById("capture-section");

// ===== TIMER =====
function formatTime(ms) {
  const totalSeconds = Math.floor(ms / 1000);
  const h = Math.floor(totalSeconds / 3600).toString().padStart(2, "0");
  const m = Math.floor((totalSeconds % 3600) / 60).toString().padStart(2, "0");
  const s = (totalSeconds % 60).toString().padStart(2, "0");
  return `${h}:${m}:${s} `;
}

function startTimer() {
  startTime = Date.now();
  timerInterval = setInterval(() => {
    const now = Date.now();
    timerDisplay.textContent = formatTime(now - startTime);

    if (isRecording && selectedRecordingId && detailViewEl.style.display !== 'none') {
      if (detailMetaHeaderEl) {
        detailMetaHeaderEl.textContent = `Recording... · ${timerDisplay.textContent}`;
      }
    }
  }, 100);
}

function stopTimer() {
  clearInterval(timerInterval);
  timerDisplay.textContent = "00:00:00";
}

// ===== RECORDING WAVEFORM (SPECTRUM STYLE) =====
let waveformInterval = null;
const NUM_BARS = 5;
let displayLevel = 0; // What we show (with slow decay)

function getWaveformCanvas() {
  return document.getElementById("recording-waveform-canvas");
}

function startWaveformAnimation() {
  const canvas = getWaveformCanvas();
  const ctx = canvas ? canvas.getContext("2d") : null;
  if (!canvas || !ctx) return;

  displayLevel = 0;

  waveformInterval = setInterval(async () => {
    try {
      const level = await invoke("get_audio_level");
      // Amplify input (RMS is naturally low)
      const amplified = Math.min(1.0, level * 6);

      // Instant attack, medium decay
      if (amplified > displayLevel) {
        // Jump up instantly
        displayLevel = amplified;
      } else {
        // Fall at medium speed (~0.5 sec from full)
        displayLevel = Math.max(0, displayLevel - 0.06);
      }

      drawSpectrum();
    } catch (e) {
      // Ignore errors
    }
  }, 30); // ~33fps for smoother animation
}

function stopWaveformAnimation() {
  if (waveformInterval) {
    clearInterval(waveformInterval);
    waveformInterval = null;
  }
  displayLevel = 0;
  const canvas = getWaveformCanvas();
  const ctx = canvas ? canvas.getContext("2d") : null;
  if (ctx && canvas) {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
  }
}

function drawSpectrum() {
  const canvas = getWaveformCanvas();
  const ctx = canvas ? canvas.getContext("2d") : null;
  if (!ctx || !canvas) return;

  const width = canvas.width;
  const height = canvas.height;
  const barWidth = Math.floor(width / NUM_BARS) - 2;
  const gap = 2;

  // Get computed accent color
  const accentColor = getComputedStyle(document.documentElement).getPropertyValue("--accent").trim() || "#a855f7";

  ctx.clearRect(0, 0, width, height);

  // Spectrum-like distribution: center bars taller, edges shorter
  const barMultipliers = [0.6, 0.9, 1.0, 0.9, 0.6];

  for (let i = 0; i < NUM_BARS; i++) {
    const multiplier = barMultipliers[i];
    const barLevel = displayLevel * multiplier;
    const barHeight = Math.max(3, barLevel * height * 0.9);
    const x = i * (barWidth + gap) + gap;
    const y = (height - barHeight) / 2;

    ctx.fillStyle = accentColor;
    ctx.fillRect(x, y, barWidth, barHeight);
  }
}

// ===== RECORDING CONTROLS =====
async function toggleRecording() {
  if (isRecordingBusy) return; // Prevent double-click during async operations

  // If we have a recording selected and not recording → play it
  if (selectedRecordingId && !isRecording) {
    try {
      await invoke('stop_audio'); // Stop any previous playback
      await invoke('play_audio', { recordingId: selectedRecordingId });
    } catch (err) {
      console.error('Playback error:', err);
    }
    return;
  }

  if (isRecording) {
    await stopRecording();
  } else {
    await startRecording();
  }
}

function setRecordingUI(recording) {
  if (recording) {
    if (statusIndicator) statusIndicator.className = "status-recording";
    document.body.classList.add("is-recording-active");
    if (recordToggleBtn) {
      recordToggleBtn.innerHTML = "⏹";
      recordToggleBtn.classList.add("is-active");
      recordToggleBtn.classList.remove("is-play-mode");
      recordToggleBtn.title = "Stop Recording";
    }
  } else {
    if (statusIndicator) statusIndicator.className = "status-idle";
    document.body.classList.remove("is-recording-active");
    updateMainButton();
  }
}

// Update main button based on context: play (when recording selected) or record (default)
function updateMainButton() {
  if (!recordToggleBtn) return;
  recordToggleBtn.classList.remove("is-active");

  if (selectedRecordingId && !isRecording) {
    // Show play button
    recordToggleBtn.innerHTML = '<svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>';
    recordToggleBtn.classList.add("is-play-mode");
    recordToggleBtn.title = "Play Recording";
  } else {
    // Show record button
    recordToggleBtn.innerHTML = "⏺";
    recordToggleBtn.classList.remove("is-play-mode");
    recordToggleBtn.title = "Start Recording";
  }
}

async function startRecording() {
  isRecordingBusy = true;
  ViewManager.showRecordings();

  const saveMixOnly = appSettings?.save_mix_only !== false;
  console.log('DEBUG: Starting recording with saveMixOnly =', saveMixOnly);
  try {
    const metadata = await invoke("start_recording", { saveMixOnly: saveMixOnly });
    isRecording = true;
    setRecordingUI(true);

    // Auto-assign default pipeline if set and no chip was clicked
    if (appSettings?.default_pipeline && currentAssignedPipeline === null) {
      try {
        currentAssignedPipeline = appSettings.default_pipeline;
        await invoke('assign_pipeline', { recordingId: metadata.id, pipelineName: appSettings.default_pipeline });
      } catch (e) {
        console.error('Failed to auto-assign default pipeline:', e);
        currentAssignedPipeline = null;
      }
    }

    await loadRecordings();
    startTimer();
    startWaveformAnimation();
    showDetailView(metadata.id);

  } catch (error) {
    // Revert all state on failure
    isRecording = false;
    stopTimer();
    stopWaveformAnimation();
    setRecordingUI(false);
    console.error("Failed to start recording:", error);
    alert("Failed to start: " + error);
  } finally {
    isRecordingBusy = false;
  }
}

async function stopRecording() {
  isRecordingBusy = true;
  try {
    const currentId = selectedRecordingId;
    const stoppedPipeline = currentAssignedPipeline; // Capture before clearing (for 06-03 auto-execute)

    if (detailTitleInput && selectedRecordingId) {
      try {
        await invoke('update_title', { recordingId: selectedRecordingId, title: detailTitleInput.value });
      } catch (e) {
        console.error('Title sync failed (non-fatal):', e);
      }
    }

    await invoke("stop_recording");
    isRecording = false;
    currentAssignedPipeline = null; // Clear global after capturing to local

    stopTimer();
    stopWaveformAnimation();
    setRecordingUI(false);
    renderPipelineChips(); // Reset chip visual state after recording stops

    await loadRecordings();

    if (selectedRecordingId === currentId) {
      showDetailView(currentId);
      renderPipelineChips(); // Ensure chips are updated after detail view re-renders
    }

    // Auto-transcribe + auto-execute (EXEC-01) — fire and forget
    if (stoppedPipeline && appSettings?.transcription?.enabled) {
      autoTranscribeAndExecute(currentId, stoppedPipeline);
    }

  } catch (error) {
    // Always reset UI state, even on error
    isRecording = false;
    currentAssignedPipeline = null;
    stopTimer();
    stopWaveformAnimation();
    setRecordingUI(false);
    renderPipelineChips();
    console.error("Failed to stop:", error);
    if (error && error.includes && error.includes("discarded")) {
      hideDetailView();
      await loadRecordings();
    }
  } finally {
    isRecordingBusy = false;
  }
}

async function autoTranscribeAndExecute(recordingId, pipelineName) {
  // Step 1: Transcribe (blocking — must complete before pipeline)
  try {
    if (window.__NBP_setTranscribingId) window.__NBP_setTranscribingId(recordingId);
    await invoke('transcribe_recording', { recordingId });
  } catch (err) {
    console.error('Auto-transcription failed:', err);
    // Refresh detail view to show current state
    await loadRecordings();
    if (selectedRecordingId === recordingId) showDetailView(recordingId);
    return; // Do not proceed to pipeline execution if transcription failed
  }

  // Step 2: Execute pipeline (non-blocking from user perspective — status shown in detail view)
  try {
    await invoke('execute_pipeline', { recordingId, pipelineName });
  } catch (err) {
    console.error('Auto-pipeline execution failed:', err);
  }

  // Refresh recordings list and detail view
  await loadRecordings();
  if (selectedRecordingId === recordingId) showDetailView(recordingId);
}

async function subscribeToProgress(recordingId) {
  // Clean up previous listener (Research Pitfall 3)
  if (pipelineProgressUnlisten) {
    pipelineProgressUnlisten();
    pipelineProgressUnlisten = null;
  }

  pipelineProgressUnlisten = await window.__TAURI__.event.listen('pipeline-progress', (event) => {
    const payload = event.payload;
    if (payload.recording_id !== recordingId) return;
    // Re-render pipeline status section on any progress event
    renderPipelineStatus(recordingId);
  });
}

const PIPELINE_STATUS_DISPLAY = {
  waiting: 'Waiting',
  running: 'Running',
  done: 'Done',
  partial: 'Failed'
};

async function renderPipelineStatus(recordingId) {
  const section = document.getElementById('pipeline-status-section');
  const content = document.getElementById('pipeline-status-content');
  if (!section || !content) return;

  try {
    const states = await invoke('get_all_pipeline_states', { recordingId });
    if (!states || states.length === 0) {
      section.style.display = 'none';
      return;
    }

    section.style.display = '';

    let html = '';
    for (const state of states) {
      const displayText = PIPELINE_STATUS_DISPLAY[state.status] || state.status;
      html += `<div class="pipeline-status-row">
  <span class="pipeline-status-name">${escapeHtml(state.name)}</span>
  <span class="pipeline-status-badge status-${escapeHtml(state.status)}">${escapeHtml(displayText)}</span>
</div>`;

      // Show per-step status detail for partial and done pipelines
      if (state.status === 'partial' || state.status === 'done') {
        try {
          const steps = await invoke('get_step_outputs', { recordingId, pipelineName: state.name });
          if (steps && steps.length > 0) {
            html += '<div class="pipeline-steps-detail">';
            for (const step of steps) {
              let rowClass = 'pipeline-step-row';
              let iconHtml = '';
              let extraHtml = '';

              if (step.status === 'done') {
                rowClass += ' step-done';
                iconHtml = '<span class="step-status-icon">&#10003;</span>';
              } else if (step.status === 'failed') {
                rowClass += ' step-failed';
                iconHtml = '<span class="step-status-icon">&#10007;</span>';
                if (step.error) {
                  const shortError = step.error.length > 80 ? step.error.substring(0, 80) + '...' : step.error;
                  extraHtml = `<span class="step-error" title="${escapeHtml(step.error)}">${escapeHtml(shortError)}</span>`;
                }
              } else if (step.status === 'skipped') {
                rowClass += ' step-skipped';
                iconHtml = '<span class="step-status-icon">&#9675;</span>';
                extraHtml = '<span class="step-skipped-label">(skipped)</span>';
              } else {
                // pending or running — should not appear in final state but handle gracefully
                rowClass += ' step-pending';
                iconHtml = '<span class="step-status-icon">&#9675;</span>';
              }

              // Per-step wall-clock duration (only for completed steps)
              let durationHtml = '';
              if (step.duration_secs != null && step.duration_secs > 0) {
                const formatted = step.duration_secs >= 60
                  ? `${Math.floor(step.duration_secs / 60)}m ${Math.round(step.duration_secs % 60)}s`
                  : `${step.duration_secs.toFixed(1)}s`;
                durationHtml = `<span class="step-duration">${formatted}</span>`;
              }

              html += `<div class="${rowClass}">
  ${iconHtml}
  <span class="step-name">${escapeHtml(step.name)}</span>
  ${durationHtml}
  ${extraHtml}
</div>`;

              // Expandable augmented prompt section (LLM steps with prompt augmentation)
              if (step.augmented_prompt) {
                html += `<div class="augmented-prompt-section">
  <button class="augmented-prompt-toggle" onclick="this.parentElement.classList.toggle('expanded')">
    &#9654; Augmented prompt
  </button>
  <div class="augmented-prompt-content"><pre>${escapeHtml(step.augmented_prompt)}</pre></div>
</div>`;
              }
            }
            html += '</div>';
          }
        } catch (e) {
          console.error('Failed to load step outputs:', e);
        }
      }
    }

    content.innerHTML = html;
  } catch (e) {
    console.error('Failed to load pipeline states:', e);
    section.style.display = 'none';
  }
}

// ===== PERMISSIONS =====
async function updatePermissionStatus() {
  try {
    const onboardingCompleted = appSettings?.onboarding_completed || false;
    permissions = await invoke("check_permissions", { onboardingCompleted });

    // Update Onboarding UI
    const micItem = document.getElementById("perm-mic-item");
    const sysItem = document.getElementById("perm-sys-item");

    if (micItem) {
      const btn = micItem.querySelector(".modal-btn");
      btn.style.display = permissions.mic ? 'none' : 'block';
      micItem.querySelector(".perm-status-ok").style.display = permissions.mic ? 'block' : 'none';

      if (!permissions.mic && btn.dataset.requested === "true") {
        btn.textContent = "Open Settings";
      }
    }
    if (sysItem) {
      const btn = sysItem.querySelector(".modal-btn");
      btn.style.display = permissions.system_audio ? 'none' : 'block';
      sysItem.querySelector(".perm-status-ok").style.display = permissions.system_audio ? 'block' : 'none';

      if (!permissions.system_audio && btn.dataset.requested === "true") {
        btn.textContent = "Open Settings";
      }
    }

    if (onboardingContinueBtn) {
      onboardingContinueBtn.disabled = false; // Always allow continue
      const bothMissing = !permissions.mic && !permissions.system_audio;
      onboardingContinueBtn.textContent = bothMissing ? "I'll do that later" : "Continue";
    }

    // Update Warning Banner based on actual permissions
    // Now using real recording test instead of unreliable CGPreflightScreenCaptureAccess
    if (permissionWarning) {
      permissionWarning.style.display = (permissions.mic && permissions.system_audio) ? 'none' : 'flex';
    }

  } catch (err) {
    console.error("Failed to check permissions:", err);
  }
}

async function requestMic() {
  const btn = document.getElementById("request-mic-btn");
  if (btn.textContent === "Open Settings") {
    await invoke("open_privacy_settings", { pane: "mic" });
    return;
  }

  const success = await invoke("request_mic_permission");
  btn.dataset.requested = "true";

  if (!success) {
    btn.textContent = "Open Settings";
  }

  // Poll for status updates every 100ms for fast UI response
  for (let i = 0; i < 100; i++) {
    await new Promise(r => setTimeout(r, 100));
    await updatePermissionStatus();
    if (permissions.mic) break;
  }
}

async function requestSys() {
  const btn = document.getElementById("request-sys-btn");
  if (btn.textContent === "Open Settings") {
    await invoke("open_privacy_settings", { pane: "system_audio" });
    return;
  }

  const success = await invoke("request_system_audio_permission");
  btn.dataset.requested = "true";

  if (!success) {
    btn.textContent = "Open Settings";
  }

  // Poll for status updates every 100ms for fast UI response
  for (let i = 0; i < 100; i++) {
    await new Promise(r => setTimeout(r, 100));
    await updatePermissionStatus();
    if (permissions.system_audio) break;
  }
}

if (requestMicBtn) requestMicBtn.addEventListener("click", requestMic);
if (requestSysBtn) requestSysBtn.addEventListener("click", requestSys);
if (onboardingContinueBtn) {
  onboardingContinueBtn.addEventListener("click", async () => {
    // Mark onboarding as completed
    appSettings.onboarding_completed = true;
    await invoke("save_settings", { settings: appSettings });
    onboardingOverlay.style.display = 'none';
  });
}
if (fixPermissionsBtn) {
  fixPermissionsBtn.addEventListener("click", () => {
    onboardingOverlay.style.display = 'flex';
    updatePermissionStatus();
  });
}

// ===== RECORDINGS LIST =====
async function loadRecordings() {
  try {
    const recordings = await invoke("list_recordings");
    allRecordings = recordings || [];
    renderRecordingsList();
  } catch (error) {
    console.error("Failed to load recordings:", error);
  }
}

const dateOptions = {
  month: 'short',
  day: 'numeric',
  year: 'numeric',
  hour: 'numeric',
  minute: '2-digit'
};

function renderRecordingsList() {
  if (!recordingsListEl) return;

  if (allRecordings.length === 0) {
    recordingsListEl.innerHTML = "";
    if (emptyStateEl) emptyStateEl.style.display = "block";
    return;
  }
  if (emptyStateEl) emptyStateEl.style.display = "none";
  recordingsListEl.innerHTML = allRecordings.map(rec => {
    const isProcessing = rec.status === 'processing';
    const isCurrentlyRecording = isRecording && selectedRecordingId === rec.id;
    const metaText = isProcessing ? '<span style="color:var(--accent)">Processing...</span>' : formatDuration(getDuration(rec));

    // Health indicator
    const hasIssues = rec.health && rec.health.status !== 'ok';
    const healthIcon = hasIssues ? '<span class="health-warning" title="Issues occurred during recording">⚠️</span>' : '';

    const safeTitle = escapeHtml(rec.title || "Untitled");
    const safeId = escapeHtml(rec.id);

    return `
    <div class="recording-item ${isCurrentlyRecording ? 'recording-active' : ''}" data-id="${safeId}" onclick="showDetailView(this.dataset.id)">
        <div class="recording-item-header">
          <div class="recording-title">${healthIcon}${safeTitle}${isCurrentlyRecording ? ' <span style="color:var(--accent)">●</span>' : ''}</div>
          <div class="recording-meta">
            <span>${new Date(rec.created_at).toLocaleString(undefined, dateOptions)}</span>
            <span>·</span>
            <span>${metaText}</span>
          </div>
        </div>
      </div>
    `;
  }).join("");
}

function getDuration(rec) {
  if (!rec.audio) return 0;
  return rec.audio.mix?.duration_sec || rec.audio.mic?.duration_sec || rec.audio.system?.duration_sec || 0;
}

function formatDuration(seconds) {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")} `;
}



// ===== DETAIL VIEW =====
window.showDetailView = async (id) => {
  const rec = allRecordings.find(r => r.id === id);
  if (!rec) return;

  selectedRecordingId = id;
  updateMainButton(); // Switch to play mode

  if (detailTitleInput) detailTitleInput.value = rec.title || "";

  // Check Status
  const isProcessing = rec.status === 'processing';

  // Update Metadata in the Header
  if (detailMetaHeaderEl) {
    if (isRecording && id === selectedRecordingId && !isProcessing) {
      // While recording, handled by timer
    } else {
      const statusText = isProcessing ? '<span style="color:var(--accent)">Processing...</span>' : formatDuration(getDuration(rec));
      detailMetaHeaderEl.innerHTML = `${new Date(rec.created_at).toLocaleString(undefined, dateOptions)} · ${statusText} `;
    }
  }

  // Handle visibility
  ViewManager.showDetail();
  subscribeToProgress(id);

  if (detailControlsEl) detailControlsEl.style.display = 'flex';

  // LOCK BUTTONS if Processing
  if (deleteBtnHeader) {
    deleteBtnHeader.style.opacity = isProcessing ? '0.3' : '1';
    deleteBtnHeader.style.pointerEvents = isProcessing ? 'none' : 'auto';
    deleteBtnHeader.title = isProcessing ? "Processing audio..." : "Delete";
  }
  if (openFolderBtnHeader) {
    openFolderBtnHeader.style.opacity = isProcessing ? '0.3' : '1';
    openFolderBtnHeader.style.pointerEvents = isProcessing ? 'none' : 'auto';
    openFolderBtnHeader.title = isProcessing ? "Processing audio..." : "Open Folder";
  }
  if (prBtn) {
    prBtn.disabled = isProcessing;
    if (isProcessing) {
      prBtn.innerHTML = '<span style="font-weight: 600; font-size: 12px;">Mixing Audio...</span>';
    } else {
      prBtn.innerHTML = '<span style="font-weight: 600; font-size: 12px;">Transcribe</span>';
    }
  }

  // POLLING if processing — non-recursive, self-contained polling loop
  if (isProcessing) {
    const pollId = id;
    const pollInterval = setInterval(async () => {
      if (selectedRecordingId !== pollId) {
        clearInterval(pollInterval); // User navigated away — stop polling
        return;
      }
      try {
        await loadRecordings();
        const updated = allRecordings.find(r => r.id === pollId);
        if (updated && updated.status !== 'processing') {
          clearInterval(pollInterval);
          showDetailView(pollId);
        }
      } catch (e) {
        console.error('Processing poll error:', e);
        clearInterval(pollInterval);
      }
    }, 1000);
  }

  // Detail view pipeline assignment (ASGN-04)
  const detailPipelineAssignment = document.getElementById('detail-pipeline-assignment');
  const detailPipelineSelect = document.getElementById('detail-pipeline-select');
  if (detailPipelineAssignment && detailPipelineSelect) {
    if (!isRecording && !isProcessing) {
      // Populate pipeline options
      detailPipelineSelect.innerHTML = '<option value="">None</option>';
      if (typeof allPipelineDefs !== 'undefined') {
        for (const p of allPipelineDefs) {
          const opt = document.createElement('option');
          opt.value = p.name;
          opt.textContent = p.name;
          detailPipelineSelect.appendChild(opt);
        }
      }

      // Load current pipeline assignment
      try {
        const states = await invoke('get_all_pipeline_states', { recordingId: id });
        if (states && states.length > 0) {
          detailPipelineSelect.value = states[0].name || '';
        } else {
          detailPipelineSelect.value = '';
        }
      } catch (e) {
        detailPipelineSelect.value = '';
      }

      detailPipelineAssignment.style.display = 'flex';

      // Remove previous change listener before attaching a new one
      if (detailPipelineHandler) {
        detailPipelineSelect.removeEventListener('change', detailPipelineHandler);
      }
      detailPipelineHandler = async () => {
        const selectedPipeline = detailPipelineSelect.value;
        if (selectedPipeline) {
          try {
            await invoke('assign_pipeline', { recordingId: id, pipelineName: selectedPipeline });
          } catch (err) {
            console.error('Failed to assign pipeline from detail view:', err);
          }
        }
      };
      detailPipelineSelect.addEventListener('change', detailPipelineHandler);
    } else {
      detailPipelineAssignment.style.display = 'none';
    }
  }

  // Hide transcript/structured sections if currently recording or processing
  const hideContent = isRecording || isProcessing;
  const contentGrid = document.getElementById('detail-content-grid');
  if (contentGrid) contentGrid.style.display = hideContent ? 'none' : 'flex';

  // Render pipeline status (runs in parallel with transcript loading — no await needed)
  if (!hideContent) {
    renderPipelineStatus(id);
  }

  // Load Transcript only if not recording/processing
  if (!hideContent && detailTranscriptEl) {
    detailTranscriptEl.textContent = "Loading...";
    detailTranscriptEl.classList.remove('empty');

    try {
      const transcript = await invoke("get_transcript", { recordingId: id });
      if (transcript) {
        detailTranscriptEl.textContent = transcript;
        detailTranscriptEl.classList.remove('empty');
        if (saveTranscriptBtn) saveTranscriptBtn.style.display = '';
      } else {
        detailTranscriptEl.textContent = "Not processed yet.";
        detailTranscriptEl.classList.add('empty');
        if (saveTranscriptBtn) saveTranscriptBtn.style.display = 'none';
      }
    } catch (err) {
      console.error("Failed to load transcript:", err);
      detailTranscriptEl.textContent = "Not processed yet.";
      detailTranscriptEl.classList.add('empty');
      if (saveTranscriptBtn) saveTranscriptBtn.style.display = 'none';
    }
  }

  if (!hideContent && detailStructuredEl) {
    detailStructuredEl.textContent = "Not processed yet.";
    detailStructuredEl.classList.add('empty');

    // Try to load summary if exists
    try {
      const summaryPath = `${appSettings.storage_path}/${id}/summary.md`;
      // We'd need a backend call to read this - for now just show placeholder
    } catch (err) {
      // Ignore
    }
  }

  // Reset audio player
  if (!hideContent) {
    loadAudioDuration(id);
    isPlaying = false;
    if (playPauseBtn) {
      playPauseBtn.innerHTML = '<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>';
    }
    if (currentTimeEl) currentTimeEl.textContent = '0:00';
  }
};

function hideDetailView() {
  selectedRecordingId = null;
  updateMainButton(); // Switch back to record mode
  ViewManager.showRecordings();
  if (detailControlsEl) detailControlsEl.style.display = 'none';
  if (pipelineProgressUnlisten) {
    pipelineProgressUnlisten();
    pipelineProgressUnlisten = null;
  }
}

// ===== EVENT LISTENERS =====
if (recordToggleBtn) recordToggleBtn.addEventListener("click", toggleRecording);
if (backBtn) backBtn.addEventListener("click", hideDetailView);

if (detailTitleInput) {
  // Save title immediately on blur (even during recording)
  detailTitleInput.addEventListener('blur', async (e) => {
    if (!selectedRecordingId) return;
    try {
      await invoke('update_title', { recordingId: selectedRecordingId, title: e.target.value });
      await loadRecordings();
    } catch (err) {
      console.error('Failed to update title:', err);
    }
  });

  // Also save on Enter key
  detailTitleInput.addEventListener('keypress', async (e) => {
    if (e.key === 'Enter') {
      e.target.blur(); // Trigger blur event which saves
    }
  });
}

const deleteModal = document.getElementById('delete-modal');
const confirmDeleteBtn = document.getElementById('confirm-delete-btn');
const cancelDeleteBtn = document.getElementById('cancel-delete-btn');
let deleteTargetId = null; // Capture ID at modal open time to avoid race

const deleteBtnHeader = document.getElementById('delete-btn-header');
if (deleteBtnHeader) {
  deleteBtnHeader.addEventListener('click', () => {
    if (!selectedRecordingId) return;
    deleteTargetId = selectedRecordingId;
    deleteModal.style.display = 'flex';
  });
}

if (cancelDeleteBtn) {
  cancelDeleteBtn.addEventListener('click', () => {
    deleteModal.style.display = 'none';
    deleteTargetId = null;
  });
}

if (confirmDeleteBtn) {
  confirmDeleteBtn.addEventListener('click', async () => {
    if (!deleteTargetId) return;
    try {
      await invoke('delete_recording', { recordingId: deleteTargetId });
      deleteModal.style.display = 'none';
      deleteTargetId = null;
      hideDetailView();
      await loadRecordings();
    } catch (e) {
      console.error('Delete failed:', e);
      deleteModal.style.display = 'none';
      deleteTargetId = null;
      if (e && typeof e === 'string' && e.includes('finalized')) {
        alert('Recording is still being finalized. Please wait a moment and try again.');
      } else {
        alert('Delete failed: ' + e);
      }
    }
  });
}

// Close modal when clicking outside the card
if (deleteModal) {
  deleteModal.addEventListener('click', (e) => {
    if (e.target === deleteModal) deleteModal.style.display = 'none';
  });
}

const openFolderBtnHeader = document.getElementById('open-folder-btn-header');
if (openFolderBtnHeader) {
  openFolderBtnHeader.addEventListener('click', async () => {
    if (!selectedRecordingId || !appSettings?.storage_path) return;
    const folderPath = `${appSettings.storage_path}/${selectedRecordingId}`;
    try {
      await window.__TAURI_PLUGIN_OPENER__.openPath(folderPath);
    } catch (e) {
      console.error('Failed to open folder:', e);
    }
  });
}

// Listen for live transcription segments
if (window.__TAURI__) {
  const { listen } = window.__TAURI_PLATFORM_EVENT || {
    listen: (name, cb) => {
      // Fallback if platform event not accessible directly
      (async () => {
        const { listen: tauriListen } = await import('@tauri-apps/api/event');
        tauriListen(name, cb);
      })();
    }
  };

  // Transcription segment listener — scoped to the recording being transcribed
  try {
    let transcribingRecordingId = null;

    // Track which recording is being transcribed
    window.__NBP_setTranscribingId = (id) => { transcribingRecordingId = id; };

    window.__TAURI__.event.listen('recording_warning', (event) => {
      console.warn('Recording warning:', event.payload);
      // Show non-blocking notification — system audio may have failed
      const warn = document.createElement('div');
      warn.className = 'recording-warning-toast';
      warn.textContent = event.payload;
      document.body.appendChild(warn);
      setTimeout(() => warn.remove(), 5000);
    });

    window.__TAURI__.event.listen('transcription_segment', (event) => {
      const segmentText = event.payload;
      // Only append if we're still viewing the recording that's being transcribed
      if (detailTranscriptEl && selectedRecordingId && selectedRecordingId === transcribingRecordingId) {
        if (detailTranscriptEl.classList.contains('empty')) {
          detailTranscriptEl.textContent = '';
          detailTranscriptEl.classList.remove('empty');
        }
        detailTranscriptEl.textContent += segmentText + ' ';

        const scroller = detailTranscriptEl.closest('.detail-scroller');
        if (scroller) scroller.scrollTop = scroller.scrollHeight;
      }
    });
  } catch (e) {
    console.error("Failed to setup transcription listener:", e);
  }
}

const saveTranscriptBtn = document.getElementById('save-transcript-btn');
const prBtn = document.getElementById('process-btn');
if (prBtn) {
  prBtn.addEventListener('click', async () => {
    if (!selectedRecordingId || prBtn.disabled) return;

    try {
      prBtn.disabled = true;
      prBtn.innerHTML = '<span style="font-weight: 600; font-size: 12px;">Processing...</span>';

      if (detailTranscriptEl) {
        detailTranscriptEl.textContent = '';
        detailTranscriptEl.classList.remove('empty');
      }

      // Set which recording is being transcribed so listener is scoped
      if (window.__NBP_setTranscribingId) window.__NBP_setTranscribingId(selectedRecordingId);

      const transcript = await invoke('transcribe_recording', { recordingId: selectedRecordingId });

      // Final update to ensure everything is matched correctly
      if (detailTranscriptEl) {
        detailTranscriptEl.textContent = transcript;
        detailTranscriptEl.classList.remove('empty');
      }
      if (saveTranscriptBtn) saveTranscriptBtn.style.display = '';

      prBtn.innerHTML = '<span style="font-weight: 600; font-size: 12px;">Transcribe</span>';
      prBtn.disabled = false;

    } catch (error) {
      console.error('Transcription failed:', error);
      alert(`Transcription failed: ${error}`);

      if (detailTranscriptEl) {
        detailTranscriptEl.textContent = 'Transcription failed.';
        detailTranscriptEl.classList.add('empty');
      }

      prBtn.innerHTML = '<span style="font-weight: 600; font-size: 12px;">Transcribe</span>';
      prBtn.disabled = false;
    }
  });
}

// ===== SAVE TRANSCRIPT BUTTON =====
if (saveTranscriptBtn) {
  saveTranscriptBtn.addEventListener('click', async () => {
    if (!selectedRecordingId) return;
    try {
      saveTranscriptBtn.disabled = true;
      await invoke('export_transcript_md', { recordingId: selectedRecordingId });
    } catch (error) {
      console.error('Save transcript failed:', error);
      alert(`Save failed: ${error}`);
    } finally {
      saveTranscriptBtn.disabled = false;
    }
  });
}

// ===== SUMMARIZE & TEMPLATE PROCESSING =====
const summarizeBtn = document.getElementById('summarize-btn');
const extractBtn = document.getElementById('extract-btn');
const templateSelect = document.getElementById('template-select');

if (templateSelect) {
  templateSelect.addEventListener('change', () => {
    if (extractBtn) {
      extractBtn.disabled = !templateSelect.value;
    }
  });
}

if (summarizeBtn) {
  summarizeBtn.addEventListener('click', async () => {
    if (!selectedRecordingId) return;

    try {
      summarizeBtn.disabled = true;
      summarizeBtn.innerHTML = '<span style="font-weight: 600; font-size: 12px;">Summarizing...</span>';

      const summary = await invoke('summarize_recording', {
        recordingId: selectedRecordingId,
        provider: null
      });

      if (detailStructuredEl) {
        detailStructuredEl.textContent = summary;
        detailStructuredEl.classList.remove('empty');
      }

      summarizeBtn.innerHTML = '<span style="font-weight: 600; font-size: 12px;">Summarize</span>';
      summarizeBtn.disabled = false;
    } catch (error) {
      console.error('Summarization failed:', error);
      alert(`Summarization failed: ${error}`);
      summarizeBtn.innerHTML = '<span style="font-weight: 600; font-size: 12px;">Summarize</span>';
      summarizeBtn.disabled = false;
    }
  });
}

if (extractBtn) {
  extractBtn.addEventListener('click', async () => {
    if (!selectedRecordingId || !templateSelect.value) return;

    try {
      extractBtn.disabled = true;
      extractBtn.innerHTML = '<span style="font-weight: 600; font-size: 12px;">Extracting...</span>';

      const result = await invoke('process_with_template', {
        recordingId: selectedRecordingId,
        templateName: templateSelect.value,
        provider: null
      });

      if (detailStructuredEl) {
        detailStructuredEl.textContent = result;
        detailStructuredEl.classList.remove('empty');
      }

      extractBtn.innerHTML = '<span style="font-weight: 600; font-size: 12px;">Extract</span>';
      extractBtn.disabled = !templateSelect.value;
    } catch (error) {
      console.error('Extraction failed:', error);
      alert(`Extraction failed: ${error}`);
      extractBtn.innerHTML = '<span style="font-weight: 600; font-size: 12px;">Extract</span>';
      extractBtn.disabled = !templateSelect.value;
    }
  });
}


// ===== DEFAULT PIPELINE SELECT =====
function populateDefaultPipelineSelect() {
  const select = document.getElementById('settings-default-pipeline');
  if (!select) return;

  const currentValue = select.value; // preserve current selection if any
  select.innerHTML = '<option value="">None</option>';
  if (typeof allPipelineDefs !== 'undefined') {
    for (const p of allPipelineDefs) {
      const opt = document.createElement('option');
      opt.value = p.name;
      opt.textContent = p.name;
      select.appendChild(opt);
    }
  }

  // Restore saved default_pipeline from appSettings
  if (appSettings && appSettings.default_pipeline) {
    select.value = appSettings.default_pipeline;
  } else {
    select.value = currentValue || '';
  }
}

// ===== SETTINGS ELEMENTS =====
const transcriptionEnabledCheckbox = document.getElementById("settings-transcription-enabled");
const transcriptionDetailsEl = document.getElementById("transcription-details");
const transcriptionProviderSelect = document.getElementById("settings-transcription-provider");
const providerLocalSection = document.getElementById("provider-local-section");
const providerApiSection = document.getElementById("provider-api-section");
const whisperModelSelect = document.getElementById("settings-whisper-model");
const apiKeyInputOpenAI = document.getElementById("settings-api-key-openai");
const apiKeyInputGoogle = document.getElementById("settings-api-key-google");
const apiKeyInputAnthropic = document.getElementById("settings-api-key-anthropic");
const downloadModelBtn = document.getElementById("download-model-btn");
const recordingNotificationCheckbox = document.getElementById("settings-recording-notification");
const saveMixOnlyCheckbox = document.getElementById("settings-save-mix-only");

// Helper to mask API keys (show last 4 chars)
function maskApiKey(key) {
  if (!key || key.length < 8) return key || "";
  return "•".repeat(key.length - 4) + key.slice(-4);
}

// Helper to unmask API key if it was already masked
function isKeyMasked(value) {
  return value && value.includes("•");
}

// ===== SETTINGS =====
async function loadSettings() {
  try {
    appSettings = await invoke("load_settings");

    if (storagePathInput) storagePathInput.value = appSettings.storage_path;
    if (cleanupThresholdInput) cleanupThresholdInput.value = appSettings.auto_discard_seconds;

    // Transcription Settings
    if (appSettings.transcription) {
      if (transcriptionEnabledCheckbox) {
        transcriptionEnabledCheckbox.checked = appSettings.transcription.enabled;
        updateTranscriptionVisibility();
      }
      if (transcriptionProviderSelect) transcriptionProviderSelect.value = appSettings.transcription.provider;
      if (whisperModelSelect) {
        whisperModelSelect.value = appSettings.transcription.whisper_model || "Base";
      }

      // Load API keys (masked for display)
      const apiKeys = appSettings.transcription.api_keys || {};
      if (apiKeyInputOpenAI) {
        apiKeyInputOpenAI.value = maskApiKey(apiKeys.openai);
        apiKeyInputOpenAI.dataset.originalKey = apiKeys.openai || "";
      }
      if (apiKeyInputGoogle) {
        apiKeyInputGoogle.value = maskApiKey(apiKeys.google);
        apiKeyInputGoogle.dataset.originalKey = apiKeys.google || "";
      }
      if (apiKeyInputAnthropic) {
        apiKeyInputAnthropic.value = maskApiKey(apiKeys.anthropic);
        apiKeyInputAnthropic.dataset.originalKey = apiKeys.anthropic || "";
      }

      updateProviderVisibility();
    }

    // Recording notification setting
    if (recordingNotificationCheckbox) {
      recordingNotificationCheckbox.checked = appSettings.show_recording_notification !== false;
    }

    // Save mix only setting
    if (saveMixOnlyCheckbox) {
      saveMixOnlyCheckbox.checked = appSettings.save_mix_only !== false; // default true
      console.log('DEBUG: Loaded save_mix_only =', appSettings.save_mix_only, '→ checkbox =', saveMixOnlyCheckbox.checked);
    }

    applyTheme(appSettings.theme);
  } catch (err) {
    console.error("Failed to load settings:", err);
  }
}

async function saveSettings() {
  try {
    appSettings.auto_discard_seconds = parseInt(cleanupThresholdInput.value) || 0;

    if (!appSettings.transcription) appSettings.transcription = {};

    appSettings.transcription.enabled = transcriptionEnabledCheckbox.checked;
    appSettings.transcription.provider = transcriptionProviderSelect.value;
    appSettings.transcription.whisper_model = whisperModelSelect.value;

    // Handle API keys - only update if user changed them (not masked)
    if (!appSettings.transcription.api_keys) appSettings.transcription.api_keys = {};

    // OpenAI key
    if (apiKeyInputOpenAI && !isKeyMasked(apiKeyInputOpenAI.value)) {
      appSettings.transcription.api_keys.openai = apiKeyInputOpenAI.value || null;
    }
    // Google key
    if (apiKeyInputGoogle && !isKeyMasked(apiKeyInputGoogle.value)) {
      appSettings.transcription.api_keys.google = apiKeyInputGoogle.value || null;
    }
    // Anthropic key
    if (apiKeyInputAnthropic && !isKeyMasked(apiKeyInputAnthropic.value)) {
      appSettings.transcription.api_keys.anthropic = apiKeyInputAnthropic.value || null;
    }

    // Recording notification
    if (recordingNotificationCheckbox) {
      appSettings.show_recording_notification = recordingNotificationCheckbox.checked;
    }

    // Save mix only setting
    if (saveMixOnlyCheckbox) {
      appSettings.save_mix_only = saveMixOnlyCheckbox.checked;
      console.log('DEBUG: Saving save_mix_only =', saveMixOnlyCheckbox.checked);
    }

    // Default pipeline setting
    const defaultPipelineSelect = document.getElementById('settings-default-pipeline');
    if (defaultPipelineSelect) {
      appSettings.default_pipeline = defaultPipelineSelect.value || null;
    }

    await invoke("save_settings", { settings: appSettings });
    ViewManager.showRecordings();
    await loadRecordings();
  } catch (err) {
    console.error("Failed to save settings:", err);
  }
}

function updateTranscriptionVisibility() {
  if (!transcriptionDetailsEl) return;
  transcriptionDetailsEl.style.display = transcriptionEnabledCheckbox.checked ? 'flex' : 'none';
}


let availableModels = [];

async function updateProviderVisibility() {
  if (!providerLocalSection || !providerApiSection) return;
  const provider = transcriptionProviderSelect.value;

  if (provider === "LocalWhisper") {
    providerLocalSection.style.display = 'flex';
    providerApiSection.style.display = 'none';

    // Fetch model info when showing this section
    await loadWhisperModelsAndState();
  } else {
    providerLocalSection.style.display = 'none';
    providerApiSection.style.display = 'flex';
  }
}

async function loadWhisperModelsAndState() {
  if (!whisperModelSelect) return;
  whisperModelSelect.disabled = true;
  try {
    availableModels = await invoke("get_whisper_models_info");
    const currentVal = appSettings?.transcription?.whisper_model || whisperModelSelect.value || "Base";

    whisperModelSelect.innerHTML = availableModels.map(m => {
      const sizeStr = m.size_mb ? `(~${m.size_mb} MB)` : '';
      const statusIcon = m.downloaded ? '✅' : '⬇️';

      let label = `${statusIcon} ${m.size} ${sizeStr}`;
      if (m.size === 'Base') label += ' (Recommended)';
      if (m.size === 'Large') label += ' (Best Quality)';

      return `<option value="${m.size}">${label}</option>`;
    }).join('');

    whisperModelSelect.value = currentVal;
    updateDownloadButton();
  } catch (e) {
    console.error(e);
  } finally {
    whisperModelSelect.disabled = false;
  }
}



function updateDownloadButton() {
  if (!downloadModelBtn || !whisperModelSelect) return;

  // If we are currently downloading, don't reset unless finished
  if (downloadModelBtn.dataset.downloading === "true") return;

  const selectedSize = whisperModelSelect.value;
  const model = availableModels.find(m => m.size === selectedSize);

  if (model) {
    if (model.downloaded) {
      // Show TRASH / DELETE
      downloadModelBtn.dataset.action = "delete";
      downloadModelBtn.title = "Delete Model";
      downloadModelBtn.classList.remove("mini-action-btn-primary");
      downloadModelBtn.style.color = "var(--text-danger, #ff4d4d)"; // Red for danger
      downloadModelBtn.innerHTML = `
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
           <polyline points="3 6 5 6 21 6"></polyline>
           <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
        </svg>`;
    } else {
      // Show DOWNLOAD
      downloadModelBtn.dataset.action = "download";
      downloadModelBtn.title = "Download Model";
      downloadModelBtn.classList.add("mini-action-btn-primary");
      downloadModelBtn.style.color = "var(--accent)";
      downloadModelBtn.innerHTML = `
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="7 10 12 15 17 10"></polyline>
            <line x1="12" y1="15" x2="12" y2="3"></line>
        </svg>`;
    }
  }
}

// Download/Delete Logic
if (downloadModelBtn) {
  downloadModelBtn.addEventListener("click", async () => {
    if (!whisperModelSelect) return;
    const size = whisperModelSelect.value;
    const action = downloadModelBtn.dataset.action;

    // Prevent multiple clicks
    if (downloadModelBtn.dataset.downloading === "true") return;

    if (action === "delete") {
      if (confirm(`Are you sure you want to delete the ${size} model?`)) {
        try {
          await invoke("delete_whisper_model", { size });
          await loadWhisperModelsAndState(); // Refresh UI
        } catch (err) {
          console.error("Delete failed:", err);
        }
      }
      return;
    }

    // --- DOWNLOAD LOGIC ---
    downloadModelBtn.dataset.downloading = "true";
    downloadModelBtn.title = "Downloading...";

    // Replace icon with PIE progress
    downloadModelBtn.innerHTML = `
          <svg class="progress-pie" width="24" height="24" viewBox="0 0 24 24">
             <circle cx="12" cy="12" r="10" stroke="var(--border)" stroke-width="1" fill="none" opacity="0.5"/>
             <path class="progress-pie__slice" fill="var(--accent)" d="" />
          </svg>
        `;

    try {
      // Listen for progress
      const unlisten = await window.__TAURI__.event.listen('download_progress', (event) => {
        const { percent } = event.payload;
        const slice = downloadModelBtn.querySelector('.progress-pie__slice');
        if (slice) {
          const d = getPiePath(12, 12, 10, percent);
          slice.setAttribute('d', d);
        } else {
          console.error("Progress pie slice element missing!");
        }
      });

      await invoke("download_whisper_model", { size });

      // Success
      unlisten();
      downloadModelBtn.dataset.downloading = "false";
      // Refresh state
      await loadWhisperModelsAndState();


    } catch (err) {
      console.error("Download failed:", err);
      // alert("Download failed: " + err); // Removed per user request
      downloadModelBtn.dataset.downloading = "false";
      updateDownloadButton();
    }
  });
}


function getPiePath(cx, cy, r, percentage) {
  if (percentage >= 100) {
    return `M ${cx}, ${cy} m -${r}, 0 a ${r},${r} 0 1,0 ${r * 2},0 a ${r},${r} 0 1,0 -${r * 2},0`;
  }

  // Start at top ( -90 deg)
  const startAngle = -Math.PI / 2;
  const angle = (percentage / 100) * 2 * Math.PI;
  const endAngle = startAngle + angle;

  const x1 = cx + r * Math.cos(startAngle);
  const y1 = cy + r * Math.sin(startAngle);

  const x2 = cx + r * Math.cos(endAngle);
  const y2 = cy + r * Math.sin(endAngle);

  const largeArc = percentage > 50 ? 1 : 0;

  // Move to center, Line to start, Arc to end, Close path
  return `M ${cx} ${cy} L ${x1} ${y1} A ${r} ${r} 0 ${largeArc} 1 ${x2} ${y2} Z`;
}

// Add change listener to update button state
if (whisperModelSelect) {
  whisperModelSelect.addEventListener('change', () => {
    updateDownloadButton();
    // Update settings immediately? Or wait for save?
    // Wait for save, but update internal state if needed
  });
}

function applyTheme(themeName) {
  // Migrate old theme names
  if (themeName === "light-pastel") themeName = "light";
  if (themeName === "deep-obsidian") themeName = "neon-purple";

  document.body.classList.remove("neon-purple", "deep-blue", "light");
  if (themeName !== "neon-purple") {
    document.body.classList.add(themeName);
  }

  appSettings.theme = themeName;

  themeButtons.forEach(btn => {
    btn.classList.toggle("active", btn.dataset.theme === themeName);
  });
}

// ===== SETTINGS TABS =====
function switchSettingsTab(tabName) {
  document.querySelectorAll('.settings-tab').forEach(t => {
    t.classList.toggle('active', t.dataset.tab === tabName);
  });
  document.querySelectorAll('.settings-tab-content').forEach(c => {
    c.classList.toggle('active', c.dataset.tab === tabName);
  });
}

const settingsTabs = document.getElementById('settings-tabs');
if (settingsTabs) {
  settingsTabs.addEventListener('click', (e) => {
    const tab = e.target.closest('.settings-tab');
    if (tab) switchSettingsTab(tab.dataset.tab);
  });
}

// ===== SIDEBAR NAV =====
const sidebarPipelinesBtn = document.getElementById('sidebar-pipelines-btn');
const sidebarTemplatesBtn = document.getElementById('sidebar-templates-btn');
const sidebarPipelineCount = document.getElementById('sidebar-pipeline-count');
const sidebarTemplateCount = document.getElementById('sidebar-template-count');

function updateSidebarCounts() {
  if (sidebarPipelineCount) sidebarPipelineCount.textContent = allPipelineDefs.length || '';
  if (sidebarTemplateCount) sidebarTemplateCount.textContent = allPromptTemplates.length || '';
}

function renderPipelineChips() {
  const chipBar = document.getElementById('pipeline-chip-bar');
  if (!chipBar) return;

  if (typeof allPipelineDefs === 'undefined' || allPipelineDefs.length === 0) {
    chipBar.innerHTML = '';
    chipBar.style.display = 'none';
    return;
  }

  chipBar.style.display = '';

  const MAX_CHIPS = 5;
  const visible = allPipelineDefs.slice(0, MAX_CHIPS);
  const overflow = allPipelineDefs.slice(MAX_CHIPS);

  let html = '';
  for (const p of visible) {
    let cls = 'pipeline-chip';
    if (isRecording && currentAssignedPipeline === p.name) {
      cls += ' is-assigned';
    } else if (!isRecording && appSettings?.last_used_pipeline === p.name) {
      cls += ' is-last-used';
    }
    html += `<button class="${cls}" data-pipeline-name="${escapeHtml(p.name)}">${escapeHtml(p.name)}</button>`;
  }

  if (overflow.length > 0) {
    html += `<button class="chip-overflow-btn" id="chip-overflow-btn" aria-label="Show more pipelines" aria-haspopup="true">+${overflow.length}</button>`;
  }

  chipBar.innerHTML = html;

  chipBar.querySelectorAll('.pipeline-chip').forEach(chip => {
    chip.addEventListener('click', () => handleChipClick(chip.dataset.pipelineName));
  });

  const overflowBtn = chipBar.querySelector('#chip-overflow-btn');
  if (overflowBtn) {
    overflowBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      showOverflowPopover(overflow);
    });
  }
}

function showOverflowPopover(pipelines) {
  const existing = document.querySelector('.chip-overflow-popover');
  if (existing) existing.remove();

  const chipBar = document.getElementById('pipeline-chip-bar');
  if (!chipBar) return;

  const popover = document.createElement('div');
  popover.className = 'chip-overflow-popover';
  popover.setAttribute('role', 'menu');
  popover.style.maxHeight = '240px';
  popover.style.overflowY = 'auto';

  for (const p of pipelines) {
    let cls = 'pipeline-chip';
    if (isRecording && currentAssignedPipeline === p.name) {
      cls += ' is-assigned';
    } else if (!isRecording && appSettings?.last_used_pipeline === p.name) {
      cls += ' is-last-used';
    }
    const btn = document.createElement('button');
    btn.className = cls;
    btn.dataset.pipelineName = p.name;
    btn.textContent = p.name;
    btn.setAttribute('role', 'menuitem');
    btn.addEventListener('click', () => {
      popover.remove();
      handleChipClick(p.name);
    });
    popover.appendChild(btn);
  }

  chipBar.appendChild(popover);

  function dismissOverflow(e) {
    if (!popover.contains(e.target)) {
      popover.remove();
    }
  }

  setTimeout(() => document.addEventListener('click', dismissOverflow, { once: true }), 0);
}

async function handleChipClick(pipelineName) {
  if (isRecordingBusy) return;
  if (isRecording) {
    // Mid-recording assignment (ASGN-03)
    try {
      await invoke('assign_pipeline', { recordingId: selectedRecordingId, pipelineName });
      currentAssignedPipeline = pipelineName;
      renderPipelineChips(); // Update visual state
    } catch (err) {
      console.error('Failed to assign pipeline mid-recording:', err);
    }
  } else {
    await startRecordingWithPipeline(pipelineName);
  }
}

async function startRecordingWithPipeline(pipelineName) {
  isRecordingBusy = true;
  ViewManager.showRecordings();

  const saveMixOnly = appSettings?.save_mix_only !== false;
  try {
    const metadata = await invoke('start_recording', { saveMixOnly });
    isRecording = true;
    currentAssignedPipeline = pipelineName;
    await invoke('assign_pipeline', { recordingId: metadata.id, pipelineName });
    // Save last-used pipeline so chip bar highlights it on next launch
    appSettings.last_used_pipeline = pipelineName;
    await invoke('save_settings', { settings: appSettings });
    setRecordingUI(true);
    await loadRecordings();
    startTimer();
    startWaveformAnimation();
    showDetailView(metadata.id);
    renderPipelineChips(); // Update chip visual state (show assigned chip)
  } catch (error) {
    // Revert all state on failure
    isRecording = false;
    currentAssignedPipeline = null;
    stopTimer();
    stopWaveformAnimation();
    setRecordingUI(false);
    console.error('Failed to start recording with pipeline:', error);
    alert('Failed to start: ' + error);
  } finally {
    isRecordingBusy = false;
  }
}

if (sidebarPipelinesBtn) {
  sidebarPipelinesBtn.addEventListener('click', () => {
    ViewManager.showSettings();
    switchSettingsTab('pipelines');
  });
}

if (sidebarTemplatesBtn) {
  sidebarTemplatesBtn.addEventListener('click', () => {
    ViewManager.showSettings();
    switchSettingsTab('templates');
  });
}

// ===== SETTINGS EVENT LISTENERS =====
if (settingsBtn) settingsBtn.addEventListener("click", () => {
  ViewManager.showSettings();
});
if (settingsBackBtn) settingsBackBtn.addEventListener("click", () => {
  ViewManager.showRecordings();
});
if (saveSettingsBtn) saveSettingsBtn.addEventListener("click", saveSettings);

if (transcriptionEnabledCheckbox) {
  transcriptionEnabledCheckbox.addEventListener("change", updateTranscriptionVisibility);
}

if (transcriptionProviderSelect) {
  transcriptionProviderSelect.addEventListener("change", updateProviderVisibility);
}



if (browseStorageBtn) {
  browseStorageBtn.addEventListener("click", async () => {
    try {
      const selected = await window.__TAURI__.dialog.open({
        directory: true,
        multiple: false,
        defaultPath: appSettings.storage_path
      });
      if (selected) {
        appSettings.storage_path = selected;
        storagePathInput.value = selected;
      }
    } catch (err) {
      console.error("Failed to browse:", err);
    }
  });
}

themeButtons.forEach(btn => {
  btn.addEventListener("click", () => applyTheme(btn.dataset.theme));
});

// ===== PROMPT TEMPLATE MANAGEMENT =====
let allPromptTemplates = [];
let editingPromptTemplate = null; // null = new, string = editing name

const promptTemplatesListEl = document.getElementById('prompt-templates-list');
const addPromptTemplateBtn = document.getElementById('add-prompt-template-btn');
const promptTemplateEditor = document.getElementById('prompt-template-editor');
const promptEditorTitle = document.getElementById('prompt-editor-title');
const promptEditorName = document.getElementById('prompt-editor-name');
const promptEditorDesc = document.getElementById('prompt-editor-desc');
const promptEditorText = document.getElementById('prompt-editor-text');
const savePromptTemplateBtn = document.getElementById('save-prompt-template-btn');
const deletePromptTemplateBtn = document.getElementById('delete-prompt-template-btn');
const closePromptEditorBtn = document.getElementById('close-prompt-editor');

async function loadPromptTemplates() {
  try {
    allPromptTemplates = await invoke('list_prompt_templates');
    renderPromptTemplatesList();
    updateSidebarCounts();
  } catch (err) {
    console.error('Failed to load prompt templates:', err);
  }
}

function renderPromptTemplatesList() {
  if (!promptTemplatesListEl) return;
  if (allPromptTemplates.length === 0) {
    promptTemplatesListEl.innerHTML = '<div style="color: var(--text-secondary); opacity: 0.6; font-size: 0.85rem;">No templates yet.</div>';
    return;
  }
  promptTemplatesListEl.innerHTML = allPromptTemplates.map(t => {
    const safeName = escapeHtml(t.name);
    const safeDesc = escapeHtml(t.description || '');
    const safePreview = escapeHtml((t.prompt || '').substring(0, 80)) + (t.prompt && t.prompt.length > 80 ? '...' : '');
    return `
    <div class="template-item" data-name="${safeName}">
      <div class="template-item-info">
        <div class="template-item-name">${safeName}</div>
        <div class="template-item-desc">${safeDesc}</div>
        <div class="template-item-preview">${safePreview}</div>
      </div>
    </div>
  `;
  }).join('');

  promptTemplatesListEl.querySelectorAll('.template-item').forEach(el => {
    el.addEventListener('click', () => openPromptEditor(el.dataset.name));
  });
}

function openPromptEditor(name) {
  if (!promptTemplateEditor) return;
  if (name) {
    const t = allPromptTemplates.find(t => t.name === name);
    if (!t) return;
    editingPromptTemplate = name;
    promptEditorTitle.textContent = 'Edit Prompt Template';
    promptEditorName.value = t.name;
    promptEditorDesc.value = t.description || '';
    promptEditorText.value = t.prompt || '';
    if (deletePromptTemplateBtn) deletePromptTemplateBtn.style.display = 'inline-block';
  } else {
    editingPromptTemplate = null;
    promptEditorTitle.textContent = 'New Prompt Template';
    promptEditorName.value = '';
    promptEditorDesc.value = '';
    promptEditorText.value = '';
    if (deletePromptTemplateBtn) deletePromptTemplateBtn.style.display = 'none';
  }
  promptTemplateEditor.style.display = 'block';
  promptEditorName.focus();
}

function closePromptEditor() {
  if (promptTemplateEditor) promptTemplateEditor.style.display = 'none';
  editingPromptTemplate = null;
}

if (addPromptTemplateBtn) addPromptTemplateBtn.addEventListener('click', () => openPromptEditor(null));
if (closePromptEditorBtn) closePromptEditorBtn.addEventListener('click', closePromptEditor);

if (savePromptTemplateBtn) {
  savePromptTemplateBtn.addEventListener('click', async () => {
    const name = promptEditorName.value.trim();
    const desc = promptEditorDesc.value.trim();
    const prompt = promptEditorText.value.trim();
    if (!name) { alert('Name is required'); return; }
    if (!prompt) { alert('Prompt text is required'); return; }

    try {
      const template = {
        name,
        description: desc,
        prompt,
        created_at: editingPromptTemplate ? (allPromptTemplates.find(t => t.name === editingPromptTemplate)?.created_at || new Date().toISOString()) : new Date().toISOString(),
        updated_at: new Date().toISOString()
      };

      // If renaming, delete old first
      if (editingPromptTemplate && editingPromptTemplate !== name) {
        await invoke('delete_prompt_template', { name: editingPromptTemplate, force: true });
      }
      await invoke('save_prompt_template', { template });
      closePromptEditor();
      await loadPromptTemplates();
    } catch (err) {
      console.error('Failed to save prompt template:', err);
      alert('Failed to save: ' + err);
    }
  });
}

if (deletePromptTemplateBtn) {
  deletePromptTemplateBtn.addEventListener('click', async () => {
    if (!editingPromptTemplate) return;
    if (!confirm(`Delete template "${editingPromptTemplate}"?`)) return;
    try {
      await invoke('delete_prompt_template', { name: editingPromptTemplate, force: true });
      closePromptEditor();
      await loadPromptTemplates();
    } catch (err) {
      console.error('Failed to delete prompt template:', err);
      alert('Failed to delete: ' + err);
    }
  });
}

// ===== SLACK INTEGRATIONS =====
let slackIntegrations = {};

const addSlackBtn = document.getElementById('add-slack-btn');
const addSlackModal = document.getElementById('add-slack-modal');
const slackNameInput = document.getElementById('slack-name-input');
const slackTokenInput = document.getElementById('slack-token-input');
const slackSaveBtn = document.getElementById('slack-save-btn');
const slackCancelBtn = document.getElementById('slack-cancel-btn');
async function loadSlackIntegrations() {
  try {
    slackIntegrations = await invoke('list_slack_integrations');
  } catch (err) {
    console.error('Failed to load Slack integrations:', err);
  }
}

// add-slack-btn handler removed — integrations-settings.js now handles opening the modal
// via the "Available" section Slack card click handler.

if (slackCancelBtn) {
  slackCancelBtn.addEventListener('click', () => {
    addSlackModal.style.display = 'none';
  });
}

if (slackSaveBtn) {
  slackSaveBtn.addEventListener('click', async () => {
    const name = slackNameInput.value.trim();
    const token = slackTokenInput.value.trim();
    
    if (!name) {
      alert('Please enter a name');
      return;
    }
    if (!token) {
      alert('Please enter a bot token');
      return;
    }
    if (!token.startsWith('xoxb-')) {
      alert('Invalid token format. Bot tokens start with xoxb-');
      return;
    }
    
    // Generate ID from name
    const id = name.toLowerCase().replace(/[^a-z0-9]/g, '-');
    
    slackSaveBtn.disabled = true;
    slackSaveBtn.textContent = 'Saving...';
    
    try {
      await invoke('add_slack_integration', { id, name, token });
      addSlackModal.style.display = 'none';
      await loadSlackIntegrations();
    } catch (err) {
      alert(`Failed to add Slack workspace: ${err}`);
    } finally {
      slackSaveBtn.disabled = false;
      slackSaveBtn.textContent = 'Save';
    }
  });
}

// ===== LOAD TEMPLATES =====
async function loadTemplates() {
  if (!templateSelect) return;

  try {
    const templates = await invoke('list_templates');
    templateSelect.innerHTML = '<option value="">Select template...</option>' +
      templates.map(t => `<option value="${escapeHtml(t.name)}">${escapeHtml(t.description)}</option>`).join('');
  } catch (err) {
    console.error('Failed to load templates:', err);
  }
}

// ===== INIT =====
async function init() {
  await loadSettings();
  await loadRecordings();
  await loadTemplates();
  await loadPromptTemplates();
  await loadPipelineDefs();
  await loadSlackIntegrations();
  try {
    const version = await invoke("get_app_version");
    const versionEl = document.getElementById("app-version");
    if (versionEl) versionEl.textContent = `v${version} `;
  } catch (err) {
    console.error("Failed to fetch version:", err);
  }

  await updatePermissionStatus();

  // Show onboarding only if never completed before
  if (!appSettings.onboarding_completed) {
    onboardingOverlay.style.display = 'flex';
  }
}

init().catch(e => console.error('Init failed:', e)).finally(() => {
  // Initialize health check controls (event listeners wired once)
  if (typeof initHealthCheck === 'function') initHealthCheck();

  // Schedule DOM audit after browser is idle
  const scheduleAudit = () => {
    if (typeof runHealthAudit === 'function') {
      runHealthAudit();
      // Trigger walkthrough on first launch (after audit so badge is visible)
      // Only if onboarding is already completed (permissions granted)
      if (typeof appSettings !== 'undefined' &&
          appSettings.onboarding_completed &&
          !appSettings.walkthrough_completed &&
          typeof startWalkthrough === 'function') {
        startWalkthrough();
      }
    }
  };
  if (typeof requestIdleCallback !== 'undefined') {
    requestIdleCallback(scheduleAudit, { timeout: 2000 });
  } else {
    setTimeout(scheduleAudit, 500);
  }
});
