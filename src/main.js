const { invoke } = window.__TAURI__.core;

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

let allRecordings = [];
let selectedTags = []; // Current filter tags
let selectedRecordingId = null;
let currentRecordingTags = []; // Tags of the recording being viewed/edited

let permissions = { mic: false, system_audio: false };
let appSettings = null;

// ===== DOM ELEMENTS =====
const statusIndicator = document.getElementById("status-indicator");
const timerDisplay = document.getElementById("timer");
const recordToggleBtn = document.getElementById("record-toggle-btn");

const activeTagChipsEl = document.getElementById("active-tag-chips");
const allTagsListEl = document.getElementById("all-tags-list");

const recordingsListEl = document.getElementById("recordings-list");
const emptyStateEl = document.getElementById("empty-state");
const detailViewEl = document.getElementById("detail-view");
const appLayoutEl = document.querySelector(".app-layout");
const backBtn = document.getElementById("back-btn");

const detailTitleInput = document.getElementById("detail-title");
const detailMetaHeaderEl = document.getElementById("detail-meta-header");
const detailTagsInput = document.getElementById("detail-tags-input");
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
  if (isRecording) {
    await stopRecording();
  } else {
    await startRecording();
  }
}

async function startRecording() {
  ViewManager.showRecordings();

  const tags = [...selectedTags];
  const saveMixOnly = appSettings?.save_mix_only !== false; // default true
  try {
    const metadata = await invoke("start_recording", { tags, saveMixOnly });
    isRecording = true;

    if (statusIndicator) statusIndicator.className = "status-recording";
    document.body.classList.add("is-recording-active");
    if (recordToggleBtn) {
      recordToggleBtn.innerHTML = "⏹";
      recordToggleBtn.classList.add("is-active");
      recordToggleBtn.title = "Stop Recording";
    }

    await loadRecordings();
    startTimer();
    startWaveformAnimation();
    showDetailView(metadata.id);

  } catch (error) {
    console.error("Failed to start recording:", error);
    alert("Failed to start: " + error);
  }
}

