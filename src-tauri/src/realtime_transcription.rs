use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

use crate::audio_processing::TRANSCRIPTION_BUFFER;
use crate::storage::get_data_dir;

const EVENT_TRANSCRIPT_UPDATED: &str = "realtime_transcript_updated";
const EVENT_TRANSCRIPTION_ERROR: &str = "realtime_transcription_error";

#[derive(Clone, Serialize)]
pub struct TranscriptUpdated {
    pub recording_id: String,
    pub text: String,
}

const LOCAL_WINDOW_SAMPLES: usize = 16000 * 5;
const LOCAL_STEP_SAMPLES: usize = 16000;
const VAD_RMS_THRESHOLD: f32 = 0.005;

pub struct LocalTranscriber {
    should_stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl LocalTranscriber {
    pub fn start(
        app_handle: tauri::AppHandle,
        model_path: PathBuf,
        recording_id: String,
    ) -> Result<Self, String> {
        if !model_path.exists() {
            return Err(format!("Whisper model not found: {}", model_path.display()));
        }

        let should_stop = Arc::new(AtomicBool::new(false));
        let stop_flag = should_stop.clone();

        let handle = std::thread::spawn(move || {
            if let Err(e) = run_local_transcription(app_handle.clone(), &model_path, &recording_id, stop_flag) {
                eprintln!("Local transcription error: {}", e);
                let _ = app_handle.emit(EVENT_TRANSCRIPTION_ERROR, e);
            }
        });

        Ok(Self {
            should_stop,
            handle: Some(handle),
        })
    }

    pub fn stop(&mut self) {
        self.should_stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for LocalTranscriber {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_local_transcription(
    app_handle: tauri::AppHandle,
    model_path: &std::path::Path,
    recording_id: &str,
    should_stop: Arc<AtomicBool>,
) -> Result<(), String> {
    use std::os::raw::c_int;
    use whisper_rs::{FullParams, SamplingStrategy};

    let ctx = crate::transcription::load_whisper_context(model_path)?;

    let mut state = ctx
        .create_state()
        .map_err(|e| format!("Failed to create Whisper state: {}", e))?;

    let mut window: Vec<f32> = Vec::with_capacity(LOCAL_WINDOW_SAMPLES);
    let mut prompt_tokens: Vec<c_int> = Vec::new();
    let mut committed_text = String::new();
    let mut was_speaking = false;
    let mut last_text = String::new();
    let mut last_write_len: usize = 0;

    let recording_dir = get_data_dir().join(recording_id);
    let step_interval = std::time::Duration::from_secs(1);

    while !should_stop.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let available = TRANSCRIPTION_BUFFER.available();
        if available > 0 {
            window.extend_from_slice(&TRANSCRIPTION_BUFFER.pop(available));
        }
        if window.len() >= LOCAL_STEP_SAMPLES {
            break;
        }
    }

    while !should_stop.load(Ordering::Relaxed) {
        let step_start = std::time::Instant::now();

        let available = TRANSCRIPTION_BUFFER.available();
        if available > 0 {
            window.extend_from_slice(&TRANSCRIPTION_BUFFER.pop(available));
        }

        if window.len() > LOCAL_WINDOW_SAMPLES {
            let excess = window.len() - LOCAL_WINDOW_SAMPLES;
            window.drain(..excess);
        }

        let vad_start = window.len().saturating_sub(LOCAL_STEP_SAMPLES);
        let is_speaking = compute_rms(&window[vad_start..]) > VAD_RMS_THRESHOLD;

        if is_speaking {
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_language(None);
            params.set_translate(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);

            if !prompt_tokens.is_empty() {
                params.set_tokens(&prompt_tokens);
            }

            if let Err(e) = state.full(params, &window) {
                eprintln!("Whisper inference error: {}", e);
                sleep_remaining(step_start, step_interval, &should_stop);
                continue;
            }

            let mut text = String::new();
            let n_segments = state.full_n_segments().unwrap_or(0);
            for i in 0..n_segments {
                if let Ok(seg_text) = state.full_get_segment_text(i) {
                    text.push_str(&seg_text);
                }
            }
            let text = text.trim().to_string();

            if n_segments > 0 {
                let last_seg = n_segments - 1;
                let n_tokens = state.full_n_tokens(last_seg).unwrap_or(0);
                prompt_tokens.clear();
                for t in 0..n_tokens {
                    if let Ok(tid) = state.full_get_token_id(last_seg, t) {
                        prompt_tokens.push(tid);
                    }
                }
            }

            if !text.is_empty() && text != last_text {
                last_text = text;
                let combined = if committed_text.is_empty() {
                    last_text.clone()
                } else {
                    format!("{} {}", committed_text, last_text)
                };
                if combined.len() > last_write_len {
                    if let Err(e) = write_transcript_json(&recording_dir, &combined, "whisper-realtime") {
                        eprintln!("Failed to write incremental transcript: {}", e);
                    } else {
                        last_write_len = combined.len();
                        let _ = app_handle.emit(EVENT_TRANSCRIPT_UPDATED, TranscriptUpdated {
                            recording_id: recording_id.to_string(),
                            text: combined,
                        });
                    }
                }
            }

            was_speaking = true;
        } else if was_speaking {
            if !last_text.is_empty() {
                if !committed_text.is_empty() {
                    committed_text.push(' ');
                }
                committed_text.push_str(&last_text);
                last_text.clear();
                prompt_tokens.clear();
            }
            was_speaking = false;
        }

        if committed_text.len() > last_write_len {
            if let Err(e) = write_transcript_json(&recording_dir, &committed_text, "whisper-realtime") {
                eprintln!("Failed to write transcript: {}", e);
            } else {
                last_write_len = committed_text.len();
                let _ = app_handle.emit(EVENT_TRANSCRIPT_UPDATED, TranscriptUpdated {
                    recording_id: recording_id.to_string(),
                    text: committed_text.clone(),
                });
            }
        }

        sleep_remaining(step_start, step_interval, &should_stop);
    }

    if !last_text.is_empty() {
        if !committed_text.is_empty() {
            committed_text.push(' ');
        }
        committed_text.push_str(&last_text);
    }

    if !committed_text.is_empty() {
        if let Err(e) = write_transcript_json(&recording_dir, &committed_text, "whisper-realtime") {
            eprintln!("Failed to write final transcript: {}", e);
        } else {
            let _ = app_handle.emit(EVENT_TRANSCRIPT_UPDATED, TranscriptUpdated {
                recording_id: recording_id.to_string(),
                text: committed_text,
            });
        }
    }

    Ok(())
}

fn write_transcript_json(recording_dir: &std::path::Path, text: &str, model: &str) -> Result<(), String> {
    use crate::transcript_migration::TranscriptSource;
    
    let transcript = crate::transcription::TranscriptJson::new(
        TranscriptSource::Local,
        model.to_string(),
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        read_metadata_from_dir(recording_dir)
            .and_then(|m| m.audio.mix.map(|a| a.duration_sec))
            .unwrap_or(0.0),
        Some("auto".to_string()),
        Some(text.to_string()),
    );

    let json_str = serde_json::to_string_pretty(&transcript)
        .map_err(|e| format!("Failed to serialize transcript JSON: {}", e))?;
    let json_path = recording_dir.join("transcript.json");
    let temp_path = json_path.with_extension("json.tmp");
    std::fs::write(&temp_path, &json_str)
        .map_err(|e| format!("Failed to write transcript: {}", e))?;
    std::fs::rename(&temp_path, &json_path)
        .map_err(|e| format!("Failed to finalize transcript: {}", e))?;
    
    Ok(())
}

fn read_metadata_from_dir(dir: &std::path::Path) -> Option<crate::storage::RecordingMetadata> {
    let path = dir.join("metadata.json");
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum_sq / samples.len() as f64).sqrt() as f32
}

fn sleep_remaining(
    start: std::time::Instant,
    interval: std::time::Duration,
    should_stop: &AtomicBool,
) {
    let elapsed = start.elapsed();
    if elapsed >= interval {
        return;
    }
    let remaining = interval - elapsed;
    let check = std::time::Duration::from_millis(100);
    let mut slept = std::time::Duration::ZERO;
    while slept < remaining && !should_stop.load(Ordering::Relaxed) {
        let chunk = check.min(remaining - slept);
        std::thread::sleep(chunk);
        slept += chunk;
    }
}

// ---------------------------------------------------------------------------
// Managed state and Tauri commands
// ---------------------------------------------------------------------------

/// Holds either a cloud or local transcriber, abstracts stop().
pub enum ActiveTranscriber {
    Cloud(CloudTranscriber),
    Local(LocalTranscriber),
}

impl ActiveTranscriber {
    fn stop(&mut self) {
        match self {
            Self::Cloud(c) => c.stop(),
            Self::Local(l) => l.stop(),
        }
    }
}

/// Tauri-managed state for the active real-time transcription session.
pub struct RealtimeTranscriptionState {
    pub active: Mutex<Option<ActiveTranscriber>>,
    pub recording_id: Mutex<Option<String>>,
}

impl RealtimeTranscriptionState {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(None),
            recording_id: Mutex::new(None),
        }
    }
}

