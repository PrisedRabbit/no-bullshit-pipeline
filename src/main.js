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
const detailMetaEl = document.getElementById("detail-meta");
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
      detailMetaEl.textContent = `Recording... · ${timerDisplay.textContent} `;
    }
  }, 100);
}

function stopTimer() {
  clearInterval(timerInterval);
  timerDisplay.textContent = "00:00:00";
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
  try {
    const metadata = await invoke("start_recording", { tags });
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
    showDetailView(metadata.id);

  } catch (error) {
    console.error("Failed to start recording:", error);
    alert("Failed to start: " + error);
  }
}

async function stopRecording() {
  try {
    const currentId = selectedRecordingId;
    await invoke("stop_recording");
    isRecording = false;

    stopTimer();
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
    permissions = await invoke("check_permissions");

    // Update Onboarding UI
    const micItem = document.getElementById("perm-mic-item");
    const sysItem = document.getElementById("perm-sys-item");

    if (micItem) {
      const btn = micItem.querySelector(".modal-btn");
      btn.style.display = permissions.mic ? 'none' : 'block';
      micItem.querySelector(".perm-status-ok").style.display = permissions.mic ? 'block' : 'none';
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

  await invoke("request_mic_permission");
  btn.dataset.requested = "true";

  // Open settings right away to help user find the permission
  await invoke("open_privacy_settings", { pane: "mic" });

  // Poll a few times for mic
  for (let i = 0; i < 5; i++) {
    await new Promise(r => setTimeout(r, 1000));
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

  await invoke("request_system_audio_permission");
  btn.dataset.requested = "true";

  // System audio toggle often happens in settings, 
  // so we poll for 10 seconds to catch the change.
  for (let i = 0; i < 10; i++) {
    await new Promise(r => setTimeout(r, 1000));
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
  recordingsListEl.innerHTML = filtered.map(rec => `
  <div class="recording-item" onclick="showDetailView('${rec.id}')">
      <div class="recording-item-header">
        <div class="recording-title">${rec.title || "Untitled"}</div>
        <div class="recording-meta">
          <span>${new Date(rec.created_at).toLocaleString(undefined, dateOptions)}</span>
          <span>·</span>
          <span>${formatDuration(getDuration(rec))}</span>
        </div>
      </div>
      <div class="recording-tags">
        ${(rec.tags || []).map(tag => `<span class="recording-tag">#${tag}</span>`).join("")}
      </div>
    </div>
  `).join("");
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

const detailMetaHeaderEl = document.getElementById("detail-meta-header");
const detailControlsEl = document.getElementById("detail-controls");
const captureSectionEl = document.getElementById("capture-section");

// ===== DETAIL VIEW =====
window.showDetailView = async (id) => {
  const rec = allRecordings.find(r => r.id === id);
  if (!rec) return;

  selectedRecordingId = id;
  currentRecordingTags = [...(rec.tags || [])];

  if (detailTitleInput) detailTitleInput.value = rec.title || "";

  // Update Metadata in the Header
  if (detailMetaHeaderEl) {
    if (isRecording && id === selectedRecordingId) {
      // While recording, the capture bar is visible anyway, so this is handled by timer
    } else {
      detailMetaHeaderEl.textContent = `${new Date(rec.created_at).toLocaleString(undefined, dateOptions)} · ${formatDuration(getDuration(rec))} `;
    }
  }

  // Handle visibility
  ViewManager.showDetail();

  if (detailControlsEl) detailControlsEl.style.display = 'flex';

  renderTagChips();

  if (detailTranscriptEl) {
    detailTranscriptEl.textContent = "Not processed yet.";
    detailTranscriptEl.classList.add('empty');
  }
  if (detailStructuredEl) {
    detailStructuredEl.textContent = "Not processed yet.";
    detailStructuredEl.classList.add('empty');
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
  detailTitleInput.addEventListener('blur', async (e) => {
    if (!selectedRecordingId) return;
    await invoke('update_title', { recordingId: selectedRecordingId, title: e.target.value });
    await loadRecordings();
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
    const folderPath = `/ Users / skopanev / nbp - data / ${selectedRecordingId} `;
    await window.__TAURI_PLUGIN_OPENER__.openPath(folderPath);
  });
}

const pBtn = document.getElementById('play-btn');
if (pBtn) pBtn.addEventListener('click', () => alert('Playback coming soon'));
const prBtn = document.getElementById('process-btn');
if (prBtn) prBtn.addEventListener('click', () => alert('AI Processing coming soon'));

// ===== SETTINGS =====
async function loadSettings() {
  try {
    appSettings = await invoke("load_settings");
    if (storagePathInput) storagePathInput.value = appSettings.storage_path;
    if (cleanupThresholdInput) cleanupThresholdInput.value = appSettings.auto_discard_seconds;

    applyTheme(appSettings.theme);
  } catch (err) {
    console.error("Failed to load settings:", err);
  }
}

async function saveSettings() {
  try {
    appSettings.auto_discard_seconds = parseInt(cleanupThresholdInput.value) || 0;
    // storage_path is updated via browse

    await invoke("save_settings", { settings: appSettings });
    ViewManager.showRecordings();
    // After changing storage path, we should probably reload recordings
    await loadRecordings();
  } catch (err) {
    console.error("Failed to save settings:", err);
  }
}

function applyTheme(themeName) {
  document.body.classList.remove("neon-purple", "deep-obsidian");
  if (themeName !== "neon-purple") {
    document.body.classList.add(themeName);
  }

  appSettings.theme = themeName;

  themeButtons.forEach(btn => {
    btn.classList.toggle("active", btn.dataset.theme === themeName);
  });
}

// ===== SETTINGS EVENT LISTENERS =====
if (settingsBtn) settingsBtn.addEventListener("click", () => ViewManager.showSettings());
if (settingsBackBtn) settingsBackBtn.addEventListener("click", () => ViewManager.showRecordings());
if (saveSettingsBtn) saveSettingsBtn.addEventListener("click", saveSettings);

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

// ===== INIT =====
async function init() {
  await loadSettings();
  await loadRecordings();
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
