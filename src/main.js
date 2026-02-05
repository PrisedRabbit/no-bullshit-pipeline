const { invoke } = window.__TAURI__.core;

// ===== SPEAKER TRANSCRIPT FORMATTING =====
// Matches compact format: "SP1: text" or "SP2: text"
const SPEAKER_LINE_PATTERN = /^(SP\d+):\s*(.+)$/m;
const SPEAKER_COLORS = ['var(--accent)', 'var(--success)', '#f59e0b', '#ec4899', '#06b6d4', '#8b5cf6'];

function hasSpeakerLabels(text) {
  return SPEAKER_LINE_PATTERN.test(text);
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function formatSpeakerTranscript(text) {
  const lines = text.split('\n');
  const speakerOrder = [];

  // First pass: discover unique speakers in order
  for (const line of lines) {
    const m = line.match(SPEAKER_LINE_PATTERN);
    if (m && !speakerOrder.includes(m[1])) {
      speakerOrder.push(m[1]);
    }
  }

  // Second pass: render with color by speaker identity
  return lines.map(line => {
    const match = line.match(SPEAKER_LINE_PATTERN);
    if (match) {
      const speakerLabel = escapeHtml(match[1]);
      const content = escapeHtml(match[2]);
      const colorIdx = speakerOrder.indexOf(match[1]) % SPEAKER_COLORS.length;
      const color = SPEAKER_COLORS[colorIdx];
      return `<div class="speaker-segment">` +
        `<div class="speaker-label">` +
          `<span class="speaker-name" style="color:${color}">${speakerLabel}</span>` +
        `</div>` +
        `<div class="speaker-text">${content}</div>` +
      `</div>`;
    }
    if (line.trim()) {
      return `<div>${escapeHtml(line)}</div>`;
    }
    return '';
  }).join('');
}

function renderTranscript(element, text) {
  if (hasSpeakerLabels(text)) {
    element.innerHTML = formatSpeakerTranscript(text);
  } else {
    element.textContent = text;
  }
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

let permissions = { mic: false, system_audio: false };
let appSettings = null;

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

    if (detailTitleInput && selectedRecordingId) {
      try {
        await invoke('update_title', { recordingId: selectedRecordingId, title: detailTitleInput.value });
      } catch (e) {
        console.error('Title sync failed (non-fatal):', e);
      }
    }

    await invoke("stop_recording");
    isRecording = false;

    stopTimer();
    stopWaveformAnimation();
    setRecordingUI(false);

    await loadRecordings();

    if (selectedRecordingId === currentId) {
      showDetailView(currentId);
    }

  } catch (error) {
    // Always reset UI state, even on error
    isRecording = false;
    stopTimer();
    stopWaveformAnimation();
    setRecordingUI(false);
    console.error("Failed to stop:", error);
    if (error && error.includes && error.includes("discarded")) {
      hideDetailView();
      await loadRecordings();
    }
  } finally {
    isRecordingBusy = false;
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

  // Hide transcript/structured sections if currently recording or processing
  const hideContent = isRecording || isProcessing;
  const contentGrid = document.getElementById('detail-content-grid');
  if (contentGrid) contentGrid.style.display = hideContent ? 'none' : 'flex';

  // Load Transcript only if not recording/processing
  if (!hideContent && detailTranscriptEl) {
    detailTranscriptEl.textContent = "Loading...";
    detailTranscriptEl.classList.remove('empty');

    try {
      const transcript = await invoke("get_transcript", { recordingId: id });
      if (transcript) {
        renderTranscript(detailTranscriptEl, transcript);
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

    // Listen for transcription progress (FluidAudio stages)
    window.__TAURI__.event.listen('transcription_progress', (event) => {
      const { recording_id, stage, percent } = event.payload;
      // Only update if this is our current transcription
      if (recording_id === transcribingRecordingId) {
        const btn = document.getElementById('process-btn');
        if (btn && btn.disabled) {
          // Show percent only for actual processing stages (not downloading)
          const text = percent > 0 ? `${stage} ${percent}%` : stage;
          btn.innerHTML = `<span style="font-weight: 600; font-size: 12px;">${escapeHtml(text)}</span>`;
        }
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
        renderTranscript(detailTranscriptEl, transcript);
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
  } else if (provider === "FluidAudio") {
    // FluidAudio needs no config — hide both sections
    providerLocalSection.style.display = 'none';
    providerApiSection.style.display = 'none';
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
const slackIntegrationsListEl = document.getElementById('slack-integrations-list');

async function loadSlackIntegrations() {
  try {
    slackIntegrations = await invoke('list_slack_integrations');
    renderSlackIntegrationsList();
  } catch (err) {
    console.error('Failed to load Slack integrations:', err);
  }
}

function renderSlackIntegrationsList() {
  if (!slackIntegrationsListEl) return;
  
  const entries = Object.entries(slackIntegrations);
  if (entries.length === 0) {
    slackIntegrationsListEl.innerHTML = '<div style="text-align: center; padding: 40px 0; color: var(--text-secondary); opacity: 0.6;">No Slack workspaces connected yet</div>';
    return;
  }
  
  slackIntegrationsListEl.innerHTML = entries.map(([id, data]) => {
    const safeName = escapeHtml(data.name);
    const safeWorkspace = escapeHtml(data.workspace_name || 'Unknown workspace');
    return `
      <div class="integration-item" data-id="${escapeHtml(id)}" style="
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 16px;
        background: var(--surface-color);
        border: 1px solid var(--border-color);
        border-radius: 8px;
      ">
        <div style="flex: 1;">
          <div style="font-weight: 600; margin-bottom: 4px;">${safeName}</div>
          <div style="font-size: 0.75rem; color: var(--text-secondary); opacity: 0.8; font-family: 'SF Mono', monospace;">${safeWorkspace}</div>
        </div>
        <div style="display: flex; gap: 8px;">
          <button class="mini-action-btn test-slack-btn" data-id="${escapeHtml(id)}">Test</button>
          <button class="mini-action-btn danger remove-slack-btn" data-id="${escapeHtml(id)}">Remove</button>
        </div>
      </div>
    `;
  }).join('');
  
  // Attach event listeners
  slackIntegrationsListEl.querySelectorAll('.test-slack-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      btn.disabled = true;
      btn.textContent = 'Testing...';
      try {
        const workspaceName = await invoke('test_slack_integration', { id });
        alert(`Connected to: ${workspaceName}`);
      } catch (err) {
        alert(`Connection failed: ${err}`);
      } finally {
        btn.disabled = false;
        btn.textContent = 'Test';
      }
    });
  });
  
  slackIntegrationsListEl.querySelectorAll('.remove-slack-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = btn.dataset.id;
      const integration = slackIntegrations[id];
      if (!confirm(`Remove Slack workspace "${integration.name}"?`)) return;
      
      try {
        await invoke('remove_slack_integration', { id });
        await loadSlackIntegrations();
      } catch (err) {
        alert(`Failed to remove: ${err}`);
      }
    });
  });
}

if (addSlackBtn) {
  addSlackBtn.addEventListener('click', () => {
    slackNameInput.value = '';
    slackTokenInput.value = '';
    addSlackModal.style.display = 'flex';
  });
}

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

// ===== PIPELINE DEFINITION MANAGEMENT =====
let allPipelineDefs = [];
let editingPipelineDef = null; // null = new, string = editing name
let pipelineEditorSteps = []; // Working copy of steps

const pipelineDefsListEl = document.getElementById('pipeline-defs-list');
const addPipelineDefBtn = document.getElementById('add-pipeline-def-btn');
const pipelineEditor = document.getElementById('pipeline-editor');
const pipelineEditorTitle = document.getElementById('pipeline-editor-title');
const pipelineEditorName = document.getElementById('pipeline-editor-name');
const pipelineEditorDesc = document.getElementById('pipeline-editor-desc');
const pipelineStepsListEl = document.getElementById('pipeline-steps-list');
const addPipelineStepBtn = document.getElementById('add-pipeline-step-btn');
const pipelinePreviewEl = document.getElementById('pipeline-preview');
const savePipelineDefBtn = document.getElementById('save-pipeline-def-btn');
const deletePipelineDefBtn = document.getElementById('delete-pipeline-def-btn');
const closePipelineEditorBtn = document.getElementById('close-pipeline-editor');

async function loadPipelineDefs() {
  try {
    allPipelineDefs = await invoke('list_pipelines');
    renderPipelineDefsList();
    updateSidebarCounts();
  } catch (err) {
    console.error('Failed to load pipelines:', err);
  }
}

function renderPipelineDefsList() {
  if (!pipelineDefsListEl) return;
  if (allPipelineDefs.length === 0) {
    pipelineDefsListEl.innerHTML = '<div style="color: var(--text-secondary); opacity: 0.6; font-size: 0.85rem;">No pipelines yet.</div>';
    return;
  }
  pipelineDefsListEl.innerHTML = allPipelineDefs.map(p => {
    const safeName = escapeHtml(p.name);
    const safeDesc = escapeHtml(p.description || '');
    return `
    <div class="pipeline-def-item" data-name="${safeName}">
      <div class="pipeline-def-info">
        <div class="pipeline-def-name">${safeName}</div>
        <div class="pipeline-def-desc">${safeDesc} &middot; ${p.steps.length} step${p.steps.length !== 1 ? 's' : ''}</div>
      </div>
    </div>
  `;
  }).join('');

  pipelineDefsListEl.querySelectorAll('.pipeline-def-item').forEach(el => {
    el.addEventListener('click', () => openPipelineEditor(el.dataset.name));
  });
}

function openPipelineEditor(name) {
  if (!pipelineEditor) return;
  if (name) {
    const p = allPipelineDefs.find(p => p.name === name);
    if (!p) return;
    editingPipelineDef = name;
    pipelineEditorTitle.textContent = 'Edit Pipeline';
    pipelineEditorName.value = p.name;
    pipelineEditorDesc.value = p.description || '';
    pipelineEditorSteps = JSON.parse(JSON.stringify(p.steps));
    if (deletePipelineDefBtn) deletePipelineDefBtn.style.display = 'inline-block';
  } else {
    editingPipelineDef = null;
    pipelineEditorTitle.textContent = 'New Pipeline';
    pipelineEditorName.value = '';
    pipelineEditorDesc.value = '';
    pipelineEditorSteps = [];
    if (deletePipelineDefBtn) deletePipelineDefBtn.style.display = 'none';
  }
  pipelineEditor.style.display = 'block';
  renderPipelineSteps();
  renderPipelinePreview();
  pipelineEditorName.focus();
}

function closePipelineEditor() {
  if (pipelineEditor) pipelineEditor.style.display = 'none';
  editingPipelineDef = null;
  pipelineEditorSteps = [];
}

function renderPipelineSteps() {
  if (!pipelineStepsListEl) return;
  if (pipelineEditorSteps.length === 0) {
    pipelineStepsListEl.innerHTML = '<div style="color: var(--text-secondary); opacity: 0.5; font-size: 0.85rem;">No steps. Click "+ Add Step" to begin.</div>';
    return;
  }

  pipelineStepsListEl.innerHTML = pipelineEditorSteps.map((step, i) => {
    const inputLabel = step.input === 'transcript' ? 'transcript' : escapeHtml(step.input);
    const safeName = escapeHtml(step.name || 'Unnamed');
    const safeConnector = escapeHtml(step.connector);
    const safeDesc = step.description ? escapeHtml(step.description) : '';
    return `
      <div class="pipeline-step-item" draggable="true" data-index="${i}">
        <span class="step-drag-handle">&#9776;</span>
        <span class="step-number">${i + 1}</span>
        <div class="step-info" data-index="${i}" style="cursor: pointer;">
          <div class="step-name-row">
            <span class="step-name">${safeName}</span>
            <span class="step-connector-badge">${safeConnector}</span>
          </div>
          ${safeDesc ? `<div class="step-description">${safeDesc}</div>` : ''}
          <div class="step-input-label">input: ${inputLabel}</div>
        </div>
        <button class="step-remove-btn" data-index="${i}" title="Remove step">&times;</button>
      </div>
    `;
  }).join('');

  // Drag-and-drop handlers
  const items = pipelineStepsListEl.querySelectorAll('.pipeline-step-item');
  let dragSrcIndex = null;

  items.forEach(item => {
    item.addEventListener('dragstart', (e) => {
      dragSrcIndex = parseInt(item.dataset.index);
      item.classList.add('dragging');
      e.dataTransfer.effectAllowed = 'move';
    });
    item.addEventListener('dragend', () => {
      item.classList.remove('dragging');
      items.forEach(el => el.classList.remove('drag-over'));
    });
    item.addEventListener('dragover', (e) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'move';
      item.classList.add('drag-over');
    });
    item.addEventListener('dragleave', () => {
      item.classList.remove('drag-over');
    });
    item.addEventListener('drop', (e) => {
      e.preventDefault();
      const dropIndex = parseInt(item.dataset.index);
      if (dragSrcIndex !== null && dragSrcIndex !== dropIndex) {
        const [moved] = pipelineEditorSteps.splice(dragSrcIndex, 1);
        pipelineEditorSteps.splice(dropIndex, 0, moved);
        // Fix input references after reorder
        fixStepInputs();
        renderPipelineSteps();
        renderPipelinePreview();
      }
      items.forEach(el => el.classList.remove('drag-over'));
    });
  });

  // Click step to edit
  pipelineStepsListEl.querySelectorAll('.step-info').forEach(el => {
    el.addEventListener('click', () => showStepEditor(parseInt(el.dataset.index)));
  });

  // Remove step
  pipelineStepsListEl.querySelectorAll('.step-remove-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const idx = parseInt(btn.dataset.index);
      pipelineEditorSteps.splice(idx, 1);
      fixStepInputs();
      renderPipelineSteps();
      renderPipelinePreview();
    });
  });
}

