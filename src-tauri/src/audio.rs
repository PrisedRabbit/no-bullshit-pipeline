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
    pub stream: Mutex<Option<cpal::Stream>>,
    pub writer: Mutex<Option<WavWriterHandle>>,
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
            stream: Mutex::new(None),
            writer: Mutex::new(None),
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

    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("No input device available")?;
    let config = device.default_input_config().map_err(|e| e.to_string())?;

    // Create recording metadata with UUID
    let title = tags.join(" ");
    let metadata = storage::create_recording(title, tags)?;
    
    // Audio file path
    let audio_path = storage::get_recording_dir(&metadata.id).join("raw.wav");
    let spec = hound::WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let writer = hound::WavWriter::create(&audio_path, spec).map_err(|e| e.to_string())?;
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

    *state.stream.lock().map_err(|e| e.to_string())? = Some(stream);
    *state.writer.lock().map_err(|e| e.to_string())? = Some(writer_handle);
    *state.current_session.lock().map_err(|e| e.to_string())? = Some(metadata);
    *is_recording = true;

    Ok(())
}

#[tauri::command]
pub fn pause_recording(state: State<'_, AudioState>) -> Result<(), String> {
    let stream_guard = state.stream.lock().map_err(|e| e.to_string())?;
    if let Some(stream) = stream_guard.as_ref() {
        stream.pause().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn resume_recording(state: State<'_, AudioState>) -> Result<(), String> {
    let stream_guard = state.stream.lock().map_err(|e| e.to_string())?;
    if let Some(stream) = stream_guard.as_ref() {
        stream.play().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn stop_recording(state: State<'_, AudioState>) -> Result<(), String> {
    let mut is_recording = state.is_recording.lock().map_err(|e| e.to_string())?;
    if !*is_recording {
        return Ok(());
    }

    // Drop stream to stop it
    {
        let mut stream_guard = state.stream.lock().map_err(|e| e.to_string())?;
        *stream_guard = None;
    }
    
    // Finalize writer and update meta
    {
         let mut writer_guard = state.writer.lock().map_err(|e| e.to_string())?;
         if let Some(handle) = writer_guard.take() {
             if let Ok(mut inner_guard) = handle.lock() {
                 if let Some(w) = inner_guard.take() {
                     w.finalize().map_err(|e| e.to_string())?;
                 }
             }
         }
    }

    // Update metadata with final duration
    {
        let mut session_guard = state.current_session.lock().map_err(|e| e.to_string())?;
        if let Some(metadata) = session_guard.as_mut() {
            // Calculate duration from the WAV file
            let audio_path = storage::get_recording_dir(&metadata.id).join("raw.wav");
            if let Ok(reader) = hound::WavReader::open(&audio_path) {
                let spec = reader.spec();
                let duration_samples = reader.len();
                let duration_sec = duration_samples as f64 / (spec.sample_rate as f64 * spec.channels as f64);
                metadata.audio.duration_sec = duration_sec;
                
                // Write updated metadata
                storage::write_metadata(metadata)?;
            }
        }
    }

    *is_recording = false;
    Ok(())
}
