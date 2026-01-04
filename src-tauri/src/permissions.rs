use serde::{Deserialize, Serialize};
use cidre::av;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PermissionsState {
    pub mic: bool,
    pub system_audio: bool,
}

pub struct PermissionsStateCache(pub Arc<Mutex<PermissionsState>>);

#[tauri::command]
pub async fn check_permissions(
    state: tauri::State<'_, PermissionsStateCache>,
    onboarding_completed: bool
) -> Result<PermissionsState, String> {
    // For MICROPHONE: ALWAYS check the system API directly
    // This API is safe and does NOT trigger a permission dialog
    let mic_status = av::CaptureDevice::authorization_status_for_media_type(av::MediaType::audio());
    let mic_authorized = matches!(mic_status, Ok(av::AuthorizationStatus::Authorized));
    
    // For SYSTEM AUDIO: Try a quick silent check ONLY if NOT first run
    let system_audio_authorized = {
        let cache = state.0.lock().map_err(|e| e.to_string())?;
        
        // If already verified this session, use cache
        if cache.system_audio {
            true
        } else if !onboarding_completed {
            // First run - skip the test, just return false
            false
        } else {
            // Not first run - try ONE quick test to verify
            drop(cache); // Release lock before test
            
            let temp_path = PathBuf::from("/tmp/nbp-startup-permission-check.ogg");
            let verified = if let Ok(mut recorder) = crate::system_audio::start_system_capture(temp_path.clone()) {
                std::thread::sleep(std::time::Duration::from_millis(500));
                recorder.stop();
                let exists = temp_path.exists();
                let _ = std::fs::remove_file(&temp_path);
                exists
            } else {
                false
            };
            
            // Update cache with result
            let mut cache = state.0.lock().map_err(|e| e.to_string())?;
            cache.system_audio = verified;
            verified
        }
    };
    
    // Update mic cache too
    // Update mic cache too
    let final_mic_authorized = {
        let mut cache = state.0.lock().map_err(|e| e.to_string())?;
        if cache.mic {
            // Already verified (e.g. via request_mic_permission success)
            // Trust the cache, ignore system API latency
            true
        } else {
            // Not verified yet, trust system API
            cache.mic = mic_authorized;
            mic_authorized
        }
    };
    
    Ok(PermissionsState {
        mic: final_mic_authorized,
        system_audio: system_audio_authorized,
    })
}

#[tauri::command]
pub async fn request_mic_permission(state: tauri::State<'_, PermissionsStateCache>) -> Result<bool, String> {
    println!("DEBUG: Requesting Microphone Access via cpal...");
    
    // We try to start a capture to a temporary file. 
    // This will trigger the macOS microphone permission dialog.
    let temp_path = std::path::PathBuf::from("/tmp/nbp-mic-permission-check.ogg");
    
    match crate::mic_audio::start_mic_capture(temp_path.clone()) {
        Ok(mut recorder) => {
            println!("✅ Mic capture started");
            
            // Wait to ensure audio is actually being captured (permission granted)
            std::thread::sleep(std::time::Duration::from_millis(2000));
            
            recorder.stop();
            
            // Verify file was created (indicates permission was granted)
            let permission_granted = temp_path.exists();
            let _ = std::fs::remove_file(&temp_path);
            
            // Update cache
            let mut cache = state.0.lock().map_err(|e| e.to_string())?;
            cache.mic = permission_granted;
            
            println!("DEBUG: Mic permission verified: {}", permission_granted);
            Ok(permission_granted)
        }
        Err(e) => {
            println!("❌ Mic capture failed: {:?}", e);
            
            // Update cache to false
            let mut cache = state.0.lock().map_err(|e| e.to_string())?;
            cache.mic = false;
            
            Ok(false)
        }
    }
}

#[tauri::command]
pub fn request_system_audio_permission(state: tauri::State<'_, PermissionsStateCache>) -> Result<bool, String> {
    println!("DEBUG: Triggering System Audio permission via test recording...");
    
    // Create a temporary path for the test recording
    let temp_path = PathBuf::from("/tmp/nbp-permission-test.ogg");
    
    // Start system audio capture (this triggers the permission dialog)
    let mut recorder = match crate::system_audio::start_system_capture(temp_path.clone()) {
        Ok(r) => {
            println!("✅ System audio recorder created");
            r
        }
        Err(e) => {
            println!("❌ System audio recorder failed: {:?}", e);
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
    
    println!("DEBUG: System Audio permission result: {}", permission_granted);
    Ok(permission_granted)
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
