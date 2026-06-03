import { invoke } from '../core/tauri.js';
import * as state from '../core/state.js';
import { saveSettings } from './settings.js';

const onboardingOverlay = document.getElementById('onboarding-overlay');
const requestMicBtn = document.getElementById('request-mic-btn');
const requestSysBtn = document.getElementById('request-sys-btn');
const requestCalBtn = document.getElementById('request-cal-btn');
const onboardingContinueBtn = document.getElementById('onboarding-continue-btn');
const permissionWarning = document.getElementById('permission-warning');
const fixPermissionsBtn = document.getElementById('fix-permissions-btn');

export async function updatePermissionStatus() {
  try {
    state.setPermissions(await invoke('check_permissions'));

    const micItem = document.getElementById('perm-mic-item');
    const sysItem = document.getElementById('perm-sys-item');

    if (micItem) {
      micItem.querySelector('.perm-status').textContent = state.permissions.mic ? 'Granted' : 'Not Granted';
      micItem.querySelector('.perm-status').className = `perm-status ${state.permissions.mic ? 'granted' : 'denied'}`;
    }
    if (sysItem) {
      sysItem.querySelector('.perm-status').textContent = state.permissions.system_audio ? 'Granted' : 'Not Granted';
      sysItem.querySelector('.perm-status').className = `perm-status ${state.permissions.system_audio ? 'granted' : 'denied'}`;
    }

    // Calendar (Settings → Recording → Calendar). Optional feature; granting
    // the permission is the opt-in — matching runs whenever it's granted.
    const calStatus = document.getElementById('perm-cal-status');
    if (calStatus) {
      calStatus.textContent = state.permissions.calendar ? 'Granted' : 'Not Granted';
      calStatus.className = `perm-status ${state.permissions.calendar ? 'granted' : 'denied'}`;
    }
    if (requestCalBtn) {
      requestCalBtn.textContent = state.permissions.calendar ? 'Granted ✔' : 'Grant';
      requestCalBtn.disabled = state.permissions.calendar;
    }

    // Show warning if mic is not granted (outside onboarding)
    if (permissionWarning) {
      permissionWarning.style.display = state.permissions.mic ? 'none' : 'flex';
    }

    // Onboarding button states
    if (requestMicBtn) {
      requestMicBtn.textContent = state.permissions.mic ? 'Microphone \u2714' : 'Grant Microphone';
      requestMicBtn.disabled = state.permissions.mic;
    }
    if (requestSysBtn) {
      requestSysBtn.textContent = state.permissions.system_audio ? 'System Audio \u2714' : 'Grant System Audio';
      requestSysBtn.disabled = state.permissions.system_audio;
    }
    if (onboardingContinueBtn) {
      onboardingContinueBtn.disabled = !state.permissions.mic;
    }
  } catch (err) {
    console.error('Permission check failed:', err);
  }
}

export function initPermissions() {
  if (requestMicBtn) {
    requestMicBtn.addEventListener('click', async () => {
      requestMicBtn.disabled = true;
      requestMicBtn.textContent = 'Requesting...';
      try {
        await invoke('request_mic_permission');
        await updatePermissionStatus();
      } catch (err) {
        console.error('Mic permission error:', err);
        requestMicBtn.textContent = 'Grant Microphone';
        requestMicBtn.disabled = false;
      }
    });
  }

  if (requestSysBtn) {
    requestSysBtn.addEventListener('click', async () => {
      requestSysBtn.disabled = true;
      requestSysBtn.textContent = 'Requesting...';
      try {
        await invoke('request_system_audio_permission');
        await updatePermissionStatus();
      } catch (err) {
        console.error('System audio permission error:', err);
        requestSysBtn.textContent = 'Grant System Audio';
        requestSysBtn.disabled = false;
      }
    });
  }

  if (requestCalBtn) {
    requestCalBtn.addEventListener('click', async () => {
      requestCalBtn.disabled = true;
      requestCalBtn.textContent = 'Requesting...';
      try {
        // macOS shows the TCC prompt only the first time; afterwards this
        // resolves immediately with the current status. If already determined
        // (and not granted), send the user to System Settings to flip it.
        const granted = await invoke('request_calendar_permission');
        if (!granted) await invoke('open_calendar_settings');
      } catch (err) {
        console.error('Calendar permission error:', err);
        await invoke('open_calendar_settings').catch(() => {});
      }
      await updatePermissionStatus();
    });
  }

  if (onboardingContinueBtn) {
    onboardingContinueBtn.addEventListener('click', async () => {
      if (onboardingOverlay) onboardingOverlay.style.display = 'none';
      state.appSettings.onboarding_completed = true;
      await saveSettings();
    });
  }

  if (fixPermissionsBtn) {
    fixPermissionsBtn.addEventListener('click', async () => {
      try { await invoke('open_system_preferences'); } catch (err) { console.error('Failed to open prefs:', err); }
    });
  }
}
