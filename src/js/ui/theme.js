// Theme system: trinary auto / light / dark. The previous Neon Purple +
// Deep Blue + Light split is collapsed to a single Dark (= former Neon
// Purple) and Light, plus Auto which follows the macOS appearance via
// `prefers-color-scheme`. Legacy values are normalized on apply so old
// stored settings ("neon-purple", "deep-blue", "deep-obsidian",
// "light-pastel") just work without a one-shot migration.
//
// The normalize + auto-follow logic lives in `theme-core.js` so the dictation
// HUD (a separate window/bundle) can share it instead of duplicating it.

import * as state from '../core/state.js';
import { normalizeTheme, watchEffectiveTheme } from './theme-core.js';

let detachTheme = null;

function setBodyClass(effective) {
  document.body.classList.remove('dark', 'light');
  document.body.classList.add(effective);
}

export function applyTheme(theme) {
  const normalized = normalizeTheme(theme);

  // (Re)bind the effective-theme watcher — applies now and, for 'auto', tracks
  // the system appearance until the next applyTheme call.
  if (detachTheme) detachTheme();
  detachTheme = watchEffectiveTheme(theme, setBodyClass);

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
