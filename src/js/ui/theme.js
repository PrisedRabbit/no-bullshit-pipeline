// Theme system: trinary auto / light / dark. The previous Neon Purple +
// Deep Blue + Light split is collapsed to a single Dark (= former Neon
// Purple) and Light, plus Auto which follows the macOS appearance via
// `prefers-color-scheme`. Legacy values are normalized on apply so old
// stored settings ("neon-purple", "deep-blue", "deep-obsidian",
// "light-pastel") just work without a one-shot migration.

import * as state from '../core/state.js';

let mediaQuery = null;
let mediaListener = null;

function normalize(theme) {
  if (theme === 'neon-purple' || theme === 'deep-obsidian' || theme === 'deep-blue') return 'dark';
  if (theme === 'light-pastel') return 'light';
  if (theme === 'auto' || theme === 'light' || theme === 'dark') return theme;
  return 'auto';
}

function setBodyClass(effective) {
  document.body.classList.remove('dark', 'light');
  document.body.classList.add(effective);
}

function detachAutoListener() {
  if (mediaQuery && mediaListener) {
    mediaQuery.removeEventListener('change', mediaListener);
  }
  mediaQuery = null;
  mediaListener = null;
}

export function applyTheme(theme) {
  const normalized = normalize(theme);
  detachAutoListener();

  if (normalized === 'auto') {
    mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = (matches) => setBodyClass(matches ? 'dark' : 'light');
    apply(mediaQuery.matches);
    mediaListener = (e) => apply(e.matches);
    mediaQuery.addEventListener('change', mediaListener);
  } else {
    setBodyClass(normalized);
  }

  // Mirror the chosen value back into shared state so saveSettings picks up
  // "auto"/"light"/"dark" instead of an outdated stored legacy name.
  if (state.appSettings) {
    state.appSettings.theme = normalized;
  }

  // Update toggle button active state. We compare against the normalized
  // value so a legacy stored "neon-purple" lights up the "Auto"/"Dark"
  // button consistently with what was actually applied.
  document.querySelectorAll('.theme-btn').forEach((b) => {
    b.classList.toggle('active', b.dataset.theme === normalized);
  });
}
