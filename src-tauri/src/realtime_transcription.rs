use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use futures_util::{SinkExt, StreamExt};
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;
use tokio_tungstenite::tungstenite;

use crate::audio_processing::TRANSCRIPTION_BUFFER;

const OPENAI_REALTIME_URL: &str = "wss://api.openai.com/v1/realtime?intent=transcription";

/// Audio chunk interval: read from TRANSCRIPTION_BUFFER every 100ms
const AUDIO_CHUNK_INTERVAL_MS: u64 = 100;

/// Tauri event names emitted to the frontend
const EVENT_TRANSCRIPT_DELTA: &str = "realtime_transcript_delta";
const EVENT_TRANSCRIPTION_ERROR: &str = "realtime_transcription_error";

/// Payload emitted to the frontend for transcript updates
#[derive(Clone, Serialize)]
pub struct TranscriptDelta {
    pub text: String,
    pub is_final: bool,
    pub item_id: String,
}

/// Server event types we care about
#[derive(Deserialize)]
struct ServerEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    item_id: Option<String>,
    #[serde(default)]
    delta: Option<String>,
    #[serde(default)]
    transcript: Option<String>,
    #[serde(default)]
    error: Option<ServerError>,
}

#[derive(Deserialize)]
struct ServerError {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    code: Option<String>,
}

/// Handle to a running cloud transcription session.
/// Drop or call `stop()` to terminate the WebSocket connection and audio streaming.
pub struct CloudTranscriber {
    should_stop: Arc<AtomicBool>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl CloudTranscriber {
    /// Start a new cloud transcription session.
    ///
    /// Connects to the OpenAI Realtime API via WebSocket, configures the transcription
    /// session, and begins streaming audio from TRANSCRIPTION_BUFFER.
    ///
    /// Emits `realtime_transcript_delta` events to the frontend with partial and final transcripts.
    pub fn start(
        app_handle: tauri::AppHandle,
        api_key: String,
        model: String,
        language: Option<String>,
    ) -> Result<Self, String> {
        let should_stop = Arc::new(AtomicBool::new(false));
        let stop_flag = should_stop.clone();

        let task_handle = tokio::spawn(async move {
            if let Err(e) = run_cloud_transcription(
                app_handle.clone(),
                api_key,
                model,
                language,
                stop_flag,
            ).await {
                eprintln!("Cloud transcription error: {}", e);
                let _ = app_handle.emit(EVENT_TRANSCRIPTION_ERROR, e);
            }
        });

        Ok(Self {
            should_stop,
            task_handle: Some(task_handle),
        })
    }

    pub fn stop(&mut self) {
        self.should_stop.store(true, Ordering::Relaxed);
        // Don't abort — let the task observe the flag and shut down gracefully
        // (flushes remaining audio, sends WebSocket close frame)
    }
}

impl Drop for CloudTranscriber {
    fn drop(&mut self) {
        self.should_stop.store(true, Ordering::Relaxed);
    }
}

/// Build the `transcription_session.update` configuration event
fn build_session_update(model: &str, language: Option<&str>) -> String {
    let mut transcription = serde_json::json!({
        "model": model,
    });
    if let Some(lang) = language {
        transcription["language"] = serde_json::json!(lang);
    }

    let event = serde_json::json!({
        "type": "transcription_session.update",
        "input_audio_format": "pcm16",
        "input_audio_transcription": transcription,
        "turn_detection": {
            "type": "server_vad",
            "threshold": 0.5,
            "prefix_padding_ms": 300,
            "silence_duration_ms": 500,
        },
    });
    event.to_string()
}

/// Create a resampler for 16kHz → 24kHz mono conversion
fn create_resampler() -> Result<SincFixedIn<f32>, String> {
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };
    SincFixedIn::<f32>::new(
        24000.0 / 16000.0, // ratio: output/input
        2.0,
        params,
        1024, // chunk size
        1,    // mono
    )
    .map_err(|e| format!("Failed to create resampler: {}", e))
}

