use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fs::File;
use std::io::BufWriter;
use tauri::State;
use crate::storage::{self, RecordingMetadata};

// Simplified writer type for readability
type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

// Structure to hold the audio stream and state
pub struct AudioState {
    pub is_recording: Mutex<bool>,
    // Microphone (cpal)
    pub mic_stream: Mutex<Option<cpal::Stream>>,
    pub mic_writer: Mutex<Option<WavWriterHandle>>,
    // System Audio (ScreenCaptureKit)
    pub system_recorder: Mutex<Option<crate::system_audio::SystemAudioRecorder>>,
    
    pub current_session: Mutex<Option<RecordingMetadata>>,
}

// Explicitly implement Send/Sync. Since all fields are wrapped in Mutex, 
// this is safe for Tauri's multi-threaded runtime.
unsafe impl Send for AudioState {}
unsafe impl Sync for AudioState {}

impl AudioState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
            mic_stream: Mutex::new(None),
            mic_writer: Mutex::new(None),
            system_recorder: Mutex::new(None),
            current_session: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn start_recording(state: State<'_, AudioState>, tags: Vec<String>) -> Result<(), String> {
    let mut is_recording = state.is_recording.lock().map_err(|e| e.to_string())?;
    if *is_recording {
        return Err("Already recording".to_string());
    }

    // --- Microphone Capture (cpal) ---
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("No input device available")?;
    let config = device.default_input_config().map_err(|e| e.to_string())?;

    // Create recording metadata with UUID
    let title = tags.join(" ");
    let metadata = storage::create_recording(title, tags)?;
    
    // Mic Audio file path
    let mic_audio_path = storage::get_recording_dir(&metadata.id).join("raw_mic.wav");
    let spec = hound::WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let writer = hound::WavWriter::create(&mic_audio_path, spec).map_err(|e| e.to_string())?;
    let writer_handle = Arc::new(Mutex::new(Some(writer)));
    
    // Store handle for the callback
    let callback_writer = writer_handle.clone();
    
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    
    let stream = match config.sample_format() {
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _: &_| {
                if let Ok(mut guard) = callback_writer.lock() {
                    if let Some(w) = guard.as_mut() {
                        for &sample in data {
                            let _ = w.write_sample(sample);
                        }
                    }
                }
            },
            err_fn,
            None
        ),
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| {
                if let Ok(mut guard) = callback_writer.lock() {
                    if let Some(w) = guard.as_mut() {
                        for &sample in data {
                            let sample_i16 = (sample * i16::MAX as f32) as i16;
                            let _ = w.write_sample(sample_i16);
                        }
                    }
                }
            },
            err_fn,
            None
        ),
        _ => return Err("Unsupported sample format".to_string()),
    }.map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;

    // --- Update State ---
    *state.mic_stream.lock().map_err(|e| e.to_string())? = Some(stream);
    *state.mic_writer.lock().map_err(|e| e.to_string())? = Some(writer_handle);
    
    // --- System Audio ---
    let system_wav_path = storage::get_recording_dir(&metadata.id).join("raw_system.wav");
    match crate::system_audio::start_system_capture(system_wav_path) {
        Ok(recorder) => {
            *state.system_recorder.lock().map_err(|e| e.to_string())? = Some(recorder);
        },
        Err(e) => {
            println!("System audio capture failed: {}", e);
            // Proceed with Mic only
        }
    }
    
    *state.current_session.lock().map_err(|e| e.to_string())? = Some(metadata);
    *is_recording = true;

    Ok(())
}

#[tauri::command]
pub fn pause_recording(state: State<'_, AudioState>) -> Result<(), String> {
    // Pause microphone
    let mic_guard = state.mic_stream.lock().map_err(|e| e.to_string())?;
    if let Some(stream) = mic_guard.as_ref() {
        stream.pause().map_err(|e| e.to_string())?;
    }
    
    // Pause system audio (TODO: Implement SCK pause)
    
    Ok(())
}

#[tauri::command]
pub fn resume_recording(state: State<'_, AudioState>) -> Result<(), String> {
    // Resume microphone
    let mic_guard = state.mic_stream.lock().map_err(|e| e.to_string())?;
    if let Some(stream) = mic_guard.as_ref() {
        stream.play().map_err(|e| e.to_string())?;
    }
    
    // Resume system audio (TODO: Implement SCK resume)
    
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
        let mut stream_guard = state.mic_stream.lock().map_err(|e| e.to_string())?;
        *stream_guard = None;
    }
    
    // Finalize mic writer
    {
         let mut writer_guard = state.mic_writer.lock().map_err(|e| e.to_string())?;
         if let Some(handle) = writer_guard.take() {
             if let Ok(mut inner_guard) = handle.lock() {
                 if let Some(w) = inner_guard.take() {
                     w.finalize().map_err(|e| e.to_string())?;
                 }
             }
         }
    }
    
    // --- Stop System Audio ---
    {
        let mut system_guard = state.system_recorder.lock().map_err(|e| e.to_string())?;
        if let Some(mut recorder) = system_guard.take() {
             recorder.stop();
        }
    }

    // --- Update Metadata ---
    {
        let mut session_guard = state.current_session.lock().map_err(|e| e.to_string())?;
        if let Some(metadata) = session_guard.as_mut() {
            // Update Mic Info
            let mic_path = storage::get_recording_dir(&metadata.id).join("raw_mic.wav");
            if let Ok(reader) = hound::WavReader::open(&mic_path) {
                let spec = reader.spec();
                let duration_samples = reader.len();
                let duration_sec = duration_samples as f64 / (spec.sample_rate as f64 * spec.channels as f64);
                
                metadata.audio.mic = Some(storage::AudioInfo {
                    file: "raw_mic.wav".to_string(),
                    duration_sec,
                    sample_rate: spec.sample_rate,
                    channels: spec.channels,
                });
            }
            
            // Update System Info
            let system_path = storage::get_recording_dir(&metadata.id).join("raw_system.wav");
            if system_path.exists() {
                 if let Ok(reader) = hound::WavReader::open(&system_path) {
                    let spec = reader.spec();
                    let duration_samples = reader.len();
                    if spec.sample_rate > 0 && spec.channels > 0 {
                         let duration_sec = duration_samples as f64 / (spec.sample_rate as f64 * spec.channels as f64);
                         metadata.audio.system = Some(storage::AudioInfo {
                            file: "raw_system.wav".to_string(),
                            duration_sec,
                            sample_rate: spec.sample_rate,
                            channels: spec.channels,
                        });
                    }
                 }
            }
            
            // Write updated metadata
            storage::write_metadata(metadata)?;
        }
    }

    *is_recording = false;
    Ok(())
}