function fixStepInputs() {
  // Fix input references: first step must reference 'transcript'
  // Others can reference 'transcript' or a previous step name
  for (let i = 0; i < pipelineEditorSteps.length; i++) {
    const step = pipelineEditorSteps[i];
    if (i === 0) {
      step.input = 'transcript';
    } else {
      const validInputs = ['transcript', ...pipelineEditorSteps.slice(0, i).map(s => s.name)];
      if (!validInputs.includes(step.input)) {
        step.input = pipelineEditorSteps[i - 1].name || 'transcript';
      }
    }
  }
}

function getInputOptions(stepIndex) {
  const options = ['transcript'];
  for (let i = 0; i < stepIndex; i++) {
    if (pipelineEditorSteps[i].name) options.push(pipelineEditorSteps[i].name);
  }
  return options;
}

function showStepEditor(index) {
  const step = pipelineEditorSteps[index];
  if (!step) return;

  const inputOptions = getInputOptions(index).map(o =>
    `<option value="${escapeHtml(o)}" ${step.input === o ? 'selected' : ''}>${escapeHtml(o)}</option>`
  ).join('');

  const promptTemplateOptions = allPromptTemplates.map(t =>
    `<option value="${escapeHtml(t.name)}" ${step.config?.prompt_template === t.name ? 'selected' : ''}>${escapeHtml(t.name)}</option>`
  ).join('');

  // Build connector-specific config fields
  let configFields = '';
  if (step.connector === 'llm') {
    configFields = `
      <div class="step-editor-row"><label>Prompt</label><select data-field="prompt_template"><option value="">Select template...</option>${promptTemplateOptions}</select></div>
      <div class="step-editor-row"><label>Provider</label><select data-field="provider">
        <option value="openai" ${step.config?.provider === 'openai' ? 'selected' : ''}>OpenAI</option>
        <option value="google" ${step.config?.provider === 'google' ? 'selected' : ''}>Google</option>
        <option value="anthropic" ${step.config?.provider === 'anthropic' ? 'selected' : ''}>Anthropic</option>
      </select></div>
      <div class="step-editor-row"><label>Model</label><input data-field="model" value="${escapeHtml(step.config?.model || '')}" placeholder="e.g. gpt-4o" /></div>
    `;
  } else if (step.connector === 'save') {
    configFields = `
      <div class="step-editor-row"><label>Path</label><input data-field="path" value="${escapeHtml(step.config?.path || '')}" placeholder="~/Documents/{date}-{pipeline-name}.md" /></div>
    `;
  } else if (step.connector === 'webhook') {
    configFields = `
      <div class="step-editor-row"><label>URL</label><input data-field="url" value="${escapeHtml(step.config?.url || '')}" placeholder="https://hooks.example.com/..." /></div>
      <div class="step-editor-row"><label>Method</label><select data-field="method">
        <option value="POST" ${step.config?.method === 'POST' ? 'selected' : ''}>POST</option>
        <option value="PUT" ${step.config?.method === 'PUT' ? 'selected' : ''}>PUT</option>
        <option value="PATCH" ${step.config?.method === 'PATCH' ? 'selected' : ''}>PATCH</option>
      </select></div>
    `;
  } else if (step.connector === 'slack') {
    const slackIntegrationOptions = Object.entries(slackIntegrations).map(([id, data]) => 
      `<option value="${escapeHtml(id)}" ${step.config?.integration_id === id ? 'selected' : ''}>${escapeHtml(data.name)}</option>`
    ).join('');
    configFields = `
      <div class="step-editor-row"><label>Workspace</label><select data-field="integration_id">
        <option value="">Select workspace...</option>
        ${slackIntegrationOptions}
      </select></div>
      <div class="step-editor-row"><label>Target</label><input data-field="target" value="${escapeHtml(step.config?.target || '')}" placeholder="#channel, email@example.com, or U123456" /></div>
      <div class="step-editor-row"><label>Thread TS (optional)</label><input data-field="thread_ts" value="${escapeHtml(step.config?.thread_ts || '')}" placeholder="1234567890.123456" /></div>
    `;
  } else if (step.connector === 'mcp') {
    configFields = `
      <div class="step-editor-row"><label>Server</label><input data-field="server" value="${escapeHtml(step.config?.server || '')}" placeholder="e.g. slack-mcp" /></div>
      <div class="step-editor-row"><label>Tool</label><input data-field="tool" value="${escapeHtml(step.config?.tool || '')}" placeholder="e.g. send-message" /></div>
      <div class="step-editor-row"><label>Args</label><textarea data-field="args" rows="2" placeholder='{"channel": "#team"}'>${escapeHtml(step.config?.args ? JSON.stringify(step.config.args, null, 2) : '')}</textarea></div>
    `;
  }

  // Replace step item (or existing editor) with new editor
  const stepChildren = pipelineStepsListEl.querySelectorAll('.pipeline-step-item, .step-editor');
  const stepEl = stepChildren[index];
  if (!stepEl) return;

  const editorEl = document.createElement('div');
  editorEl.className = 'step-editor';
  editorEl.innerHTML = `
    <div class="step-editor-row"><label>Name</label><input data-field="name" value="${escapeHtml(step.name)}" placeholder="step name" /></div>
    <div class="step-editor-row"><label>Connector</label><select data-field="connector">
      <option value="llm" ${step.connector === 'llm' ? 'selected' : ''}>LLM</option>
      <option value="save" ${step.connector === 'save' ? 'selected' : ''}>Save</option>
      <option value="webhook" ${step.connector === 'webhook' ? 'selected' : ''}>Webhook</option>
      <option value="slack" ${step.connector === 'slack' ? 'selected' : ''}>Slack</option>
      <option value="mcp" ${step.connector === 'mcp' ? 'selected' : ''}>MCP</option>
    </select></div>
    <div class="step-editor-row"><label>Input</label><select data-field="input">${inputOptions}</select></div>
    <div class="step-editor-row"><label>Description</label><input data-field="description" value="${escapeHtml(step.description || '')}" placeholder="What this step does..." /></div>
    <div id="step-config-fields">${configFields}</div>
    <div class="step-editor-actions">
      <button class="mini-action-btn compact-add-btn step-editor-done">Done</button>
    </div>
  `;

  stepEl.replaceWith(editorEl);

  // Connector change → re-render config fields
  const connectorSelect = editorEl.querySelector('[data-field="connector"]');
  connectorSelect.addEventListener('change', () => {
    step.connector = connectorSelect.value;
    step.config = {};
    showStepEditor(index);
  });

  // Done button
  editorEl.querySelector('.step-editor-done').addEventListener('click', () => {
    // Read values back
    step.name = editorEl.querySelector('[data-field="name"]').value.trim();
    step.connector = editorEl.querySelector('[data-field="connector"]').value;
    step.input = editorEl.querySelector('[data-field="input"]').value;
    step.description = editorEl.querySelector('[data-field="description"]').value.trim() || null;

    // Read connector-specific config
    const configFieldsEl = editorEl.querySelector('#step-config-fields');
    step.config = {};
    configFieldsEl.querySelectorAll('[data-field]').forEach(field => {
      const key = field.dataset.field;
      let val = field.value.trim();
      if (key === 'args') {
        try { val = JSON.parse(val); } catch { val = {}; }
      }
      if (val !== '') step.config[key] = val;
    });

    renderPipelineSteps();
    renderPipelinePreview();
  });
}

