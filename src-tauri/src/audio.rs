use std::sync::Mutex;
use std::time::SystemTime;
use std::thread::JoinHandle;
use tauri::{Emitter, State};
use crate::storage::{self, RecordingMetadata};

pub struct AudioState {
    pub is_recording: Mutex<bool>,
    pub mic_recorder: Mutex<Option<crate::mic_audio::MicAudioRecorder>>,
    pub system_recorder: Mutex<Option<crate::system_audio::SystemAudioRecorder>>,
    pub realtime_mixer: Mutex<Option<crate::audio_processing::RealtimeMixer>>,
    pub current_session: Mutex<Option<RecordingMetadata>>,
    pub start_timestamp: Mutex<Option<SystemTime>>,
    pub save_mix_only: Mutex<bool>,
    /// Handle to the background finalization thread — joined on next stop or app shutdown
    pub finalization_handle: Mutex<Option<JoinHandle<()>>>,
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
            mic_recorder: Mutex::new(None),
            system_recorder: Mutex::new(None),
            realtime_mixer: Mutex::new(None),
            current_session: Mutex::new(None),
            start_timestamp: Mutex::new(None),
            save_mix_only: Mutex::new(true),
            finalization_handle: Mutex::new(None),
        }
    }

    /// Wait for any in-progress finalization to complete (called on shutdown / before new recording)
    pub fn wait_for_finalization(&self) {
        if let Ok(mut handle_guard) = self.finalization_handle.lock() {
            if let Some(handle) = handle_guard.take() {
                let _ = handle.join();
            }
        }
    }
}

#[tauri::command]
pub fn start_recording(app_handle: tauri::AppHandle, state: State<'_, AudioState>, save_mix_only: bool) -> Result<storage::RecordingMetadata, String> {
    let mut is_recording = state.is_recording.lock().map_err(|e| e.to_string())?;
    if *is_recording {
        return Err("Already recording".to_string());
    }

    // Wait for any previous finalization to complete before starting new recording
    state.wait_for_finalization();

    // --- Disk space check ---
    let data_dir = storage::get_data_dir();
    check_disk_space(&data_dir)?;

    let metadata = storage::create_recording(String::new(), vec![])?;
    let mix_only = save_mix_only;
    eprintln!("DEBUG: start_recording received save_mix_only={}", mix_only);

    *state.save_mix_only.lock().map_err(|e| e.to_string())? = mix_only;

    // --- Real-time Mixer FIRST (so it's ready before capture threads push data) ---
    let mix_path = storage::get_recording_dir(&metadata.id).join("audio_mix.ogg");
    match crate::audio_processing::RealtimeMixer::new(mix_path) {
        Ok(mixer) => {
            *state.realtime_mixer.lock().map_err(|e| e.to_string())? = Some(mixer);
        },
        Err(e) => {
            eprintln!("WARNING: Real-time mixer failed: {}", e);
        }
    }

    // --- Microphone Capture ---
    let mic_path = storage::get_recording_dir(&metadata.id).join("raw_mic.ogg");
    match crate::mic_audio::start_mic_capture(mic_path, None, mix_only) {
        Ok(recorder) => {
            *state.mic_recorder.lock().map_err(|e| e.to_string())? = Some(recorder);
        },
        Err(e) => {
            return Err(format!("Microphone capture failed: {}", e));
        }
    }

    // --- System Audio ---
    let system_path = storage::get_recording_dir(&metadata.id).join("raw_system.ogg");
    eprintln!("DEBUG: Starting system audio capture to {:?}", system_path);
    match crate::system_audio::start_system_capture(system_path, mix_only) {
        Ok(recorder) => {
            eprintln!("DEBUG: System audio capture started successfully");
            *state.system_recorder.lock().map_err(|e| e.to_string())? = Some(recorder);
        },
        Err(e) => {
            eprintln!("ERROR: System audio capture failed: {:?}", e);
            let _ = app_handle.emit("recording_warning", format!("System audio unavailable: {}", e));
        }
    }

    *state.start_timestamp.lock().map_err(|e| e.to_string())? = Some(std::time::SystemTime::now());

    let metadata_clone = metadata.clone();
    *state.current_session.lock().map_err(|e| e.to_string())? = Some(metadata);
    *is_recording = true;

    Ok(metadata_clone)
}

