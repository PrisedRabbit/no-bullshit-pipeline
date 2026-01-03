const { invoke } = window.__TAURI__.core;

let timerInterval;
let startTime;
let elapsedPausedTime = 0;
let isPaused = false;

const statusIndicator = document.getElementById("status-indicator");
const timerDisplay = document.getElementById("timer");
const projectInput = document.getElementById("project-tags");
const recordBtn = document.getElementById("record-btn");
const pauseBtn = document.getElementById("pause-btn");
const stopBtn = document.getElementById("stop-btn");

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

async function startRecording() {
  const tags = projectInput.value.split(" ").filter(t => t.length > 0);
  try {
    // Invoke Rust command
    await invoke("start_recording", { tags });
    console.log("Started recording", tags);

    statusIndicator.textContent = "Recording";
    statusIndicator.className = "status-recording";
    recordBtn.disabled = true;
    pauseBtn.disabled = false;
    stopBtn.disabled = false;
    pauseBtn.textContent = "Pause";

    startTimer();
  } catch (error) {
    console.error("Failed to start recording:", error);
    alert("Failed to start: " + error);
  }
}

async function pauseRecording() {
  try {
    if (isPaused) {
      // Resume
      await invoke("resume_recording");
      console.log("Resumed recording");
      statusIndicator.textContent = "Recording";
      statusIndicator.className = "status-recording";
      pauseBtn.textContent = "Pause";
      isPaused = false;
      startTime = Date.now() - elapsedPausedTime;
    } else {
      // Pause
      await invoke("pause_recording");
      console.log("Paused recording");
      statusIndicator.textContent = "Paused";
      statusIndicator.className = "status-paused";
      pauseBtn.textContent = "Resume";
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
    pauseBtn.textContent = "Pause";
  } catch (error) {
    console.error("Failed to stop:", error);
  }
}

recordBtn.addEventListener("click", startRecording);
pauseBtn.addEventListener("click", pauseRecording);
stopBtn.addEventListener("click", stopRecording);
