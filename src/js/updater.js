// Check GitHub Releases for a newer version on startup. If one exists,
// reveal the #update-banner with a Download link to the release page.
// Network failures / no-newer-version → silent no-op (banner stays hidden).
//
// Banner uses the unified .app-banner component — see styles.css for the
// component contract; do not introduce a bespoke banner class for this.

import { invoke } from './core/tauri.js';

export async function checkForUpdates() {
  let info = null;
  try {
    info = await invoke('check_for_updates');
  } catch (_e) {
    // Offline / API error — keep quiet, banner stays hidden.
    return;
  }
  if (!info) return;

  const banner = document.getElementById('update-banner');
  const text = document.getElementById('update-banner-text');
  const dl = document.getElementById('update-download-btn');
  const dismiss = document.getElementById('update-dismiss-btn');
  if (!banner || !text || !dl || !dismiss) return;

  text.textContent = `${info.version} available — you have v${info.current}`;
  banner.style.display = '';

  dl.onclick = async () => {
    try {
      const opener = window.__TAURI__?.opener;
      if (opener?.openUrl) await opener.openUrl(info.url);
    } catch (err) {
      console.error('Failed to open release URL:', err);
    }
  };

  dismiss.onclick = () => {
    banner.style.display = 'none';
  };
}