function renderPipelinePreview() {
  if (!pipelinePreviewEl) return;
  if (pipelineEditorSteps.length === 0) {
    pipelinePreviewEl.innerHTML = '<span class="pipeline-preview-empty">Add steps to see preview</span>';
    return;
  }

  let html = '<span class="preview-node source">transcript</span>';
  for (const step of pipelineEditorSteps) {
    html += '<span class="preview-arrow">&rarr;</span>';
    html += `<span class="preview-node step">${escapeHtml(step.name || '?')} <small style="opacity:0.6">(${escapeHtml(step.connector)})</small></span>`;
  }
  pipelinePreviewEl.innerHTML = html;
}

if (addPipelineDefBtn) addPipelineDefBtn.addEventListener('click', () => openPipelineEditor(null));
if (closePipelineEditorBtn) closePipelineEditorBtn.addEventListener('click', closePipelineEditor);

if (addPipelineStepBtn) {
  addPipelineStepBtn.addEventListener('click', () => {
    const prevStepName = pipelineEditorSteps.length > 0
      ? pipelineEditorSteps[pipelineEditorSteps.length - 1].name
      : null;
    pipelineEditorSteps.push({
      name: '',
      connector: 'llm',
      input: prevStepName || 'transcript',
      config: {},
      description: null
    });
    renderPipelineSteps();
    renderPipelinePreview();
    // Auto-open editor for new step
    showStepEditor(pipelineEditorSteps.length - 1);
  });
}

