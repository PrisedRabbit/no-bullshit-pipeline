// Streams JS-side ignore-trace logs into the Rust log facade so they end
// up in the same terminal stream as Rust `log::info!` output. Avoids
// having to babysit two log surfaces (terminal + DevTools console).
//
// Drop this once the Ignore-on-FaceTime debugging is over.

const { invoke } = window.__TAURI__.core;

export function trace(...args) {
  const msg = args
    .map((a) => {
      if (typeof a === 'string') return a;
      try {
        return JSON.stringify(a);
      } catch {
        return String(a);
      }
    })
    .join(' ');
  console.log('[ignore-trace]', msg);
  invoke('log_from_js', { msg }).catch(() => {});
}
