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
    
    // Start real-time mixer (reads from shared buffers, not files)
    let mix_path = storage::get_recording_dir(&metadata.id).join("audio_mix.ogg");
    match crate::audio_processing::RealtimeMixer::new(mix_path) {
        Ok(mixer) => {
            *state.realtime_mixer.lock().map_err(|e| e.to_string())? = Some(mixer);
            println!("Real-time mixer started (buffer-based)");
        },
        Err(e) => {
            println!("Real-time mixer failed: {}", e);
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
        if let Some(in_memory_metadata) = session_guard.take() {
            let id = in_memory_metadata.id.clone();
            
            // Re-load from disk to get latest title/tags edited during recording
            let mut metadata = match storage::read_metadata(&id) {
                Ok(m) => m,
                Err(_) => in_memory_metadata // Fallback to in-memory if disk read fails
            };

            // 1. Immediate Update: Save wall-clock duration
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
            
            // Save state as 'processing'
            if let Err(e) = storage::write_metadata(&metadata) {
                 eprintln!("Failed to write initial metadata: {}", e);
            }

            // 2. Background Processing
            // We pass ONLY the ID and initial duration to avoid stale metadata issues
            let id = metadata.id.clone();
            
            std::thread::spawn(move || {
                use std::time::Instant;
                let overall_start = Instant::now();
                let start_time = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                
                let dir = storage::get_recording_dir(&id);
                let mic_path = dir.join("raw_mic.ogg");
                let system_path = dir.join("raw_system.ogg");
                let mix_path = dir.join("audio_mix.ogg");
                
                println!("[{}] Finalizing recording session: {}", start_time, id);

                // Small delay to ensure encoder threads flush their files
                std::thread::sleep(std::time::Duration::from_millis(500));

                // A. Use wall-clock duration (already accurate, instant)
                let actual_duration = duration_sec;
                println!("  Using wall-clock duration: {:.2}s", actual_duration);

                // B. Auto-Discard
                let settings = crate::config::load_settings();
                let threshold = settings.auto_discard_seconds as f64;
                
                if actual_duration < threshold {
                    println!("Discarding recording {} (duration {:.2}s < threshold {:.2}s)", id, actual_duration, threshold);
                    let _ = storage::delete_recording(&id);
                    return;
                }

                // C. Mix (if not already done by realtime_mixer)
                let mut mix_exists = mix_path.exists();
                
                if !mix_exists && mic_path.exists() && system_path.exists() {
                    let step_start = Instant::now();
                    println!("Real-time mix missing, falling back to post-mix for {}...", id);
                    if let Ok(_) = crate::audio_processing::mix_audio_files(&mic_path, &system_path, &mix_path) {
                        mix_exists = true;
                        println!("  [TIMING] Post-processing mix: {:.3}s", step_start.elapsed().as_secs_f64());
                    }
                } else if mix_exists {
                    println!("  Real-time mix exists, skipping post-mix");
                }

                // D. FINAL SAVE: Reload metadata and update with ACTUAL file info
                match storage::read_metadata(&id) {
                    Ok(mut latest_metadata) => {
                        latest_metadata.status = "ready".to_string();

                        // Get actual file info from OGG files (correct duration/sample_rate)
                        if mic_path.exists() {
                            if let Ok(info) = crate::waveform::get_ogg_file_info(&mic_path) {
                                latest_metadata.audio.mic = Some(storage::AudioInfo {
                                    file: "raw_mic.ogg".to_string(),
                                    duration_sec: info.duration_sec,
                                    sample_rate: info.sample_rate,
                                    channels: info.channels,
                                });
                                println!("  Mic: {:.2}s @ {}Hz {}ch", info.duration_sec, info.sample_rate, info.channels);
                            }
                        }

                        if system_path.exists() {
                            if let Ok(info) = crate::waveform::get_ogg_file_info(&system_path) {
                                latest_metadata.audio.system = Some(storage::AudioInfo {
                                    file: "raw_system.ogg".to_string(),
                                    duration_sec: info.duration_sec,
                                    sample_rate: info.sample_rate,
                                    channels: info.channels,
                                });
                                println!("  System: {:.2}s @ {}Hz {}ch", info.duration_sec, info.sample_rate, info.channels);
                            }
                        }

                        if mix_exists {
                            if let Ok(info) = crate::waveform::get_ogg_file_info(&mix_path) {
                                latest_metadata.audio.mix = Some(storage::AudioInfo {
                                    file: "audio_mix.ogg".to_string(),
                                    duration_sec: info.duration_sec,
                                    sample_rate: info.sample_rate,
                                    channels: info.channels,
                                });
                                println!("  Mix: {:.2}s @ {}Hz {}ch", info.duration_sec, info.sample_rate, info.channels);
                            }
                        }

                        if let Err(e) = storage::write_metadata(&latest_metadata) {
                            eprintln!("Failed to save final metadata for {}: {}", id, e);
                        }
                    },
                    Err(e) => eprintln!("Failed to reload metadata for background process {}: {}", id, e),
                }
                
                let total_time = overall_start.elapsed().as_secs_f64();
                let end_time = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                println!("[{}] Recording session finalized: {} (TOTAL: {:.3}s)", end_time, id, total_time);
            });
        }
    }

    *is_recording = false;
    Ok(())
}