/// Build a model path for local (Whisper) realtime transcription.
/// Prefers `realtime_model` setting; falls back to `whisper_model`.
fn resolve_local_model_path(settings: &crate::config::AppSettings) -> Result<PathBuf, String> {
    use crate::config::WhisperModelSize;

    let model_size: Option<WhisperModelSize> = settings
        .transcription
        .realtime_model
        .as_deref()
        .and_then(|name| match name.to_lowercase().as_str() {
            "tiny" => Some(WhisperModelSize::Tiny),
            "base" => Some(WhisperModelSize::Base),
            "small" => Some(WhisperModelSize::Small),
            "medium" => Some(WhisperModelSize::Medium),
            "large" | "large-v3" => Some(WhisperModelSize::Large),
            _ => None,
        })
        .or_else(|| settings.transcription.whisper_model.clone());

    let size = model_size.ok_or("No Whisper model configured for real-time transcription")?;
    let url = crate::transcription::get_model_url(&size);
    let filename = url.split('/').last().unwrap();
    let path = crate::config::get_models_dir().join(filename);
    if !path.exists() {
        return Err(format!("Whisper model not downloaded: {}", path.display()));
    }
    Ok(path)
}

/// Start a transcriber according to the current settings.
fn do_start(
    app_handle: tauri::AppHandle,
    settings: &crate::config::AppSettings,
) -> Result<ActiveTranscriber, String> {
    match settings.transcription.realtime_provider {
        crate::config::RealtimeTranscriptionProvider::OpenAI => {
            let api_key = crate::config::get_api_key_for_provider(settings, "openai")
                .ok_or("OpenAI API key not configured")?;
            let model = settings
                .transcription
                .realtime_model
                .clone()
                .unwrap_or_else(|| "gpt-4o-mini-transcribe".to_string());
            let transcriber = CloudTranscriber::start(app_handle, api_key, model, None)?;
            Ok(ActiveTranscriber::Cloud(transcriber))
        }
        crate::config::RealtimeTranscriptionProvider::Local
        | crate::config::RealtimeTranscriptionProvider::Unknown => {
            let model_path = resolve_local_model_path(settings)?;
            let transcriber = LocalTranscriber::start(app_handle, model_path)?;
            Ok(ActiveTranscriber::Local(transcriber))
        }
    }
}

