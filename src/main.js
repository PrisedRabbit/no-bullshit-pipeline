const { invoke } = window.__TAURI__.core;

// ===== STATE =====
let timerInterval;
let startTime;
let elapsedPausedTime = 0;
let isPaused = false;

let allRecordings = [];
let selectedTags = [];
let selectedRecordingId = null;
let currentRecordingTags = [];

// ===== DOM ELEMENTS =====
const statusIndicator = document.getElementById("status-indicator");
const timerDisplay = document.getElementById("timer");
const projectInput = document.getElementById("project-tags");
// audioSourceSelect removed
const recordBtn = document.getElementById("record-btn");
const pauseBtn = document.getElementById("pause-btn");
const stopBtn = document.getElementById("stop-btn");

const tagFiltersEl = document.getElementById("tag-filters");
const recordingsListEl = document.getElementById("recordings-list");
const emptyStateEl = document.getElementById("empty-state");
const detailViewEl = document.getElementById("detail-view");
const appLayoutEl = document.querySelector(".app-layout");
const backBtn = document.getElementById("back-btn");

// ===== TIMER =====
function formatTime(ms) {
  const totalSeconds = Math.floor(ms / 1000);
  const h = Math.floor(totalSeconds / 3600).toString().padStart(2, "0");
  const m = Math.floor((totalSeconds % 3600) / 60).toString().padStart(2, "0");
  const s = (totalSeconds % 60).toString().padStart(2, "0");
  return `${h}:${m}:${s}`;
}

function startTimer() {
  startTime = Date.now() - elapsedPausedTime;
  timerInterval = setInterval(() => {
    if (!isPaused) {
      const now = Date.now();
      timerDisplay.textContent = formatTime(now - startTime);
    }
  }, 100);
}

function stopTimer() {
  clearInterval(timerInterval);
  timerDisplay.textContent = "00:00:00";
  elapsedPausedTime = 0;
  isPaused = false;
}

// ===== RECORDING CONTROLS =====
async function startRecording() {
  const tags = projectInput.value.split(" ").filter(t => t.length > 0);

  try {
    // Calling backend (which now does dual recording automatically)
    await invoke("start_recording", { tags });
    console.log("Started recording", tags);

    statusIndicator.textContent = "Recording";
    statusIndicator.className = "status-recording";
    recordBtn.disabled = true;
    pauseBtn.disabled = false;
    stopBtn.disabled = false;
    // Selector removed
    pauseBtn.innerHTML = "⏸";
    pauseBtn.title = "Pause";

    startTimer();
  } catch (error) {
    console.error("Failed to start recording:", error);
    alert("Failed to start: " + error);
  }
}

async function pauseRecording() {
  try {
    if (isPaused) {
      await invoke("resume_recording");
      console.log("Resumed recording");
      statusIndicator.textContent = "Recording";
      statusIndicator.className = "status-recording";
      pauseBtn.innerHTML = "⏸";
      pauseBtn.title = "Pause";
      isPaused = false;
      startTime = Date.now() - elapsedPausedTime;
    } else {
      await invoke("pause_recording");
      console.log("Paused recording");
      statusIndicator.textContent = "Paused";
      statusIndicator.className = "status-paused";
      pauseBtn.innerHTML = "▶";
      pauseBtn.title = "Resume";
      isPaused = true;
      elapsedPausedTime = Date.now() - startTime;
    }
  } catch (error) {
    console.error("Failed to pause/resume:", error);
  }
}

async function stopRecording() {
  try {
    await invoke("stop_recording");
    console.log("Stopped recording");

    stopTimer();
    statusIndicator.textContent = "Idle";
    statusIndicator.className = "status-idle";

    recordBtn.disabled = false;
    pauseBtn.disabled = true;
    stopBtn.disabled = true;

    // Selector removed
    pauseBtn.innerHTML = "⏸";
    pauseBtn.title = "Pause";

    await loadRecordings();
  } catch (error) {
    console.error("Failed to stop:", error);
  }
}

