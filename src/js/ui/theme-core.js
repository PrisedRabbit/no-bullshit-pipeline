// Pure, dependency-free theme helpers shared by the main window and the
// dictation HUD (separate esbuild bundles). No DOM-structure or app-state deps,
// so either window can import this without dragging in the other's code.

/** Collapse legacy theme names to the current trinary set (auto/light/dark). */
export function normalizeTheme(theme) {
  if (theme === 'neon-purple' || theme === 'deep-obsidian' || theme === 'deep-blue') return 'dark';
  if (theme === 'light-pastel') return 'light';
  if (theme === 'auto' || theme === 'light' || theme === 'dark') return theme;
  return 'auto';
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
