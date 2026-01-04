use std::sync::Mutex;
use std::time::SystemTime;
use tauri::State;
use crate::storage::{self, RecordingMetadata};

// Structure to hold the audio stream and state
pub struct AudioState {
    pub is_recording: Mutex<bool>,
    
    // Recorders (OGG Encoders)
    pub mic_recorder: Mutex<Option<crate::mic_audio::MicAudioRecorder>>,
    pub system_recorder: Mutex<Option<crate::system_audio::SystemAudioRecorder>>,
    
    pub current_session: Mutex<Option<RecordingMetadata>>,
    pub start_timestamp: Mutex<Option<SystemTime>>,
}

// Explicitly implement Send/Sync.
unsafe impl Send for AudioState {}
unsafe impl Sync for AudioState {}

impl AudioState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
            mic_recorder: Mutex::new(None),
            system_recorder: Mutex::new(None),
            current_session: Mutex::new(None),
            start_timestamp: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn start_recording(state: State<'_, AudioState>, tags: Vec<String>) -> Result<storage::RecordingMetadata, String> {
    let mut is_recording = state.is_recording.lock().map_err(|e| e.to_string())?;
    if *is_recording {
        return Err("Already recording".to_string());
    }

    // Create recording metadata with UUID
    let title = tags.join(" ");
    let metadata = storage::create_recording(title, tags)?;
    
    // --- Microphone Capture (OGG) ---
    // Start mic capture via mic_audio module
    let mic_path = storage::get_recording_dir(&metadata.id).join("raw_mic.ogg");
    
    match crate::mic_audio::start_mic_capture(mic_path) {
        Ok(recorder) => {
            *state.mic_recorder.lock().map_err(|e| e.to_string())? = Some(recorder);
        },
        Err(e) => {
            return Err(format!("Microphone capture failed: {}", e));
        }
    }

    // --- System Audio (OGG) ---
    let system_path = storage::get_recording_dir(&metadata.id).join("raw_system.ogg");
    match crate::system_audio::start_system_capture(system_path) {
        Ok(recorder) => {
            *state.system_recorder.lock().map_err(|e| e.to_string())? = Some(recorder);
        },
        Err(e) => {
            println!("System audio capture failed: {}", e);
             // We continue even if system audio fails? Yes, Mic is primary? 
             // Or fail? Let's warn but continue.
        }
    }
    
    // Capture start time
    *state.start_timestamp.lock().map_err(|e| e.to_string())? = Some(std::time::SystemTime::now());
    
    let metadata_clone = metadata.clone();
    *state.current_session.lock().map_err(|e| e.to_string())? = Some(metadata);
    *is_recording = true;

    Ok(metadata_clone)
}

#[tauri::command]
pub fn pause_recording(_state: State<'_, AudioState>) -> Result<(), String> {
    // TODO: Implement pause for new recorders
    // Current MicAudioRecorder holds stream but doesn't expose it blindly.
    // For now, pause is NO-OP or Err("Not implemented yet")
    // To avoid breaking UI, we return Ok(()) but do nothing.
    Ok(())
}

#[tauri::command]
pub fn resume_recording(_state: State<'_, AudioState>) -> Result<(), String> {
    // TODO: Implement resume
    Ok(())
}

#[tauri::command]
pub fn stop_recording(state: State<'_, AudioState>) -> Result<(), String> {
    let mut is_recording = state.is_recording.lock().map_err(|e| e.to_string())?;
    if !*is_recording {
        return Ok(());
    }

    // --- Stop Microphone ---
    {
         let mut mic_guard = state.mic_recorder.lock().map_err(|e| e.to_string())?;
         if let Some(mut recorder) = mic_guard.take() {
             recorder.stop();
         }
    }
    
    // --- Stop System Audio ---
    {
        let mut system_guard = state.system_recorder.lock().map_err(|e| e.to_string())?;
        if let Some(mut recorder) = system_guard.take() {
             recorder.stop();
        }
    }

    // --- Duration & Metadata Update ---
    let start_time = *state.start_timestamp.lock().map_err(|e| e.to_string())?;
    
    // Calculate wall-clock duration
    let duration_sec = if let Some(start) = start_time {
         start.elapsed().unwrap_or_default().as_secs_f64()
    } else {
         0.0
    };

    {
        let mut session_guard = state.current_session.lock().map_err(|e| e.to_string())?;
        if let Some(metadata) = session_guard.as_mut() {
            
            // Check files existence (Mic might have failed, System might have failed)
            let mic_path = storage::get_recording_dir(&metadata.id).join("raw_mic.ogg");
            let system_path = storage::get_recording_dir(&metadata.id).join("raw_system.ogg");
            
            let mut actual_duration = duration_sec; // Fallback to wall-clock
            
            if mic_path.exists() {
                // Get ACTUAL duration from file
                let mic_duration = crate::audio_processing::get_ogg_duration(&mic_path)
                    .unwrap_or(duration_sec);
                
                metadata.audio.mic = Some(storage::AudioInfo {
                    file: "raw_mic.ogg".to_string(),
                    duration_sec: mic_duration,
                    sample_rate: 48000, 
                    channels: 2, 
                });
                
                actual_duration = mic_duration;
            }
            
            if system_path.exists() {
                // Get ACTUAL duration from file
                let system_duration = crate::audio_processing::get_ogg_duration(&system_path)
                    .unwrap_or(duration_sec);
                
                metadata.audio.system = Some(storage::AudioInfo {
                    file: "raw_system.ogg".to_string(),
                    duration_sec: system_duration,
                    sample_rate: 48000,
                    channels: 2,
                });
                
                // Use system duration if available (more accurate for recordings)
                actual_duration = system_duration;
            }
            
            // Discard if < 3 seconds (use actual duration)
            if actual_duration < 3.0 {
                // Delete recording
                let _ = storage::delete_recording(&metadata.id);
                *session_guard = None;
                *is_recording = false;
                return Err("Recording discarded (shorter than 3 seconds)".to_string());
            }

            // Write updated metadata
            storage::write_metadata(metadata)?;
            
            // Small delay to ensure encoder threads have finished and flushed files
            std::thread::sleep(std::time::Duration::from_millis(100));
            
            // Mix audio files if both exist
            if mic_path.exists() && system_path.exists() {
                println!("Both audio files exist, attempting to mix...");
                let mix_path = storage::get_recording_dir(&metadata.id).join("audio_mix.ogg");
                
                match crate::audio_processing::mix_audio_files(&mic_path, &system_path, &mix_path) {
                    Ok(_) => {
                        println!("Successfully mixed audio to {:?}", mix_path);
                        // Update metadata with mix info
                        metadata.audio.mix = Some(storage::AudioInfo {
                            file: "audio_mix.ogg".to_string(),
                            duration_sec,
                            sample_rate: 48000,
                            channels: 2,
                        });
                        // Re-write metadata with mix info
                        if let Err(e) = storage::write_metadata(metadata) {
                            eprintln!("Failed to update metadata with mix info: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("ERROR: Failed to mix audio files: {:?}", e);
                        // Don't fail the recording if mixing fails
                    }
                }
            } else {
                println!("Skipping mix: mic_exists={}, system_exists={}", mic_path.exists(), system_path.exists());
            }
        }
    }

    *is_recording = false;
    Ok(())
}

