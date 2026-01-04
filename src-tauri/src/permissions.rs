use serde::{Deserialize, Serialize};
use cidre::av;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PermissionsState {
    pub mic: bool,
    pub system_audio: bool,
}

pub struct PermissionsStateCache(pub Arc<Mutex<PermissionsState>>);

#[tauri::command]
pub async fn check_permissions(state: tauri::State<'_, PermissionsStateCache>) -> Result<PermissionsState, String> {
    let mic_status = av::CaptureDevice::authorization_status_for_media_type(av::MediaType::audio());
    let mic_authorized = matches!(mic_status, Ok(av::AuthorizationStatus::Authorized));

    // For system audio: Try to create a tap silently to check permission
    // This won't show a dialog if permission was already granted
    let system_audio_authorized = {
        let mut cache = state.0.lock().map_err(|e| e.to_string())?;
        
        // Only check if not already verified
        if !cache.system_audio {
            // Try creating a temp tap to verify
            let temp_path = PathBuf::from("/tmp/nbp-permission-check.ogg");
            if let Ok(recorder) = crate::system_audio::start_system_capture(temp_path.clone()) {
                drop(recorder);
                let _ = std::fs::remove_file(&temp_path);
                cache.system_audio = true;
                true
            } else {
                false
            }
        } else {
            cache.system_audio
        }
    };
    
    Ok(PermissionsState {
        mic: mic_authorized,
        system_audio: system_audio_authorized,
    })
}

#[tauri::command]
pub async fn request_mic_permission() -> bool {
    println!("DEBUG: Requesting Microphone Access...");
    let _ = av::CaptureDevice::request_access_for_media_type(av::MediaType::audio());
    true
}

#[tauri::command]
pub fn request_system_audio_permission(state: tauri::State<'_, PermissionsStateCache>) -> Result<bool, String> {
    println!("DEBUG: Triggering System Audio permission via test recording...");
    
    // Create a temporary path for the test recording
    let temp_path = PathBuf::from("/tmp/nbp-permission-test.ogg");
    
    // Start system audio capture (this triggers the permission dialog)
    let recorder = match crate::system_audio::start_system_capture(temp_path.clone()) {
        Ok(r) => {
            println!("✅ System audio recorder created - permission granted");
            r
        }
        Err(e) => {
            println!("❌ System audio recorder failed: {:?}", e);
            let mut cache = state.0.lock().map_err(|e| e.to_string())?;
            cache.system_audio = false;
            return Ok(false);
        }
    };
    
    // Immediately stop it (this was just to trigger the permission)
    drop(recorder);
    
    // Clean up the test file
    let _ = std::fs::remove_file(&temp_path);
    
    // Update cache
    let mut cache = state.0.lock().map_err(|e| e.to_string())?;
    cache.system_audio = true;
    
    println!("DEBUG: System Audio permission result: true");
    Ok(true)
}

#[tauri::command]
pub async fn open_privacy_settings(pane: String) {
    let url = match pane.as_str() {
        "mic" => "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
        "system_audio" => "x-apple.systempreferences:com.apple.preference.security",
        _ => "x-apple.systempreferences:com.apple.preference.security",
    };
    
    println!("DEBUG: Opening Privacy Settings: {}", url);
    let _ = std::process::Command::new("open").arg(url).spawn();
}