/// Check that at least 100MB of disk space is available
fn check_disk_space(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        let c_path = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|e| format!("Invalid path: {}", e))?;
        let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::statfs(c_path.as_ptr(), &mut stat) };
        if ret == 0 {
            let available_bytes = stat.f_bavail as u64 * stat.f_bsize as u64;
            let min_bytes = 100 * 1024 * 1024; // 100 MB
            if available_bytes < min_bytes {
                return Err(format!(
                    "Insufficient disk space: {:.0} MB available, need at least 100 MB",
                    available_bytes as f64 / (1024.0 * 1024.0)
                ));
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn pause_recording(_state: State<'_, AudioState>) -> Result<(), String> {
    Err("Pause is not yet implemented".to_string())
}

#[tauri::command]
pub fn resume_recording(_state: State<'_, AudioState>) -> Result<(), String> {
    Err("Resume is not yet implemented".to_string())
}

#[tauri::command]
pub fn stop_recording(state: State<'_, AudioState>) -> Result<(), String> {
    let mut is_recording = state.is_recording.lock().map_err(|e| e.to_string())?;
    if !*is_recording {
        return Ok(());
    }

    // CORRECT ORDER: Stop capture sources FIRST, then let mixer drain

    // 1. Stop Microphone capture (stops cpal stream, joins encoder thread)
    {
        let mut mic_guard = state.mic_recorder.lock().map_err(|e| e.to_string())?;
        if let Some(mut recorder) = mic_guard.take() {
            recorder.stop();
        }
    }

    // 2. Stop System Audio capture (stops Core Audio device, joins encoder thread)
    {
        let mut system_guard = state.system_recorder.lock().map_err(|e| e.to_string())?;
        if let Some(mut recorder) = system_guard.take() {
            recorder.stop();
        }
    }

    // 3. Stop Real-time Mixer LAST — it drains remaining buffer samples before finishing
    {
        let mut mixer_guard = state.realtime_mixer.lock().map_err(|e| e.to_string())?;
        if let Some(mut mixer) = mixer_guard.take() {
            mixer.stop();
        }
    }

    // --- Duration & Metadata Update ---
    let start_time = *state.start_timestamp.lock().map_err(|e| e.to_string())?;
    let duration_sec = if let Some(start) = start_time {
        start.elapsed().unwrap_or_default().as_secs_f64()
    } else {
        0.0
    };

    let save_mix_only = *state.save_mix_only.lock().map_err(|e| e.to_string())?;

    let finalization_handle = {
        let mut session_guard = state.current_session.lock().map_err(|e| e.to_string())?;
        if let Some(in_memory_metadata) = session_guard.take() {
            let id = in_memory_metadata.id.clone();

            let mut metadata = match storage::read_metadata(&id) {
                Ok(m) => m,
                Err(_) => in_memory_metadata
            };

            metadata.status = "processing".to_string();
            metadata.audio.mic = Some(storage::AudioInfo {
                file: "raw_mic.ogg".to_string(),
                duration_sec,
                sample_rate: 48000,
                channels: 2,
            });
            metadata.audio.system = Some(storage::AudioInfo {
                file: "raw_system.ogg".to_string(),
                duration_sec,
                sample_rate: 48000,
                channels: 2,
            });

            if let Err(e) = storage::write_metadata(&metadata) {
                eprintln!("Failed to write initial metadata: {}", e);
            }

            let id = metadata.id.clone();

            // Finalization in background thread — handle is STORED, not fire-and-forget
            let finalization_id = id.clone();
            Some(std::thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    finalize_recording(&finalization_id, duration_sec, save_mix_only);
                }));
                if result.is_err() {
                    eprintln!("Finalization thread panicked for {}", finalization_id);
                    if let Ok(mut m) = storage::read_metadata(&finalization_id) {
                        m.status = "error".to_string();
                        let _ = storage::write_metadata(&m);
                    }
                }
            }))
        } else {
            None
        }
    };

    // Store finalization handle so it can be joined on shutdown or next recording
    if let Some(handle) = finalization_handle {
        if let Ok(mut h) = state.finalization_handle.lock() {
            // Join any previous handle first
            if let Some(prev) = h.take() {
                let _ = prev.join();
            }
            *h = Some(handle);
        }
    }

    *is_recording = false;

    crate::mic_audio::reset_audio_level();
    crate::system_audio::reset_system_audio_level();

    Ok(())
}

/// Background finalization: cleanup files and update metadata to "ready"
fn finalize_recording(id: &str, duration_sec: f64, save_mix_only: bool) {
    let dir = storage::get_recording_dir(id);
    let mix_path = dir.join("audio_mix.ogg");
    let mic_path = dir.join("raw_mic.ogg");
    let system_path = dir.join("raw_system.ogg");

    // Auto-Discard check
    let settings = crate::config::load_settings();
    let threshold = settings.auto_discard_seconds as f64;
    if duration_sec < threshold {
        eprintln!("Discarding recording {} (duration {:.2}s < threshold {:.2}s)", id, duration_sec, threshold);
        let _ = storage::delete_recording(id);
        return;
    }

    let mix_exists = mix_path.exists();

    // Delete separate files if save_mix_only
    if save_mix_only {
        let _ = std::fs::remove_file(&mic_path);
        let _ = std::fs::remove_file(&system_path);
    }

    // Final metadata update
    match storage::read_metadata(id) {
        Ok(mut latest_metadata) => {
            latest_metadata.status = "ready".to_string();

            if !save_mix_only {
                latest_metadata.audio.mic = Some(storage::AudioInfo {
                    file: "raw_mic.ogg".to_string(),
                    duration_sec,
                    sample_rate: 48000,
                    channels: 2,
                });
                latest_metadata.audio.system = Some(storage::AudioInfo {
                    file: "raw_system.ogg".to_string(),
                    duration_sec,
                    sample_rate: 48000,
                    channels: 2,
                });
            } else {
                latest_metadata.audio.mic = None;
                latest_metadata.audio.system = None;
            }

            if mix_exists {
                latest_metadata.audio.mix = Some(storage::AudioInfo {
                    file: "audio_mix.ogg".to_string(),
                    duration_sec,
                    sample_rate: 48000,
                    channels: 2,
                });
            }

            if let Err(e) = storage::write_metadata(&latest_metadata) {
                eprintln!("Failed to save final metadata for {}: {}", id, e);
            }
        },
        Err(e) => eprintln!("Failed to reload metadata for {}: {}", id, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time test: AudioState must be Send + Sync
    /// This verifies Story 7.1 - compiler enforces thread safety after removing unsafe impls
    #[test]
    fn test_audio_state_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<AudioState>();
        assert_sync::<AudioState>();
    }
}

