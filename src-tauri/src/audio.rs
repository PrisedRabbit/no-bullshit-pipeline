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
    
   // Real-time mixer
    pub realtime_mixer: Mutex<Option<crate::audio_processing::RealtimeMixer>>,
    
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
            realtime_mixer: Mutex::new(None),
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
    
    // --- Real-time Mixer ---
    // Start mixer to create audio_mix.ogg during recording
    let mix_path = storage::get_recording_dir(&metadata.id).join("audio_mix.ogg");
    let mic_path_for_mixer = storage::get_recording_dir(&metadata.id).join("raw_mic.ogg");
    let system_path_for_mixer = storage::get_recording_dir(&metadata.id).join("raw_system.ogg");
    
    match crate::audio_processing::RealtimeMixer::new(mic_path_for_mixer, system_path_for_mixer, mix_path) {
        Ok(mixer) => {
            *state.realtime_mixer.lock().map_err(|e| e.to_string())? = Some(mixer);
            println!("Real-time mixer started");
        },
        Err(e) => {
            println!("Real-time mixer failed to start: {}", e);
            // Continue without mixer - will fall back to post-processing
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

    // --- Stop Real-time Mixer First ---
    {
        let mut mixer_guard = state.realtime_mixer.lock().map_err(|e| e.to_string())?;
        if let Some(mut mixer) = mixer_guard.take() {
            mixer.stop();
            println!("Real-time mixer stopped");
        }
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
        if let Some(mut metadata) = session_guard.take() {
            // 1. Immediate Update: Save wall-clock duration so UI has something valid immediately
            metadata.status = "processing".to_string();
            metadata.audio.mic = Some(storage::AudioInfo {
                file: "raw_mic.ogg".to_string(),
                duration_sec,
                sample_rate: 48000, 
                channels: 2, 
            });
            // Assume system exists for now if we were recording, it will be corrected in background
            metadata.audio.system = Some(storage::AudioInfo {
                file: "raw_system.ogg".to_string(),
                duration_sec,
                sample_rate: 48000,
                channels: 2,
            });
            
            // Save initial state
            if let Err(e) = storage::write_metadata(&metadata) {
                 eprintln!("Failed to write initial metadata: {}", e);
            }

            // 2. Background Processing
            // We clone metadata to move into the thread
            let metadata_clone = metadata.clone();
            
            std::thread::spawn(move || {
                let mut metadata = metadata_clone;
                let id = metadata.id.clone();
                let dir = storage::get_recording_dir(&id);
                let mic_path = dir.join("raw_mic.ogg");
                let system_path = dir.join("raw_system.ogg");
                
                println!("Background processing for recording: {}", id);

                // Small delay to ensure encoder threads flush their files
                std::thread::sleep(std::time::Duration::from_millis(500));

                 // A. Refine Durations
                let mut actual_duration = duration_sec;
                
                if mic_path.exists() {
                     match crate::audio_processing::get_ogg_duration(&mic_path) {
                        Ok(d) => {
                            if let Some(ref mut audio) = metadata.audio.mic {
                                audio.duration_sec = d;
                            }
                            actual_duration = d;
                        }
                        Err(e) => eprintln!("Failed to get mic duration: {}", e),
                     }
                }
                
                if system_path.exists() {
                     match crate::audio_processing::get_ogg_duration(&system_path) {
                        Ok(d) => {
                            if let Some(ref mut audio) = metadata.audio.system {
                                audio.duration_sec = d;
                            }
                            // System duration is usually more reliable for 'overall' if it exists
                            actual_duration = d; 
                        }
                        Err(e) => eprintln!("Failed to get system duration: {}", e),
                     }
                }

                // B. Auto-Discard
                let settings = crate::config::load_settings();
                let threshold = settings.auto_discard_seconds as f64;
                
                if actual_duration < threshold {
                    println!("Discarding recording {} (duration {:.2}s < threshold {:.2}s)", id, actual_duration, threshold);
                    let _ = storage::delete_recording(&id);
                    return; // Stop processing
                }

                // C. Mix
                if mic_path.exists() && system_path.exists() {
                    let mix_path = dir.join("audio_mix.ogg");
                    println!("Mixing audio to {:?}...", mix_path);
                    
                    match crate::audio_processing::mix_audio_files(&mic_path, &system_path, &mix_path) {
                        Ok(_) => {
                            println!("Mix complete.");
                            metadata.audio.mix = Some(storage::AudioInfo {
                                file: "audio_mix.ogg".to_string(),
                                duration_sec: actual_duration,
                                sample_rate: 48000,
                                channels: 2,
                            });
                        }
                        Err(e) => eprintln!("Mixing failed: {}", e),
                    }
                }

                // D. Final Save
                metadata.status = "ready".to_string();
                if let Err(e) = storage::write_metadata(&metadata) {
                    eprintln!("Failed to save final metadata: {}", e);
                }
                
                println!("Background processing finished for {}", id);
            });
        }
    }

    *is_recording = false;
    Ok(())
}

