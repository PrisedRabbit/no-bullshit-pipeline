use serde::{Deserialize, Serialize};
use cidre::av;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PermissionsState {
    pub mic: bool,
    pub system_audio: bool,
}

pub struct PermissionsStateCache(pub Arc<Mutex<PermissionsState>>);

#[tauri::command]
pub async fn check_permissions(
    state: tauri::State<'_, PermissionsStateCache>,
    _onboarding_completed: bool
) -> Result<PermissionsState, String> {
    // Check cache first - if already verified this session, return cached result
    {
        let cache = state.0.lock().map_err(|e| e.to_string())?;
        if cache.mic && cache.system_audio {
            return Ok(cache.clone());
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

    // Update mic cache
    {
        let mut cache = state.0.lock().map_err(|e| e.to_string())?;
        cache.mic = mic_authorized;
    }

    Ok(PermissionsState {
        mic: mic_authorized,
        system_audio: system_audio_authorized,
    })
}

#[tauri::command]
pub async fn request_mic_permission(state: tauri::State<'_, PermissionsStateCache>) -> Result<bool, String> {
    #[cfg(debug_assertions)]
    eprintln!("DEBUG: Requesting Microphone Access via cpal...");

    // We try to start a capture to a temporary file.
    // This will trigger the macOS microphone permission dialog.
    let temp_path = std::path::PathBuf::from(format!("/tmp/nbp-mic-permission-check-{}.ogg", std::process::id()));

    match crate::mic_audio::start_mic_capture(temp_path.clone(), None, false) {
        Ok(mut recorder) => {
            #[cfg(debug_assertions)]
            eprintln!("Mic capture started for permission check");

            // Wait to ensure audio is actually being captured (permission granted)
            std::thread::sleep(std::time::Duration::from_millis(2000));

            recorder.stop();

            // Verify file was created (indicates permission was granted)
            let permission_granted = temp_path.exists();
            let _ = std::fs::remove_file(&temp_path);

            // Update cache
            let mut cache = state.0.lock().map_err(|e| e.to_string())?;
            cache.mic = permission_granted;

            #[cfg(debug_assertions)]
            eprintln!("DEBUG: Mic permission verified: {}", permission_granted);
            Ok(permission_granted)
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("Mic capture failed: {:?}", _e);

            // Update cache to false
            let mut cache = state.0.lock().map_err(|e| e.to_string())?;
            cache.mic = false;

            Ok(false)
        }
    }
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