if (savePipelineDefBtn) {
  savePipelineDefBtn.addEventListener('click', async () => {
    const name = pipelineEditorName.value.trim();
    const desc = pipelineEditorDesc.value.trim();
    if (!name) { alert('Pipeline name is required'); return; }
    if (pipelineEditorSteps.length === 0) { alert('Pipeline must have at least one step'); return; }

    // Validate step names
    for (let i = 0; i < pipelineEditorSteps.length; i++) {
      if (!pipelineEditorSteps[i].name.trim()) {
        alert(`Step ${i + 1} needs a name`);
        return;
      }
    }

    try {
      const pipeline = { name, description: desc, steps: pipelineEditorSteps };

      // If renaming, delete old first
      if (editingPipelineDef && editingPipelineDef !== name) {
        await invoke('delete_pipeline', { name: editingPipelineDef });
      }
      await invoke('save_pipeline', { pipeline });
      closePipelineEditor();
      await loadPipelineDefs();
    } catch (err) {
      console.error('Failed to save pipeline:', err);
      alert('Failed to save: ' + err);
    }
  });
}

if (deletePipelineDefBtn) {
  deletePipelineDefBtn.addEventListener('click', async () => {
    if (!editingPipelineDef) return;
    if (!confirm(`Delete pipeline "${editingPipelineDef}"?`)) return;
    try {
      await invoke('delete_pipeline', { name: editingPipelineDef });
      closePipelineEditor();
      await loadPipelineDefs();
    } catch (err) {
      console.error('Failed to delete pipeline:', err);
      alert('Failed to delete: ' + err);
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

init().catch(e => console.error('Init failed:', e));