async function stopRecording() {
  try {
    const currentId = selectedRecordingId;

    // Explicitly sync title before stopping
    if (detailTitleInput && selectedRecordingId) {
      await invoke('update_title', { recordingId: selectedRecordingId, title: detailTitleInput.value });
    }

    await invoke("stop_recording");
    isRecording = false;

    stopTimer();
    stopWaveformAnimation();
    if (statusIndicator) statusIndicator.className = "status-idle";
    document.body.classList.remove("is-recording-active");
    if (recordToggleBtn) {
      recordToggleBtn.innerHTML = "⏺";
      recordToggleBtn.classList.remove("is-active");
      recordToggleBtn.title = "Start Recording";
    }

    await loadRecordings();

    if (selectedRecordingId === currentId) {
      showDetailView(currentId);
    }

  } catch (error) {
    console.error("Failed to stop:", error);
    if (error && error.includes && error.includes("discarded")) {
      hideDetailView();
      await loadRecordings();
    }
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

    // Update Warning Banner
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

// ===== TAG FILTERING =====
function getUniqueTags() {
  const tagsMap = new Map();
  allRecordings.forEach(rec => {
    if (rec.tags) {
      rec.tags.forEach(tag => {
        tagsMap.set(tag, (tagsMap.get(tag) || 0) + 1);
      });
    }
  });
  return tagsMap;
}

function renderTags() {
  if (!allTagsListEl) return;
  const tagsMap = getUniqueTags();
  const sortedTags = Array.from(tagsMap.entries()).sort((a, b) => b[1] - a[1]);

  allTagsListEl.innerHTML = sortedTags.map(([tag, count]) => {
    const isActive = selectedTags.includes(tag);
    return `
      <div class="sidebar-tag-item ${isActive ? 'active' : ''}" onclick="toggleTagFilter('${tag}')">
        <span>#${tag}</span>
        <div style="display: flex; align-items: center; gap: 8px;">
          <span class="tag-count">${count}</span>
          ${isActive ? '<span class="tag-remove-sidebar">×</span>' : ''}
        </div>
      </div>
  `;
  }).join("");
}

window.toggleTagFilter = (tag) => {
  const index = selectedTags.indexOf(tag);
  if (index > -1) {
    selectedTags.splice(index, 1);
  } else {
    selectedTags.push(tag);
  }
  renderTags();
  renderRecordingsList();
};

// ===== RECORDINGS LIST =====
async function loadRecordings() {
  try {
    const recordings = await invoke("list_recordings");
    allRecordings = recordings || [];
    renderTags();
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
  const filtered = selectedTags.length === 0
    ? allRecordings
    : allRecordings.filter(rec => selectedTags.every(t => rec.tags && rec.tags.includes(t)));

  if (filtered.length === 0) {
    recordingsListEl.innerHTML = "";
    if (emptyStateEl) emptyStateEl.style.display = "block";
    return;
  }
  if (emptyStateEl) emptyStateEl.style.display = "none";
  recordingsListEl.innerHTML = filtered.map(rec => {
    const isProcessing = rec.status === 'processing';
    const isCurrentlyRecording = isRecording && selectedRecordingId === rec.id;
    const metaText = isProcessing ? '<span style="color:var(--accent)">Processing...</span>' : formatDuration(getDuration(rec));

    // Health indicator
    const hasIssues = rec.health && rec.health.status !== 'ok';
    const healthIcon = hasIssues ? '<span class="health-warning" title="Issues occurred during recording">⚠️</span>' : '';

    return `
    <div class="recording-item ${isCurrentlyRecording ? 'recording-active' : ''}" onclick="showDetailView('${rec.id}')">
        <div class="recording-item-header">
          <div class="recording-title">${healthIcon}${rec.title || "Untitled"}${isCurrentlyRecording ? ' <span style="color:var(--accent)">●</span>' : ''}</div>
          <div class="recording-meta">
            <span>${new Date(rec.created_at).toLocaleString(undefined, dateOptions)}</span>
            <span>·</span>
            <span>${metaText}</span>
          </div>
        </div>
        <div class="recording-tags">
          ${(rec.tags || []).map(tag => `<span class="recording-tag">#${tag}</span>`).join("")}
        </div>
      </div>
    `;
  }).join("");
}

function getDuration(rec) {
  if (!rec.audio) return 0;
  return rec.audio.mic?.duration_sec || rec.audio.system?.duration_sec || 0;
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
  currentRecordingTags = [...(rec.tags || [])];

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

  renderTagChips();

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

  // POLLING if processing
  if (isProcessing) {
    setTimeout(async () => {
      if (selectedRecordingId === id) {
        await loadRecordings();
        // Only re-call showDetailView if status actually changed to avoid flicker? 
        // loadRecordings updates allRecordings.
        const updated = allRecordings.find(r => r.id === id);
        if (updated && updated.status !== 'processing') {
          showDetailView(id);
        } else if (updated) {
          // Still processing, update timer/visuals if needed, or just poll again
          // Recursive poll
          showDetailView(id);
        }
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
        detailTranscriptEl.textContent = transcript;
        detailTranscriptEl.classList.remove('empty');
      } else {
        detailTranscriptEl.textContent = "Not processed yet.";
        detailTranscriptEl.classList.add('empty');
      }
    } catch (err) {
      console.error("Failed to load transcript:", err);
      detailTranscriptEl.textContent = "Not processed yet.";
      detailTranscriptEl.classList.add('empty');
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
  ViewManager.showRecordings();
  if (detailControlsEl) detailControlsEl.style.display = 'none';
}

function renderTagChips() {
  const listEl = document.getElementById('detail-tags-list');
  if (!listEl) return;
  listEl.innerHTML = currentRecordingTags.map(tag => `
    <div class="detail-tag-chip">
    #${tag}
<span class="tag-remove" onclick="removeRecordingTag('${tag}')">×</span>
    </div>
  `).join('');
}

window.removeRecordingTag = async (tag) => {
  currentRecordingTags = currentRecordingTags.filter(t => t !== tag);
  renderTagChips();
  await updateRecordingTagsOnBackend();
};

async function updateRecordingTagsOnBackend() {
  if (!selectedRecordingId) return;
  try {
    await invoke('update_tags', { recordingId: selectedRecordingId, tags: currentRecordingTags });
    await loadRecordings();
  } catch (err) { console.error(err); }
}

const tagSuggestionsEl = document.getElementById("tag-suggestions");

function renderTagSuggestions() {
  const tagsMap = getUniqueTags();
  // Sort by frequency, take top 10
  const sorted = Array.from(tagsMap.entries())
    .sort((a, b) => b[1] - a[1])
    .filter(([tag]) => !currentRecordingTags.includes(tag))
    .slice(0, 10);

  if (sorted.length === 0) {
    tagSuggestionsEl.style.display = "none";
    return;
  }

  tagSuggestionsEl.innerHTML = sorted.map(([tag, count]) => `
    <div class="suggestion-item" onclick="selectSuggestedTag('${tag}')">
      <span>#${tag}</span>
      <span class="count">${count}</span>
    </div>
  `).join("");
  tagSuggestionsEl.style.display = "block";
}

window.selectSuggestedTag = async (tag) => {
  if (!currentRecordingTags.includes(tag)) {
    currentRecordingTags.push(tag);
    renderTagChips();
    await updateRecordingTagsOnBackend();
  }
  tagSuggestionsEl.style.display = "none";
  detailTagsInput.value = "";
};

if (detailTagsInput) {
  detailTagsInput.addEventListener('focus', renderTagSuggestions);

  // Hide suggestions when clicking outside, but allow time for selection click
  detailTagsInput.addEventListener('blur', () => {
    setTimeout(() => { tagSuggestionsEl.style.display = "none"; }, 200);
  });

  detailTagsInput.addEventListener('keypress', async (e) => {
    if (e.key === 'Enter') {
      const val = e.target.value.trim().toLowerCase().replace(/^#/, '');
      if (val) {
        if (!currentRecordingTags.includes(val)) {
          currentRecordingTags.push(val);
          renderTagChips();
          await updateRecordingTagsOnBackend();
        }
        e.target.value = '';
        tagSuggestionsEl.style.display = "none";
      }
    }
  });
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

const deleteBtnHeader = document.getElementById('delete-btn-header');
if (deleteBtnHeader) {
  deleteBtnHeader.addEventListener('click', () => {
    if (!selectedRecordingId) return;
    deleteModal.style.display = 'flex';
  });
}

if (cancelDeleteBtn) {
  cancelDeleteBtn.addEventListener('click', () => {
    deleteModal.style.display = 'none';
  });
}

if (confirmDeleteBtn) {
  confirmDeleteBtn.addEventListener('click', async () => {
    if (!selectedRecordingId) return;
    await invoke('delete_recording', { recordingId: selectedRecordingId });
    deleteModal.style.display = 'none';
    hideDetailView();
    await loadRecordings();
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
    if (!selectedRecordingId) return;
    const folderPath = `${appSettings.storage_path}/${selectedRecordingId}`;
    await window.__TAURI_PLUGIN_OPENER__.openPath(folderPath);
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

  // Note: Using window.__TAURI__.event.listen for simplicity if available
  try {
    window.__TAURI__.event.listen('transcription_segment', (event) => {
      const segmentText = event.payload;
      if (detailTranscriptEl) {
        if (detailTranscriptEl.classList.contains('empty')) {
          detailTranscriptEl.textContent = '';
          detailTranscriptEl.classList.remove('empty');
        }
        // Append new segment
        detailTranscriptEl.textContent += segmentText + ' ';

        // Auto-scroll to bottom of the detail container
        const scroller = detailTranscriptEl.closest('.detail-scroller');
        if (scroller) scroller.scrollTop = scroller.scrollHeight;
      }
    });
  } catch (e) {
    console.error("Failed to setup transcription listener:", e);
  }
}

const prBtn = document.getElementById('process-btn');
if (prBtn) {
  prBtn.addEventListener('click', async () => {
    if (!selectedRecordingId) return;

    try {
      // Show loading state
      prBtn.disabled = true;
      prBtn.innerHTML = '<span style="font-weight: 600; font-size: 12px;">Processing...</span>';

      if (detailTranscriptEl) {
        detailTranscriptEl.textContent = ''; // Clear for live segments
        detailTranscriptEl.classList.remove('empty');
      }

      // Call backend - segments will start arriving via the listener above
      const transcript = await invoke('transcribe_recording', { recordingId: selectedRecordingId });

      // Final update to ensure everything is matched correctly
      if (detailTranscriptEl) {
        detailTranscriptEl.textContent = transcript;
        detailTranscriptEl.classList.remove('empty');
      }

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

// ===== SIMPLE AUDIO PLAYER =====
const playPauseBtn = document.getElementById('play-pause-btn');
const currentTimeEl = document.getElementById('current-time');
const totalTimeEl = document.getElementById('total-time');
let isPlaying = false;
let audioDurationMs = 0;
let playbackPollInterval = null;

function formatDurationShort(seconds) {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

async function loadAudioDuration(recordingId) {
  try {
    // Get duration from recording metadata
    const recordings = await invoke('list_recordings');
    const rec = recordings.find(r => r.id === recordingId);
    if (rec) {
      const duration = rec.audio?.mix?.duration_sec || rec.audio?.mic?.duration_sec || 0;
      audioDurationMs = duration * 1000;
      if (totalTimeEl) {
        totalTimeEl.textContent = formatDurationShort(duration);
      }
    }
  } catch (err) {
    console.error('Failed to load audio info:', err);
  }
}

/**
 * Start polling playback state to detect when playback finishes
 */
function startPlaybackPolling() {
  if (playbackPollInterval) return;

  playbackPollInterval = setInterval(async () => {
    if (!isPlaying) {
      stopPlaybackPolling();
      return;
    }

    try {
      const state = await invoke('get_playback_state');

      // Update current time display
      if (currentTimeEl && state.current_position_ms) {
        currentTimeEl.textContent = formatDurationShort(state.current_position_ms / 1000);
      }

      // Check if playback finished (status is "Stopped")
      if (state.status === 'Stopped' && isPlaying) {
        isPlaying = false;
        if (playPauseBtn) {
          playPauseBtn.innerHTML = '<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>';
        }
        if (currentTimeEl) currentTimeEl.textContent = '0:00';
        stopPlaybackPolling();
      }
    } catch (err) {
      // Ignore polling errors
    }
  }, 250); // Poll every 250ms
}

/**
 * Stop polling playback state
 */
function stopPlaybackPolling() {
  if (playbackPollInterval) {
    clearInterval(playbackPollInterval);
    playbackPollInterval = null;
  }
}

// Play/Pause button
if (playPauseBtn) {
  playPauseBtn.addEventListener('click', async () => {
    if (!selectedRecordingId) return;

    try {
      if (isPlaying) {
        await invoke('pause_audio');
        isPlaying = false;
        playPauseBtn.innerHTML = '<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>';
        stopPlaybackPolling();
      } else {
        await invoke('play_audio', { recordingId: selectedRecordingId });
        isPlaying = true;
        playPauseBtn.innerHTML = '<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="4" width="4" height="16"></rect><rect x="14" y="4" width="4" height="16"></rect></svg>';
        startPlaybackPolling();
      }
    } catch (err) {
      console.error('Playback error:', err);
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
    console.log("Loaded settings:", appSettings);

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
  document.body.classList.remove("neon-purple", "deep-obsidian", "deep-blue", "light-pastel");
  if (themeName !== "neon-purple") {
    document.body.classList.add(themeName);
  }

  appSettings.theme = themeName;

  themeButtons.forEach(btn => {
    btn.classList.toggle("active", btn.dataset.theme === themeName);
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

// ===== LOAD TEMPLATES =====
async function loadTemplates() {
  if (!templateSelect) return;

  try {
    const templates = await invoke('list_templates');
    templateSelect.innerHTML = '<option value="">Select template...</option>' +
      templates.map(t => `<option value="${t.name}">${t.description}</option>`).join('');
  } catch (err) {
    console.error('Failed to load templates:', err);
  }
}

// ===== INIT =====
async function init() {
  await loadSettings();
  await loadRecordings();
  await loadTemplates();
  try {
    const version = await invoke("get_app_version");
    const versionEl = document.getElementById("app-version");
    if (versionEl) versionEl.textContent = `v${version} `;
  } catch (err) {
    console.error("Failed to fetch version:", err);
  }

  await updatePermissionStatus();

  // Show onboarding only if not completed AND permissions are missing
  if (!appSettings.onboarding_completed && (!permissions.mic || !permissions.system_audio)) {
    onboardingOverlay.style.display = 'flex';
  }
}

init();
