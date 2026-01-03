use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use tauri::State;
use chrono::Local;

// Structure to hold the audio stream and state
pub struct AudioState {
    pub is_recording: Mutex<bool>,
    // We keep the stream in an Option. When defined, it's running (or paused).
    // cpal Stream is !Send usually, so we might need to handle it carefully.
    // simpler MVP: just a flag, and the stream runs in a background thread? 
    // No, stream must be kept alive.
    // cpal::Stream is Send on some platforms but safe to wrap in Mutex.
    pub stream: Mutex<Option<cpal::Stream>>,
    pub writer: Mutex<Option<Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>>>,
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
            stream: Mutex::new(None),
            writer: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn start_recording(state: State<'_, AudioState>, app_handle: tauri::AppHandle, tags: Vec<String>) -> Result<(), String> {
    let mut is_recording = state.is_recording.lock().map_err(|e| e.to_string())?;
    if *is_recording {
        return Err("Already recording".to_string());
    }

    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("No input device available")?;
    let config = device.default_input_config().map_err(|e| e.to_string())?;

    // Prepare file path
    // For MVP, just save to current directory or a 'recordings' folder
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let filename = format!("recording_{}.wav", timestamp); // Using wav for MVP
    let spec = hound::WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    // Ensure "recordings" dir exists
    let _ = std::fs::create_dir("recordings");
    let path = PathBuf::from("recordings").join(&filename);
    
    let writer = hound::WavWriter::create(&path, spec).map_err(|e| e.to_string())?;
    let writer = Arc::new(Mutex::new(Some(writer)));
    
    // Store writer for the callback
    let writer_clone = writer.clone();
    
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    
    let stream = match config.sample_format() {
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _: &_| {
                write_input_data(data, &writer_clone)
            },
            err_fn,
            None // timeout
        ),
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| {
                write_input_data_f32(data, &writer_clone)
            },
            err_fn,
            None
        ),
        _ => return Err("Unsupported sample format".to_string()),
    }.map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;

    *state.stream.lock().map_err(|e| e.to_string())? = Some(stream);
    *state.writer.lock().map_err(|e| e.to_string())? = Some(writer);
    *is_recording = true;

    // TODO: Save meta.json with tags

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
    
    // Finalize writer
    {
         let mut writer_guard = state.writer.lock().map_err(|e| e.to_string())?;
         if let Some(arc_writer) = writer_guard.take() {
             let mut inner_writer = arc_writer.lock().map_err(|e| e.to_string())?;
             if let Some(w) = inner_writer.take() {
                 w.finalize().map_err(|e| e.to_string())?;
             }
         }
    }

    *is_recording = false;
    Ok(())
}

fn write_input_data(input: &[i16], writer: &Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>> >) {
    if let Ok(mut guard) = writer.lock() {
        if let Some(w) = guard.as_mut() {
            for &sample in input.iter() {
                let _ = w.write_sample(sample);
            }
        }
    }
}

fn write_input_data_f32(input: &[f32], writer: &Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>> >) {
    if let Ok(mut guard) = writer.lock() {
        if let Some(w) = guard.as_mut() {
            for &sample in input.iter() {
                let sample_i16 = (sample * i16::MAX as f32) as i16;
                let _ = w.write_sample(sample_i16);
            }
        }
    }
}
