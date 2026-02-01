use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

/// Playback state
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PlaybackStatus {
    Stopped,
    Playing,
    Paused,
}

#[derive(Serialize, Clone, Debug)]
pub struct PlaybackState {
    pub status: PlaybackStatus,
    pub current_position_ms: u64,
    pub duration_ms: u64,
    pub recording_id: Option<String>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            status: PlaybackStatus::Stopped,
            current_position_ms: 0,
            duration_ms: 0,
            recording_id: None,
        }
    }
}

// Thread-safe playback control signals
static STOP_SIGNAL: AtomicBool = AtomicBool::new(false);
static PAUSE_SIGNAL: AtomicBool = AtomicBool::new(false);
static IS_PLAYING: AtomicBool = AtomicBool::new(false);
static CURRENT_POSITION_MS: AtomicU64 = AtomicU64::new(0);
static DURATION_MS: AtomicU64 = AtomicU64::new(0);

lazy_static::lazy_static! {
    static ref CURRENT_RECORDING_ID: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
}

/// Start audio playback on a dedicated thread
#[tauri::command]
pub fn play_audio(recording_id: String) -> Result<(), String> {
    // Stop any existing playback
    stop_audio()?;

    let recording_dir = crate::storage::get_data_dir().join(&recording_id);
    let mut audio_path = recording_dir.join("audio_mix.ogg");

    if !audio_path.exists() {
        audio_path = recording_dir.join("raw_mic.ogg");
    }

    if !audio_path.exists() {
        return Err("Audio file not found".to_string());
    }

    // Get duration
    let duration_ms = match crate::waveform::get_ogg_file_info(&audio_path) {
        Ok(info) => (info.duration_sec * 1000.0) as u64,
        Err(_) => 0,
    };

    DURATION_MS.store(duration_ms, Ordering::SeqCst);
    CURRENT_POSITION_MS.store(0, Ordering::SeqCst);
    STOP_SIGNAL.store(false, Ordering::SeqCst);
    PAUSE_SIGNAL.store(false, Ordering::SeqCst);
    IS_PLAYING.store(true, Ordering::SeqCst);

    if let Ok(mut id) = CURRENT_RECORDING_ID.lock() {
        *id = Some(recording_id.clone());
    }

    // Spawn high-priority playback thread
    let path = audio_path.clone();
    thread::Builder::new()
        .name("audio-playback".to_string())
        .spawn(move || {
            if let Err(e) = run_playback(path) {
                eprintln!("Playback error: {}", e);
            }
            IS_PLAYING.store(false, Ordering::SeqCst);
        })
        .map_err(|e| format!("Failed to spawn playback thread: {}", e))?;

    Ok(())
}

fn run_playback(audio_path: std::path::PathBuf) -> Result<(), String> {
    // Create output stream (must be on this thread)
    let (_stream, stream_handle) = OutputStream::try_default()
        .map_err(|e| format!("Audio output error: {}", e))?;

    let sink = Sink::try_new(&stream_handle)
        .map_err(|e| format!("Sink error: {}", e))?;

    // Open with larger buffer for smooth playback
    let file = File::open(&audio_path)
        .map_err(|e| format!("File error: {}", e))?;
    let reader = BufReader::with_capacity(256 * 1024, file); // 256KB buffer

    let source = Decoder::new(reader)
        .map_err(|e| format!("Decode error: {}", e))?;

    sink.append(source);
    sink.play();

    let start = std::time::Instant::now();
    let duration_ms = DURATION_MS.load(Ordering::SeqCst);

    // Minimal monitoring loop - let rodio do the work
    while !sink.empty() && !STOP_SIGNAL.load(Ordering::Relaxed) {
        // Handle pause
        if PAUSE_SIGNAL.load(Ordering::Relaxed) {
            sink.pause();
            while PAUSE_SIGNAL.load(Ordering::Relaxed) && !STOP_SIGNAL.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(100));
            }
            if !STOP_SIGNAL.load(Ordering::Relaxed) {
                sink.play();
            }
        }

        // Update position (less frequently to reduce overhead)
        let elapsed = start.elapsed().as_millis() as u64;
        let position = elapsed.min(duration_ms);
        CURRENT_POSITION_MS.store(position, Ordering::Relaxed);

        // Sleep longer to reduce CPU usage and prevent interference
        thread::sleep(Duration::from_millis(200));
    }

    sink.stop();
    CURRENT_POSITION_MS.store(0, Ordering::Relaxed);

    Ok(())
}

/// Pause audio playback
#[tauri::command]
pub fn pause_audio() -> Result<(), String> {
    PAUSE_SIGNAL.store(true, Ordering::SeqCst);
    Ok(())
}

/// Resume audio playback
#[tauri::command]
pub fn resume_audio() -> Result<(), String> {
    PAUSE_SIGNAL.store(false, Ordering::SeqCst);
    Ok(())
}

/// Stop audio playback
#[tauri::command]
pub fn stop_audio() -> Result<(), String> {
    STOP_SIGNAL.store(true, Ordering::SeqCst);
    PAUSE_SIGNAL.store(false, Ordering::SeqCst);
    // Give thread time to stop
    thread::sleep(Duration::from_millis(50));
    IS_PLAYING.store(false, Ordering::SeqCst);
    CURRENT_POSITION_MS.store(0, Ordering::SeqCst);
    Ok(())
}

/// Seek to position (not fully supported - restarts from beginning)
#[tauri::command]
pub fn seek_audio(_position_ms: u64) -> Result<(), String> {
    // Seeking not supported with basic rodio Decoder
    Ok(())
}

/// Get current playback state
#[tauri::command]
pub fn get_playback_state() -> PlaybackState {
    let is_playing = IS_PLAYING.load(Ordering::Relaxed);
    let is_paused = PAUSE_SIGNAL.load(Ordering::Relaxed);

    let status = if !is_playing {
        PlaybackStatus::Stopped
    } else if is_paused {
        PlaybackStatus::Paused
    } else {
        PlaybackStatus::Playing
    };

    let recording_id = CURRENT_RECORDING_ID.lock()
        .map(|id| id.clone())
        .unwrap_or(None);

    PlaybackState {
        status,
        current_position_ms: CURRENT_POSITION_MS.load(Ordering::Relaxed),
        duration_ms: DURATION_MS.load(Ordering::Relaxed),
        recording_id,
    }
}
