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

    /// Whether to save only the mix file (not separate mic/system files)
    pub save_mix_only: Mutex<bool>,
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
            save_mix_only: Mutex::new(true),
        }
    }
}

#[tauri::command]
pub fn start_recording(state: State<'_, AudioState>, tags: Vec<String>, save_mix_only: Option<bool>) -> Result<storage::RecordingMetadata, String> {
    let mut is_recording = state.is_recording.lock().map_err(|e| e.to_string())?;
    if *is_recording {
        return Err("Already recording".to_string());
    }

    // Create recording metadata with UUID
    let title = tags.join(" ");
    let metadata = storage::create_recording(title, tags)?;

    // Determine whether to save only mix file (default: true)
    let mix_only = save_mix_only.unwrap_or(true);

    // Store the save_mix_only setting for use during stop_recording
    *state.save_mix_only.lock().map_err(|e| e.to_string())? = mix_only;

    // --- Microphone Capture (OGG) ---
    // Always use system default device
    // If mix_only, skip writing mic file but still capture for real-time mixing
    let mic_path = storage::get_recording_dir(&metadata.id).join("raw_mic.ogg");

    match crate::mic_audio::start_mic_capture(mic_path, None, mix_only) {
        Ok(recorder) => {
            *state.mic_recorder.lock().map_err(|e| e.to_string())? = Some(recorder);
        },
        Err(e) => {
            return Err(format!("Microphone capture failed: {}", e));
        }
    }

    // --- System Audio (OGG) ---
    // If mix_only, skip writing system file but still capture for real-time mixing
    let system_path = storage::get_recording_dir(&metadata.id).join("raw_system.ogg");
    match crate::system_audio::start_system_capture(system_path, mix_only) {
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

    // Read save_mix_only before spawning thread
    let save_mix_only = *state.save_mix_only.lock().map_err(|e| e.to_string())?;

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

            // 2. Background Processing (fast - no file reading)
            let id = metadata.id.clone();

            std::thread::spawn(move || {
                let start_time = chrono::Local::now().format("%H:%M:%S%.3f").to_string();

                let dir = storage::get_recording_dir(&id);
                let mix_path = dir.join("audio_mix.ogg");
                let mic_path = dir.join("raw_mic.ogg");
                let system_path = dir.join("raw_system.ogg");

                println!("[{}] Finalizing recording session: {} (mix_only: {})", start_time, id, save_mix_only);

                // Small delay to ensure encoder threads flush their files
                std::thread::sleep(std::time::Duration::from_millis(200));

                // A. Auto-Discard check
                let settings = crate::config::load_settings();
                let threshold = settings.auto_discard_seconds as f64;

                if duration_sec < threshold {
                    println!("Discarding recording {} (duration {:.2}s < threshold {:.2}s)", id, duration_sec, threshold);
                    let _ = storage::delete_recording(&id);
                    return;
                }

                // B. Check if mix exists (real-time mixer should have created it)
                let mix_exists = mix_path.exists();
                if !mix_exists {
                    println!("  Warning: Real-time mix not found for {}", id);
                }

                // C. Delete separate files if save_mix_only is enabled
                if save_mix_only {
                    if mic_path.exists() {
                        if let Err(e) = std::fs::remove_file(&mic_path) {
                            eprintln!("Failed to delete mic file: {}", e);
                        } else {
                            println!("  Deleted raw_mic.ogg (save_mix_only enabled)");
                        }
                    }
                    if system_path.exists() {
                        if let Err(e) = std::fs::remove_file(&system_path) {
                            eprintln!("Failed to delete system file: {}", e);
                        } else {
                            println!("  Deleted raw_system.ogg (save_mix_only enabled)");
                        }
                    }
                }

                // D. FINAL SAVE: Use known values (no slow file reading!)
                match storage::read_metadata(&id) {
                    Ok(mut latest_metadata) => {
                        latest_metadata.status = "ready".to_string();

                        // Only include mic/system audio info if files exist
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

                        println!("  Duration: {:.2}s, Mix: {}, SaveMixOnly: {}", duration_sec, if mix_exists { "yes" } else { "no" }, save_mix_only);
                    },
                    Err(e) => eprintln!("Failed to reload metadata for {}: {}", id, e),
                }

                let end_time = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                println!("[{}] Recording ready: {}", end_time, id);
            });
        }
    }

    *is_recording = false;

    // Reset audio levels for waveform visualization
    crate::mic_audio::reset_audio_level();
    crate::system_audio::reset_system_audio_level();

    Ok(())
}

