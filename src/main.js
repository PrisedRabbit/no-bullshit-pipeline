const { invoke } = window.__TAURI__.core;

// ===== STATE =====
let timerInterval;
let startTime;
let isRecording = false;

let allRecordings = [];
let selectedTags = []; // Current filter tags
let selectedRecordingId = null;
let currentRecordingTags = []; // Tags of the recording being viewed/edited

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

// ===== TIMER =====
function formatTime(ms) {
  const totalSeconds = Math.floor(ms / 1000);
  const h = Math.floor(totalSeconds / 3600).toString().padStart(2, "0");
  const m = Math.floor((totalSeconds % 3600) / 60).toString().padStart(2, "0");
  const s = (totalSeconds % 60).toString().padStart(2, "0");
  return `${h}:${m}:${s}`;
}

function startTimer() {
  startTime = Date.now();
  timerInterval = setInterval(() => {
    const now = Date.now();
    timerDisplay.textContent = formatTime(now - startTime);

    if (isRecording && selectedRecordingId && detailViewEl.style.display !== 'none') {
      detailMetaEl.textContent = `Recording... · ${timerDisplay.textContent}`;
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
  const tags = [...selectedTags];
  try {
    const metadata = await invoke("start_recording", { tags });
    isRecording = true;

    if (statusIndicator) statusIndicator.className = "status-recording";
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

  renderActiveChips();
}

function renderActiveChips() {
  if (!activeTagChipsEl) return;
  if (selectedTags.length === 0) {
    activeTagChipsEl.innerHTML = "";
    return;
  }
  activeTagChipsEl.innerHTML = selectedTags.map(tag => `
    <div class="filter-chip">
      #${tag}
      <span class="remove" onclick="toggleTagFilter('${tag}')">×</span>
    </div>
  `).join("");
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
          <span>${new Date(rec.created_at).toLocaleString()}</span>
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
  return `${m}:${s.toString().padStart(2, "0")}`;
}

// ===== DETAIL VIEW =====
window.showDetailView = async (id) => {
  const rec = allRecordings.find(r => r.id === id);
  if (!rec) return;

  selectedRecordingId = id;
  currentRecordingTags = [...(rec.tags || [])];

  if (detailTitleInput) detailTitleInput.value = rec.title || "";

  if (detailMetaEl) {
    if (isRecording && id === selectedRecordingId) {
      detailMetaEl.textContent = `Recording... · ${timerDisplay.textContent}`;
    } else {
      detailMetaEl.textContent = `${new Date(rec.created_at).toLocaleString()} · ${formatDuration(getDuration(rec))}`;
    }
  }

  renderTagChips();

  if (detailTranscriptEl) {
    detailTranscriptEl.textContent = "Not processed yet.";
    detailTranscriptEl.classList.add('empty');
  }
  if (detailStructuredEl) {
    detailStructuredEl.textContent = "Not processed yet.";
    detailStructuredEl.classList.add('empty');
  }

  if (appLayoutEl) appLayoutEl.classList.add('detail-open');
  if (detailViewEl) detailViewEl.style.display = 'flex';
};

function hideDetailView() {
  selectedRecordingId = null;
  if (appLayoutEl) appLayoutEl.classList.remove('detail-open');
  if (detailViewEl) detailViewEl.style.display = 'none';
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

const deleteBtn = document.getElementById('delete-btn');
if (deleteBtn) {
  deleteBtn.addEventListener('click', async () => {
    if (!selectedRecordingId) return;
    if (confirm("Delete this recording?")) {
      await invoke('delete_recording', { recordingId: selectedRecordingId });
      hideDetailView();
      await loadRecordings();
    }
  });
}

const openFolderBtn = document.getElementById('open-folder-btn');
if (openFolderBtn) {
  openFolderBtn.addEventListener('click', async () => {
    if (!selectedRecordingId) return;
    const folderPath = `/Users/skopanev/nbp-data/${selectedRecordingId}`;
    await window.__TAURI_PLUGIN_OPENER__.openPath(folderPath);
  });
}

// Play/Process placeholders removed as requested, but keeping listeners safe if elements exist
const pBtn = document.getElementById('play-btn');
if (pBtn) pBtn.addEventListener('click', () => alert('Playback coming soon'));
const prBtn = document.getElementById('process-btn');
if (prBtn) prBtn.addEventListener('click', () => alert('AI Processing coming soon'));

// ===== INIT =====
async function init() {
  await loadRecordings();
  try {
    const version = await invoke("get_app_version");
    const versionEl = document.getElementById("app-version");
    if (versionEl) versionEl.textContent = `v${version}`;
  } catch (err) {
    console.error("Failed to fetch version:", err);
  }
}

init();