/// Auto-start real-time transcription if enabled in settings.
/// Called from the recording lifecycle when a new recording begins.
/// Failures are logged as warnings and do not abort recording.
pub fn auto_start(app_handle: &tauri::AppHandle, state: &RealtimeTranscriptionState) {
    let settings = crate::config::load_settings();
    if !settings.transcription.realtime_enabled {
        return;
    }
    match do_start(app_handle.clone(), &settings) {
        Ok(transcriber) => {
            if let Ok(mut guard) = state.active.lock() {
                *guard = Some(transcriber);
            }
        }
        Err(e) => eprintln!("WARNING: Failed to auto-start real-time transcription: {}", e),
    }
}

/// Stop any active real-time transcription session.
/// Called from the recording lifecycle when a recording stops.
pub fn auto_stop(state: &RealtimeTranscriptionState) {
    if let Ok(mut guard) = state.active.lock() {
        if let Some(mut t) = guard.take() {
            t.stop();
        }
    }
}

/// Start real-time transcription for the given recording.
/// Checks config for `realtime_enabled` and picks the configured provider/model.
#[tauri::command]
pub fn start_realtime_transcription(
    app_handle: tauri::AppHandle,
    recording_id: String,
    state: tauri::State<'_, RealtimeTranscriptionState>,
) -> Result<(), String> {
    if recording_id.is_empty() {
        return Err("recording_id must not be empty".to_string());
    }

    let settings = crate::config::load_settings();
    if !settings.transcription.realtime_enabled {
        return Err("Real-time transcription is disabled in settings".to_string());
    }

    let mut guard = state.active.lock().map_err(|e| e.to_string())?;
    if let Some(mut t) = guard.take() {
        t.stop();
    }

    let transcriber = do_start(app_handle, &settings)?;
    *guard = Some(transcriber);
    drop(guard);

    let mut rid_guard = state.recording_id.lock().map_err(|e| e.to_string())?;
    *rid_guard = Some(recording_id);
    Ok(())
}

/// Stop the active real-time transcription session, if any.
#[tauri::command]
pub fn stop_realtime_transcription(
    state: tauri::State<'_, RealtimeTranscriptionState>,
) -> Result<(), String> {
    let mut guard = state.active.lock().map_err(|e| e.to_string())?;
    if let Some(mut t) = guard.take() {
        t.stop();
    }
    Ok(())
}
