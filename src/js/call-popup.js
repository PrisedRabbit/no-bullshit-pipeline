// Popup for the call-session orchestrator. Recording starts automatically
// on call detection — this popup is the "we just started recording, click
// Ignore if you don't want it" affordance. After IGNORE_WINDOW_MS the popup
// fades away and recording continues silently until the call ends.
//
// Events from Rust (`call-popup`):
//   • kind: "recording", app: <name>  — recording started, Ignore button live
//   • kind: "error",     message      — start_recording failed
//   • kind: "hide"                     — recording stopped, dismiss popup now
//
// Click Ignore → invoke ignore_call_recording → backend stops + deletes.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const windowApi = window.__TAURI__.window || {};
const getPopup = windowApi.getCurrentWindow || windowApi.getCurrent;
const popup = typeof getPopup === 'function' ? getPopup() : null;

const body = document.body;
const card = document.getElementById('card');
const dot = document.getElementById('dot');
const msg = document.getElementById('msg');
const appEl = document.getElementById('app');
const ignoreBtn = document.getElementById('ignore-btn');
const progressFill = document.getElementById('progress-fill');

const IGNORE_WINDOW_MS = 7_000;
const ERROR_AUTO_HIDE_MS = 5_000;

let hideTimer = null;

// Mirror the main-window theme so the popup doesn't look out of place.
// Resolved on each `recording`-kind show (see `show()` below) so the popup
// reflects the user's current Settings + system appearance. We deliberately
// do NOT subscribe to live `prefers-color-scheme` updates — the popup lives
// for ≤10s per appearance, an appearance flip mid-display is irrelevant.

function normalizeTheme(theme) {
  if (theme === 'neon-purple' || theme === 'deep-obsidian' || theme === 'deep-blue') return 'dark';
  if (theme === 'light-pastel') return 'light';
  if (theme === 'auto' || theme === 'light' || theme === 'dark') return theme;
  return 'auto';
}

async function resolveAndApplyTheme() {
  let stored = 'auto';
  try {
    const settings = await invoke('load_settings');
    stored = settings && settings.theme;
  } catch (err) {
    console.warn('call-popup: settings load failed, defaulting to auto', err);
  }
  const normalized = normalizeTheme(stored);
  const effective = normalized === 'auto'
    ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
    : normalized;
  body.classList.remove('dark', 'light');
  body.classList.add(effective);
}

function setClickthrough(on) {
  invoke('set_call_popup_clickthrough', { clickthrough: on }).catch(() => {});
}

function restartProgressBar(durationMs) {
  if (!progressFill) return;
  // CSS-driven width animation. Force a reflow between the reset and the
  // start so the browser registers two distinct states and re-runs the
  // transition — without `offsetWidth` read the second assignment can be
  // collapsed and the bar appears frozen at the target width.
  progressFill.style.transition = 'none';
  progressFill.style.width = '100%';
  void progressFill.offsetWidth;
  progressFill.style.transition = `width ${durationMs}ms linear`;
  progressFill.style.width = '0%';
}

function show(kind, appName, message) {
  // Re-resolve theme each show — covers both initial load and any Settings
  // change that happened while the popup window was idle.
  resolveAndApplyTheme();

  dot.classList.remove('recording', 'error');
  body.classList.remove('show-actions');

  let ttl;
  if (kind === 'recording') {
    dot.classList.add('recording');
    msg.textContent = 'Recording meeting';
    body.classList.add('show-actions');
    ttl = IGNORE_WINDOW_MS;
    restartProgressBar(IGNORE_WINDOW_MS);
  } else if (kind === 'error') {
    dot.classList.add('error');
    msg.textContent = message || 'Recording failed';
    ttl = ERROR_AUTO_HIDE_MS;
  } else {
    return;
  }
  appEl.textContent = appName || '';

  body.classList.add('visible');
  setClickthrough(false);

  clearTimeout(hideTimer);
  hideTimer = setTimeout(hide, ttl);
}

function hide() {
  clearTimeout(hideTimer);
  hideTimer = null;
  body.classList.remove('visible', 'show-actions');
  setClickthrough(true);
}

// Drag via NSWindow.startDragging() — works on transparent/borderless wry
// windows where CSS -webkit-app-region: drag is unreliable. Buttons opt out
// via the e.target.closest('button') check.
if (card && popup && typeof popup.startDragging === 'function') {
  card.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    if (e.target.closest('button')) return;
    e.preventDefault();
    popup.startDragging().catch((err) => console.error('startDragging failed', err));
  });
}

ignoreBtn.addEventListener('click', async () => {
  const { trace } = await import('./ignore-trace.js');
  trace('popup Ignore clicked at', new Date().toISOString());
  invoke('ignore_call_recording')
    .then(() => trace('popup ignore_call_recording invoke resolved'))
    .catch((err) => trace('popup ignore_call_recording failed:', String(err)));
  hide();
});

listen('call-popup', (event) => {
  const payload = event.payload || {};
  const kind = payload.kind;
  const appName = payload.app || null;
  const message = payload.message || null;
  if (kind === 'recording' || kind === 'error') {
    show(kind, appName, message);
  } else if (kind === 'hide') {
    hide();
  } else {
    console.warn('call-popup: unknown kind', kind);
  }
}).catch((err) => console.error('call-popup: listen failed', err));