/// Convert f32 samples to PCM16 (i16 little-endian bytes)
fn f32_to_pcm16_bytes(samples: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let pcm16 = (clamped * 32767.0) as i16;
        bytes.extend_from_slice(&pcm16.to_le_bytes());
    }
    bytes
}

/// Main loop: connect, configure, stream audio, receive transcriptions
async fn run_cloud_transcription(
    app_handle: tauri::AppHandle,
    api_key: String,
    model: String,
    language: Option<String>,
    should_stop: Arc<AtomicBool>,
) -> Result<(), String> {
    // Build WebSocket request with auth header (upgrade headers added by tungstenite)
    let request = tungstenite::http::Request::builder()
        .uri(OPENAI_REALTIME_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .body(())
        .map_err(|e| format!("Failed to build request: {}", e))?;

    let (ws_stream, _response) = tokio_tungstenite::connect_async_tls_with_config(
        request,
        None,
        false,
        None,
    )
    .await
    .map_err(|e| format!("WebSocket connection failed: {}", e))?;

    let (mut ws_write, mut ws_read) = ws_stream.split();

    // Send session configuration
    let session_update = build_session_update(&model, language.as_deref());
    ws_write
        .send(tungstenite::Message::Text(session_update.into()))
        .await
        .map_err(|e| format!("Failed to send session config: {}", e))?;

    // Create resampler: 16kHz → 24kHz
    let mut resampler = create_resampler()?;
    let chunk_size = 1024usize;
    let mut resample_accum: Vec<f32> = Vec::with_capacity(chunk_size * 2);

    // Audio streaming interval
    let mut audio_interval = tokio::time::interval(
        tokio::time::Duration::from_millis(AUDIO_CHUNK_INTERVAL_MS),
    );

    let mut loop_error: Option<String> = None;

    loop {
        if should_stop.load(Ordering::Relaxed) {
            break;
        }

        tokio::select! {
            _ = audio_interval.tick() => {
                if should_stop.load(Ordering::Relaxed) {
                    break;
                }

                // Read available samples from TRANSCRIPTION_BUFFER (16kHz mono)
                let available = TRANSCRIPTION_BUFFER.available();
                if available == 0 {
                    continue;
                }

                let samples = TRANSCRIPTION_BUFFER.pop(available);
                if samples.is_empty() {
                    continue;
                }

                // Resample 16kHz → 24kHz
                resample_accum.extend_from_slice(&samples);
                let mut resampled_all: Vec<f32> = Vec::new();

                while resample_accum.len() >= chunk_size {
                    let chunk: Vec<f32> = resample_accum.drain(..chunk_size).collect();
                    match resampler.process(&[chunk], None) {
                        Ok(output) => {
                            if !output[0].is_empty() {
                                resampled_all.extend_from_slice(&output[0]);
                            }
                        }
                        Err(e) => {
                            eprintln!("Resample error: {}", e);
                        }
                    }
                }

                if resampled_all.is_empty() {
                    continue;
                }

                // Convert to PCM16 and base64-encode
                let pcm_bytes = f32_to_pcm16_bytes(&resampled_all);
                let b64 = BASE64.encode(&pcm_bytes);

                // Send audio chunk
                let msg = serde_json::json!({
                    "type": "input_audio_buffer.append",
                    "audio": b64,
                }).to_string();

                if let Err(e) = ws_write.send(tungstenite::Message::Text(msg.into())).await {
                    loop_error = Some(format!("Failed to send audio: {}", e));
                    break;
                }
            }

            msg = ws_read.next() => {
                match msg {
                    Some(Ok(tungstenite::Message::Text(text))) => {
                        handle_server_event(&app_handle, &text);
                    }
                    Some(Ok(tungstenite::Message::Close(frame))) => {
                        let reason = frame
                            .map(|f| format!("code={}, reason={}", f.code, f.reason))
                            .unwrap_or_else(|| "no close frame".to_string());
                        loop_error = Some(format!("Server closed connection: {}", reason));
                        break;
                    }
                    Some(Err(e)) => {
                        loop_error = Some(format!("WebSocket read error: {}", e));
                        break;
                    }
                    None => {
                        loop_error = Some("WebSocket connection closed unexpectedly".to_string());
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // Flush remaining buffered audio and close gracefully (only if connection is still alive)
    if loop_error.is_none() {
        // Drain any queued samples from TRANSCRIPTION_BUFFER
        let queued = TRANSCRIPTION_BUFFER.available();
        if queued > 0 {
            resample_accum.extend_from_slice(&TRANSCRIPTION_BUFFER.pop(queued));
        }

        let mut flush_resampled: Vec<f32> = Vec::new();

        // Process full chunks through resampler
        while resample_accum.len() >= chunk_size {
            let chunk: Vec<f32> = resample_accum.drain(..chunk_size).collect();
            if let Ok(output) = resampler.process(&[chunk], None) {
                if !output[0].is_empty() {
                    flush_resampled.extend_from_slice(&output[0]);
                }
            }
        }

        // Flush partial input chunk
        let remaining = std::mem::take(&mut resample_accum);
        if !remaining.is_empty() {
            if let Ok(output) = resampler.process_partial(Some(&[remaining]), None) {
                if !output[0].is_empty() {
                    flush_resampled.extend_from_slice(&output[0]);
                }
            }
        }

        // Drain resampler's internal delay line
        if let Ok(output) = resampler.process_partial(None::<&[Vec<f32>]>, None) {
            if !output[0].is_empty() {
                flush_resampled.extend_from_slice(&output[0]);
            }
        }

        // Send any flushed audio
        if !flush_resampled.is_empty() {
            let pcm_bytes = f32_to_pcm16_bytes(&flush_resampled);
            let b64 = BASE64.encode(&pcm_bytes);
            let msg = serde_json::json!({
                "type": "input_audio_buffer.append",
                "audio": b64,
            }).to_string();
            let _ = ws_write.send(tungstenite::Message::Text(msg.into())).await;
        }

        let _ = ws_write.send(tungstenite::Message::Close(None)).await;
    }

    if let Some(e) = loop_error {
        return Err(e);
    }
    Ok(())
}

/// Parse and handle incoming server events
fn handle_server_event(app_handle: &tauri::AppHandle, text: &str) {
    let event: ServerEvent = match serde_json::from_str(text) {
        Ok(e) => e,
        Err(_) => return,
    };

    match event.event_type.as_str() {
        "conversation.item.input_audio_transcription.delta" => {
            if let Some(delta) = event.delta {
                let _ = app_handle.emit(EVENT_TRANSCRIPT_DELTA, TranscriptDelta {
                    text: delta,
                    is_final: false,
                    item_id: event.item_id.unwrap_or_default(),
                });
            }
        }
        "conversation.item.input_audio_transcription.completed" => {
            if let Some(transcript) = event.transcript {
                let _ = app_handle.emit(EVENT_TRANSCRIPT_DELTA, TranscriptDelta {
                    text: transcript,
                    is_final: true,
                    item_id: event.item_id.unwrap_or_default(),
                });
            }
        }
        "error" => {
            if let Some(err) = event.error {
                let msg = err.message.unwrap_or_else(|| "Unknown error".to_string());
                eprintln!("OpenAI Realtime API error: {} (code: {:?})", msg, err.code);
                let _ = app_handle.emit(EVENT_TRANSCRIPTION_ERROR, msg);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Local Whisper sliding-window transcriber
// ---------------------------------------------------------------------------

/// Sliding window: 5 seconds at 16 kHz
const LOCAL_WINDOW_SAMPLES: usize = 16000 * 5;
/// Step size: 1 second at 16 kHz
const LOCAL_STEP_SAMPLES: usize = 16000;
/// Energy-based VAD threshold (RMS below this is treated as silence)
const VAD_RMS_THRESHOLD: f32 = 0.005;

/// Handle to a running local Whisper transcription session.
/// Spawns a dedicated OS thread for CPU-bound inference.
pub struct LocalTranscriber {
    should_stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl LocalTranscriber {
    /// Start local real-time transcription.
    ///
    /// Loads the Whisper model at `model_path`, then continuously reads audio from
    /// `TRANSCRIPTION_BUFFER`, runs inference on a 5 s sliding window (1 s step),
    /// and emits `realtime_transcript_delta` Tauri events.
    pub fn start(
        app_handle: tauri::AppHandle,
        model_path: PathBuf,
    ) -> Result<Self, String> {
        if !model_path.exists() {
            return Err(format!("Whisper model not found: {}", model_path.display()));
        }

        let should_stop = Arc::new(AtomicBool::new(false));
        let stop_flag = should_stop.clone();

        let handle = std::thread::spawn(move || {
            if let Err(e) = run_local_transcription(app_handle.clone(), &model_path, stop_flag) {
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
    let mut segment_counter: u64 = 0;
    let mut was_speaking = false;
    let mut last_text = String::new();

    let step_interval = std::time::Duration::from_secs(1);

    // Accumulate at least one step worth of audio before first inference
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

        // Drain new samples from the shared buffer
        let available = TRANSCRIPTION_BUFFER.available();
        if available > 0 {
            window.extend_from_slice(&TRANSCRIPTION_BUFFER.pop(available));
        }

        // Slide: keep only the last WINDOW_SAMPLES
        if window.len() > LOCAL_WINDOW_SAMPLES {
            let excess = window.len() - LOCAL_WINDOW_SAMPLES;
            window.drain(..excess);
        }

        // VAD on the newest step only (last 1 s) to avoid stale speech energy
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

            // Harvest tokens from last segment for context continuity
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
                segment_counter += 1;
                let _ = app_handle.emit(
                    EVENT_TRANSCRIPT_DELTA,
                    TranscriptDelta {
                        text: text.clone(),
                        is_final: false,
                        item_id: format!("local-{}", segment_counter),
                    },
                );
                last_text = text;
            }

            was_speaking = true;
        } else if was_speaking {
            // Speech → silence transition: commit final text
            if !last_text.is_empty() {
                let _ = app_handle.emit(
                    EVENT_TRANSCRIPT_DELTA,
                    TranscriptDelta {
                        text: last_text.clone(),
                        is_final: true,
                        item_id: format!("local-{}", segment_counter),
                    },
                );
                last_text.clear();
                prompt_tokens.clear();
            }
            was_speaking = false;
        }

        sleep_remaining(step_start, step_interval, &should_stop);
    }

    // Emit final for any remaining uncommitted text
    if !last_text.is_empty() {
        let _ = app_handle.emit(
            EVENT_TRANSCRIPT_DELTA,
            TranscriptDelta {
                text: last_text,
                is_final: true,
                item_id: format!("local-{}", segment_counter),
            },
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Unified handle
// ---------------------------------------------------------------------------

/// Wraps either a local Whisper or cloud OpenAI transcriber behind a single interface.
pub enum RealtimeTranscriberHandle {
    Local(LocalTranscriber),
    Cloud(CloudTranscriber),
}

impl RealtimeTranscriberHandle {
    pub fn stop(&mut self) {
        match self {
            RealtimeTranscriberHandle::Local(t) => t.stop(),
            RealtimeTranscriberHandle::Cloud(t) => t.stop(),
        }
    }
}

/// Compute RMS energy of a sample buffer
fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum_sq / samples.len() as f64).sqrt() as f32
}

/// Sleep for the remainder of `interval` since `start`, checking the stop flag every 100 ms
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
