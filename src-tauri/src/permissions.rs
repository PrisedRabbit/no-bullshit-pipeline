use serde::{Deserialize, Serialize};
use cidre::av;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PermissionsState {
    pub mic: bool,
    pub system_audio: bool,
    /// macOS Calendar (EventKit) full access. Optional feature — gates
    /// recording↔calendar-event matching. Checked non-prompting on every
    /// `check_permissions`; the prompt itself is user-initiated.
    #[serde(default)]
    pub calendar: bool,
}

pub struct PermissionsStateCache(pub Arc<Mutex<PermissionsState>>);

#[tauri::command]
pub async fn check_permissions(
    state: tauri::State<'_, PermissionsStateCache>,
    _onboarding_completed: bool
) -> Result<PermissionsState, String> {
    // Check cache first - if already verified this session, return cached result.
    // Calendar status is cheap + non-prompting, so refresh it every call (it can
    // flip in System Settings without going through our request command).
    {
        let cache = state.0.lock().map_err(|e| e.to_string())?;
        if cache.mic && cache.system_audio {
            let mut out = cache.clone();
            out.calendar = crate::calendar::has_full_access();
            return Ok(out);
        }
    }

    // MICROPHONE: Use system API (reliable)
    let mic_status = av::CaptureDevice::authorization_status_for_media_type(av::MediaType::audio());
    let mic_authorized = matches!(mic_status, Ok(av::AuthorizationStatus::Authorized));

    // SYSTEM AUDIO: Check if Process Tap can be created (lightweight)
    // This verifies the permission without starting a full recording
    let system_audio_authorized = {
        let cache = state.0.lock().map_err(|e| e.to_string())?;
        if cache.system_audio {
            true
        } else {
            drop(cache);

            // Try to create a Process Tap - this requires the permission
            let result = crate::system_audio::check_system_audio_permission();

            if result {
                let mut cache = state.0.lock().map_err(|e| e.to_string())?;
                cache.system_audio = true;
            }
            result
        }
    };

    let calendar_authorized = crate::calendar::has_full_access();

    // Update mic + calendar cache
    {
        let mut cache = state.0.lock().map_err(|e| e.to_string())?;
        cache.mic = mic_authorized;
        cache.calendar = calendar_authorized;
    }

    Ok(PermissionsState {
        mic: mic_authorized,
        system_audio: system_audio_authorized,
        calendar: calendar_authorized,
    })
}

#[tauri::command]
pub async fn request_mic_permission(state: tauri::State<'_, PermissionsStateCache>) -> Result<bool, String> {
    // Proper TCC request: shows the system mic prompt and resolves exactly
    // when the user answers (or immediately, with the cached value, if the
    // status is already determined). The old approach started a real cpal
    // capture and slept 2s on the async runtime — on a first launch that
    // could deadlock on CoreAudio's TCC gate and leave the onboarding UI
    // stuck forever on "Requesting…" even though the user had already
    // granted access.
    // cidre's completion-block future is !Send, so it can't be awaited
    // directly inside a Tauri async command (which requires Send). Drive it
    // on a dedicated thread with a local current-thread runtime and ship
    // back only the bool (Send) over a oneshot.
    let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(_) => {
                let _ = tx.send(false);
                return;
            }
        };
        let granted = rt.block_on(async {
            av::CaptureDevice::request_access_for_media_type(av::MediaType::audio())
                .await
                .unwrap_or(false)
        });
        let _ = tx.send(granted);
    });

    let granted = rx
        .await
        .map_err(|_| "Microphone access request failed".to_string())?;

    if let Ok(mut cache) = state.0.lock() {
        cache.mic = granted;
    }

    #[cfg(debug_assertions)]
    eprintln!("DEBUG: Mic permission granted: {}", granted);
    Ok(granted)
}

#[tauri::command]
pub fn request_system_audio_permission(state: tauri::State<'_, PermissionsStateCache>) -> Result<bool, String> {
    #[cfg(debug_assertions)]
    eprintln!("DEBUG: Triggering System Audio permission via test recording...");

    // Create a temporary path for the test recording
    let temp_path = std::path::PathBuf::from(format!("/tmp/nbp-permission-test-{}.ogg", std::process::id()));

    // Start system audio capture (this triggers the permission dialog)
    let mut recorder = match crate::system_audio::start_system_capture(temp_path.clone(), false) {
        Ok(r) => {
            #[cfg(debug_assertions)]
            eprintln!("System audio recorder created for permission check");
            r
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("System audio recorder failed: {:?}", _e);
            let mut cache = state.0.lock().map_err(|e| e.to_string())?;
            cache.system_audio = false;
            return Ok(false);
        }
    };

    // Wait a moment to let the actual recording start and verify permission was granted
    // If the dialog is shown and denied, the file won't be created
    std::thread::sleep(std::time::Duration::from_millis(2000));

    // Immediately stop it
    recorder.stop();

    // Check if the file was actually created (indicates permission was granted)
    let permission_granted = temp_path.exists();

    // Clean up the test file
    let _ = std::fs::remove_file(&temp_path);

    // Update cache only if we confirmed success
    let mut cache = state.0.lock().map_err(|e| e.to_string())?;
    cache.system_audio = permission_granted;

    #[cfg(debug_assertions)]
    eprintln!("DEBUG: System Audio permission result: {}", permission_granted);
    Ok(permission_granted)
}

#[tauri::command]
pub async fn open_privacy_settings(pane: String) {
    let url = match pane.as_str() {
        "mic" => "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
        "system_audio" => "x-apple.systempreferences:com.apple.preference.security",
        _ => "x-apple.systempreferences:com.apple.preference.security",
    };
    
    #[cfg(debug_assertions)]
    eprintln!("DEBUG: Opening Privacy Settings: {}", url);
    let _ = std::process::Command::new("open").arg(url).spawn();
}