// ===== FORMATTING HELPERS =====
function formatDuration(seconds) {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function formatDateTime(isoString) {
  const date = new Date(isoString);
  const dateStr = date.toISOString().split('T')[0];
  const timeStr = date.toTimeString().slice(0, 5);
  return `${dateStr} · ${timeStr}`;
}

// ===== TAG FILTERING =====
function extractUniqueTags(recordings) {
  const tagSet = new Set();
  recordings.forEach(rec => rec.tags.forEach(tag => tagSet.add(tag)));
  return Array.from(tagSet).sort();
}

function renderTagFilters(tags) {
  const allItem = tagFiltersEl.querySelector('[data-tag=""]');

  // Remove old tags, keep "All"
  tagFiltersEl.querySelectorAll('.filter-item:not([data-tag=""])').forEach(el => el.remove());

  // Add new tags
  tags.forEach(tag => {
    const item = document.createElement('div');
    item.className = 'filter-item';
    item.dataset.tag = tag;
    item.textContent = `#${tag}`;
    item.addEventListener('click', () => toggleTagFilter(tag));
    tagFiltersEl.appendChild(item);
  });
}

function toggleTagFilter(tag) {
  if (tag === "") {
    // "All" clicked
    selectedTags = [];
  } else {
    // Toggle tag
    const index = selectedTags.indexOf(tag);
    if (index > -1) {
      selectedTags.splice(index, 1);
    } else {
      selectedTags.push(tag);
    }
  }

  updateTagFilterUI();
  renderRecordingsList();
}

function updateTagFilterUI() {
  tagFiltersEl.querySelectorAll('.filter-item').forEach(item => {
    const tag = item.dataset.tag;
    if (tag === "" && selectedTags.length === 0) {
      item.classList.add('active');
    } else if (tag !== "" && selectedTags.includes(tag)) {
      item.classList.add('active');
    } else {
      item.classList.remove('active');
    }
  });
}

function filterRecordings(recordings, tags) {
  if (tags.length === 0) return recordings;
  // AND logic: recording must have ALL selected tags
  return recordings.filter(rec =>
    tags.every(tag => rec.tags.includes(tag))
  );
}

// ===== RECORDINGS LIST =====
async function loadRecordings() {
  try {
    const recordings = await invoke("list_recordings");
    allRecordings = recordings || [];

    // Update tag filters
    const tags = extractUniqueTags(allRecordings);
    renderTagFilters(tags);

    // Render list
    renderRecordingsList();
  } catch (error) {
    console.error("Failed to load recordings:", error);
  }
}

function renderRecordingsList() {
  const filtered = filterRecordings(allRecordings, selectedTags);

  if (filtered.length === 0) {
    recordingsListEl.innerHTML = "";
    emptyStateEl.style.display = "block";
    return;
  }

  emptyStateEl.style.display = "none";

  const html = filtered.map(rec => `
    <div class="recording-item" data-id="${rec.id}">
      <div class="recording-title">${rec.title || "Untitled"}</div>
      <div class="recording-meta">${formatDateTime(rec.created_at)} · ${formatDuration(rec.audio.duration_sec)}</div>
      <div class="recording-tags">
        ${rec.tags.map(tag => `<span class="recording-tag">#${tag}</span>`).join("")}
      </div>
    </div>
  `).join("");

  recordingsListEl.innerHTML = html;

  // Add click handlers
  recordingsListEl.querySelectorAll('.recording-item').forEach(item => {
    item.addEventListener('click', () => showDetailView(item.dataset.id));
  });
}

// ===== TAG CHIP MANAGEMENT =====
function renderTagChips() {
  const listEl = document.getElementById('detail-tags-list');
  listEl.innerHTML = currentRecordingTags.map(tag => `
    <div class="detail-tag-chip">
      #${tag}
      <span class="tag-remove" data-tag="${tag}">×</span>
    </div>
  `).join('');

  // Add remove handlers
  listEl.querySelectorAll('.tag-remove').forEach(btn => {
    btn.addEventListener('click', () => removeTag(btn.dataset.tag));
  });
}

async function addTag(tag) {
  tag = tag.trim().toLowerCase().replace(/^#/, '');
  if (!tag || currentRecordingTags.includes(tag)) return;

  currentRecordingTags.push(tag);
  renderTagChips();

  if (selectedRecordingId) {
    try {
      await invoke('update_tags', {
        recordingId: selectedRecordingId,
        tags: currentRecordingTags
      });
      await loadRecordings();
    } catch (err) {
      console.error('Failed to save tags:', err);
    }
  }
}

async function removeTag(tag) {
  currentRecordingTags = currentRecordingTags.filter(t => t !== tag);
  renderTagChips();

  if (selectedRecordingId) {
    try {
      await invoke('update_tags', {
        recordingId: selectedRecordingId,
        tags: currentRecordingTags
      });
      await loadRecordings();
    } catch (err) {
      console.error('Failed to save tags:', err);
    }
  }
}

// ===== DETAIL VIEW =====
function showDetailView(recordingId) {
  const recording = allRecordings.find(r => r.id === recordingId);
  if (!recording) return;

  selectedRecordingId = recordingId;
  currentRecordingTags = [...recording.tags];

  document.getElementById('detail-title').value = recording.title || "";
  document.getElementById('detail-meta').textContent =
    `${formatDateTime(recording.created_at)} · ${formatDuration(recording.audio.duration_sec)}`;

  renderTagChips();
  document.getElementById('detail-tags-input').value = "";

  detailViewEl.style.display = "block";
  appLayoutEl.classList.add('detail-open');
}

function hideDetailView() {
  selectedRecordingId = null;
  currentRecordingTags = [];
  detailViewEl.style.display = "none";
  appLayoutEl.classList.remove('detail-open');
}

// ===== DETAIL VIEW ACTIONS =====
async function playRecording() {
  if (!selectedRecordingId) return;
  console.log("Play:", selectedRecordingId);
  alert("Audio playback not implemented yet");
}

async function processRecording() {
  if (!selectedRecordingId) return;
  console.log("Process:", selectedRecordingId);
  alert("Processing not implemented yet");
}

async function openFolder() {
  if (!selectedRecordingId) return;
  try {
    const folderPath = `/Users/skopanev/nbp-data/${selectedRecordingId}`;

    console.log('Opening folder:', folderPath);
    await window.__TAURI_PLUGIN_OPENER__.openPath(folderPath);
  } catch (error) {
    console.error("Failed to open folder:", error);
    alert("Failed to open folder: " + error);
  }
}

async function deleteRecording() {
  console.log('DELETE BUTTON CLICKED!', selectedRecordingId);
  if (!selectedRecordingId) return;

  try {
    // Use native macOS dialog
    const confirmed = await window.__TAURI_PLUGIN_DIALOG__.ask(
      'This action cannot be undone.',
      {
        title: 'Delete Recording?',
        kind: 'warning'
      }
    );

    console.log('User confirmed:', confirmed);
    if (!confirmed) return;

    console.log('Calling delete_recording...');
    await invoke('delete_recording', { recordingId: selectedRecordingId });
    console.log('Delete successful');
    hideDetailView();
    await loadRecordings();
  } catch (error) {
    console.error("Failed to delete:", error);
    alert("Failed to delete: " + error);
  }
}

// ===== EVENT LISTENERS =====
recordBtn.addEventListener("click", startRecording);
pauseBtn.addEventListener("click", pauseRecording);
stopBtn.addEventListener("click", stopRecording);
backBtn.addEventListener("click", hideDetailView);

document.getElementById('play-btn').addEventListener('click', playRecording);
document.getElementById('process-btn').addEventListener('click', processRecording);
document.getElementById('open-folder-btn').addEventListener('click', openFolder);
document.getElementById('delete-btn').addEventListener('click', deleteRecording);

tagFiltersEl.querySelector('[data-tag=""]').addEventListener('click', () => toggleTagFilter(""));

document.getElementById('detail-tags-input').addEventListener('keypress', (e) => {
  if (e.key === 'Enter') {
    const input = e.target;
    addTag(input.value);
    input.value = '';
  }
});

// ===== INIT =====
loadRecordings();
