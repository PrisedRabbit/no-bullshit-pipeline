// Pure, dependency-free theme helpers shared by the main window and the
// dictation HUD (separate esbuild bundles). No DOM-structure or app-state deps,
// so either window can import this without dragging in the other's code.

/** Validate a theme value. Legacy names are migrated to auto/light/dark on disk
 *  by the Rust `load_settings`, so this only guards against an unexpected value. */
export function normalizeTheme(theme) {
  return theme === 'light' || theme === 'dark' || theme === 'auto' ? theme : 'auto';
}

/**
 * Resolve a theme setting to an effective 'light'/'dark' and keep it in sync.
 * Calls `onEffective(effective)` immediately, and again on every system
 * appearance change while the setting is 'auto'. Returns a cleanup function
 * that detaches the listener (no-op for fixed light/dark).
 */
export function watchEffectiveTheme(theme, onEffective) {
  const normalized = normalizeTheme(theme);
  if (normalized !== 'auto') {
    onEffective(normalized);
    return () => {};
  }
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  const apply = (matches) => onEffective(matches ? 'dark' : 'light');
  apply(mq.matches);
  const listener = (e) => apply(e.matches);
  mq.addEventListener('change', listener);
  return () => mq.removeEventListener('change', listener);
}
