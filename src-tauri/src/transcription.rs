use serde::{Deserialize, Serialize};
use crate::config::{WhisperModelSize, TranscriptionProvider, get_models_dir, load_settings};
use crate::storage::{get_data_dir, read_metadata};
use crate::cloud_ai;
use crate::transcript_migration::{TranscriptMetadata, TranscriptSource};
use tauri::Emitter;
use tauri_plugin_shell::ShellExt;

const BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelInfo {
    pub size: WhisperModelSize,
    pub filename: String,
    pub url: String,
    pub size_mb: Option<u64>,
    pub exact_bytes: Option<u64>,
    pub downloaded: bool,
    pub path: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct FluidAudioSegment {
    #[serde(rename = "speakerId")]
    speaker_id: String,
    #[serde(rename = "startTime")]
    start_time: f64,
    #[serde(rename = "endTime")]
    end_time: f64,
    text: String,
}

#[derive(Deserialize, Debug)]
struct FluidAudioOutput {
    #[serde(rename = "speakerCount")]
    speaker_count: usize,
    model: String,
    segments: Vec<FluidAudioSegment>,
}

/// JSON transcript segment (for FluidAudio diarized output)
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TranscriptSegment {
    speaker_id: String,
    start_time: f64,
    end_time: f64,
    text: String,
}

/// JSON transcript stored as transcript.json (source of truth)
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct TranscriptJson {
    source: TranscriptSource,
    model: String,
    created_at: String,
    duration_sec: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    speaker_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    segments: Option<Vec<TranscriptSegment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

pub fn get_model_url(size: &WhisperModelSize) -> String {
    let filename = match size {
        WhisperModelSize::Tiny => "ggml-tiny.bin",
        WhisperModelSize::Base => "ggml-base.bin",
        WhisperModelSize::Small => "ggml-small.bin",
        WhisperModelSize::Medium => "ggml-medium.bin",
        WhisperModelSize::Large => "ggml-large-v3.bin",
    };
    format!("{}/{}", BASE_URL, filename)
}

#[tauri::command]
pub async fn get_whisper_models_info() -> Result<Vec<ModelInfo>, String> {
    let models = vec![
        WhisperModelSize::Tiny,
        WhisperModelSize::Base,
        WhisperModelSize::Small,
        WhisperModelSize::Medium,
        WhisperModelSize::Large,
    ];

    let mut results = Vec::new();
    let models_dir = get_models_dir();
    
    if !models_dir.exists() {
        std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    }

    for size in models {
        let url = get_model_url(&size);
        let filename = url.split('/').last().unwrap().to_string();
        let local_path = models_dir.join(&filename);
        let downloaded = local_path.exists();
        let path_str = local_path.to_string_lossy().to_string();

        let size_mb = match size {
            WhisperModelSize::Tiny => 74,
            WhisperModelSize::Base => 141,
            WhisperModelSize::Small => 465,
            WhisperModelSize::Medium => 1462,
            WhisperModelSize::Large => 2951, 
        };

        results.push(ModelInfo {
            size,
            filename,
            url,
            size_mb: Some(size_mb),
            exact_bytes: None,
            downloaded,
            path: path_str,
        });
    }

    Ok(results)
}

#[derive(Clone, Serialize)]
struct DownloadProgress {
    size: WhisperModelSize,
    downloaded: u64,
    total: u64,
    percent: f64,
}

#[derive(Clone, Serialize)]
struct TranscriptionProgress {
    recording_id: String,
    stage: String,
    percent: u32,
}

#[tauri::command]
pub async fn download_whisper_model(
    app_handle: tauri::AppHandle,
    size: WhisperModelSize,
) -> Result<String, String> {
    use tokio::io::AsyncWriteExt;
    use futures_util::StreamExt;

    let url = get_model_url(&size);
    let models_dir = get_models_dir();
    
    if !models_dir.exists() {
        std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    }
    
    let filename = url.split('/').last().unwrap();
    let file_path = models_dir.join(filename);
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;
        
    let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let total_size = res.content_length().unwrap_or(0);
    
    let mut file = tokio::fs::File::create(&file_path).await.map_err(|e| e.to_string())?;
    let mut stream = res.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_emit = std::time::Instant::now();

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        if last_emit.elapsed().as_millis() > 100 || downloaded == total_size {
            let percent = if total_size > 0 { (downloaded as f64 / total_size as f64) * 100.0 } else { 0.0 };
            let _ = app_handle.emit("download_progress", DownloadProgress {
                size: size.clone(), downloaded, total: total_size, percent,
            });
            last_emit = std::time::Instant::now();
        }
    }
    
    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn delete_whisper_model(size: WhisperModelSize) -> Result<(), String> {
    let url = get_model_url(&size);
    let models_dir = get_models_dir();
    let filename = url.split('/').last().unwrap();
    let file_path = models_dir.join(filename);
    if file_path.exists() {
        std::fs::remove_file(file_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn transcribe_recording(
    app_handle: tauri::AppHandle,
    recording_id: String
) -> Result<String, String> {
    let settings = load_settings();
    if !settings.transcription.enabled {
        return Err("Transcription is disabled in settings".to_string());
    }

    let recording_dir = get_data_dir().join(&recording_id);
    let mut audio_path = recording_dir.join("audio_mix.ogg");
    if !audio_path.exists() {
        audio_path = recording_dir.join("raw_mic.ogg");
    }

    if !audio_path.exists() {
        return Err("Audio file not found".to_string());
    }

    let provider = settings.transcription.provider.clone();
    let whisper_model_ref = settings.transcription.whisper_model.clone();

    // Shared metadata fields
    let source = match provider {
        TranscriptionProvider::LocalWhisper => TranscriptSource::Local,
        TranscriptionProvider::FluidAudio => TranscriptSource::Fluidaudio,
        TranscriptionProvider::OpenAI => TranscriptSource::Openai,
        TranscriptionProvider::Google => TranscriptSource::Google,
        TranscriptionProvider::Anthropic => TranscriptSource::Anthropic,
    };

    let transcript_json = match provider {
        TranscriptionProvider::LocalWhisper => {
            let model_size = settings.transcription.whisper_model.ok_or("No whisper model selected")?;
            let url = get_model_url(&model_size);
            let filename = url.split('/').last().unwrap();
            let model_path = get_models_dir().join(filename);

            if !model_path.exists() {
                return Err(format!("Model not downloaded: {:?}", model_size));
            }

            let wav_path = recording_dir.join("temp_transcription.wav");
            convert_ogg_to_wav(&audio_path, &wav_path)?;

            let model_p = model_path.clone();
            let wav_p = wav_path.clone();

            let transcript = tokio::task::spawn_blocking(move || {
                run_whisper_transcription(&model_p, &wav_p)
            }).await.map_err(|e| e.to_string())??;

            let _ = std::fs::remove_file(&wav_path);

            let model_name = format!("whisper-{}", whisper_model_ref
                .map(|m| format!("{:?}", m).to_lowercase())
                .unwrap_or_else(|| "unknown".to_string()));
            let duration_sec = read_metadata(&recording_id)
                .ok()
                .and_then(|m| m.audio.mix.as_ref().map(|a| a.duration_sec))
                .unwrap_or(0.0);

            TranscriptJson {
                source,
                model: model_name,
                created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                duration_sec,
                language: Some("auto".to_string()),
                speaker_count: None,
                segments: None,
                text: Some(transcript),
            }
        },
        TranscriptionProvider::FluidAudio => {
            let wav_path = recording_dir.join("temp_transcription.wav");
            convert_ogg_to_wav(&audio_path, &wav_path)?;

            // Emit initial progress
            let _ = app_handle.emit("transcription_progress", TranscriptionProgress {
                recording_id: recording_id.clone(),
                stage: "Starting".to_string(),
                percent: 0,
            });

            let (mut rx, _child) = app_handle.shell().sidecar("fluidaudio-sidecar")
                .map_err(|e| format!("Failed to create sidecar command: {}", e))?
                .arg(wav_path.to_str().ok_or("Invalid WAV path")?)
                .spawn()
                .map_err(|e| format!("Failed to spawn FluidAudio sidecar: {}", e))?;

            let mut stdout_buf = Vec::new();
            let mut stderr_buf = String::new();
            let mut exit_code: Option<i32> = None;

            let timeout_duration = std::time::Duration::from_secs(600);
            let start = std::time::Instant::now();

            while let Some(event) = rx.recv().await {
                if start.elapsed() > timeout_duration {
                    return Err("FluidAudio sidecar timed out after 10 minutes".to_string());
                }

                use tauri_plugin_shell::process::CommandEvent;
                match event {
                    CommandEvent::Stdout(data) => {
                        stdout_buf.extend_from_slice(&data);
                    }
                    CommandEvent::Stderr(data) => {
                        let line = String::from_utf8_lossy(&data);
                        stderr_buf.push_str(&line);

                        // Parse progress updates: "PROGRESS:stage:percent"
                        for l in line.lines() {
                            if let Some(rest) = l.strip_prefix("PROGRESS:") {
                                let parts: Vec<&str> = rest.splitn(2, ':').collect();
                                if parts.len() == 2 {
                                    if let Ok(pct) = parts[1].parse::<u32>() {
                                        let _ = app_handle.emit("transcription_progress", TranscriptionProgress {
                                            recording_id: recording_id.clone(),
                                            stage: parts[0].to_string(),
                                            percent: pct,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    CommandEvent::Terminated(payload) => {
                        exit_code = payload.code;
                        break;
                    }
                    _ => {}
                }
            }

            let _ = std::fs::remove_file(&wav_path);

            // Print debug output from sidecar (all non-PROGRESS lines)
            for line in stderr_buf.lines() {
                if !line.starts_with("PROGRESS:") && !line.is_empty() {
                    eprintln!("{}", line);
                }
            }

            if exit_code != Some(0) {
                return Err(format!("FluidAudio sidecar failed: {}", stderr_buf));
            }

            let stdout = String::from_utf8_lossy(&stdout_buf);
            let fa_output: FluidAudioOutput = serde_json::from_str(&stdout)
                .map_err(|e| format!("Failed to parse FluidAudio output: {}", e))?;

            let segments: Vec<TranscriptSegment> = fa_output.segments.iter().map(|seg| {
                TranscriptSegment {
                    speaker_id: seg.speaker_id.clone(),
                    start_time: seg.start_time,
                    end_time: seg.end_time,
                    text: seg.text.trim().to_string(),
                }
            }).collect();

            let duration_sec = read_metadata(&recording_id)
                .ok()
                .and_then(|m| m.audio.mix.as_ref().map(|a| a.duration_sec))
                .unwrap_or(0.0);

            TranscriptJson {
                source,
                model: fa_output.model,
                created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                duration_sec,
                language: Some("auto".to_string()),
                speaker_count: Some(fa_output.speaker_count),
                segments: Some(segments),
                text: None,
            }
        },
        TranscriptionProvider::OpenAI => {
            let api_key = settings.transcription.api_keys.openai
                .ok_or("OpenAI API key not configured")?;

            let transcript = cloud_ai::transcribe_with_whisper(&api_key, &audio_path).await?;
            let duration_sec = read_metadata(&recording_id)
                .ok()
                .and_then(|m| m.audio.mix.as_ref().map(|a| a.duration_sec))
                .unwrap_or(0.0);

            TranscriptJson {
                source,
                model: "whisper-1".to_string(),
                created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                duration_sec,
                language: Some("auto".to_string()),
                speaker_count: None,
                segments: None,
                text: Some(transcript),
            }
        },
        TranscriptionProvider::Google => {
            return Err("Google provider requires a transcript first. Use Local Whisper or OpenAI for transcription, then use Google for summarization.".to_string());
        },
        TranscriptionProvider::Anthropic => {
            return Err("Anthropic provider doesn't support audio transcription. Use Local Whisper or OpenAI for transcription, then use Anthropic for structured extraction.".to_string());
        },
    };

    // Save raw JSON as source of truth
    let json_str = serde_json::to_string_pretty(&transcript_json)
        .map_err(|e| format!("Failed to serialize transcript JSON: {}", e))?;
    let json_path = recording_dir.join("transcript.json");
    let temp_path = json_path.with_extension("json.tmp");
    std::fs::write(&temp_path, &json_str)
        .map_err(|e| format!("Failed to write transcript: {}", e))?;
    std::fs::rename(&temp_path, &json_path)
        .map_err(|e| format!("Failed to finalize transcript: {}", e))?;

    // Return rendered text for immediate UI display
    Ok(render_transcript_from_json(&transcript_json))
}

/// Render compact text from a TranscriptJson struct
pub(crate) fn render_transcript_from_json(tj: &TranscriptJson) -> String {
    if let Some(segments) = &tj.segments {
        segments.iter().map(|seg| {
            let short_id = seg.speaker_id.replace("Speaker ", "SP");
            format!("{}: {}", short_id, seg.text)
        }).collect::<Vec<_>>().join("\n")
    } else if let Some(text) = &tj.text {
        text.clone()
    } else {
        String::new()
    }
}

/// Render transcript text on the fly from transcript.json (with .md fallback)
fn render_transcript_text(recording_id: &str) -> Option<String> {
    let recording_dir = get_data_dir().join(recording_id);
    let json_path = recording_dir.join("transcript.json");

    if json_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&json_path) {
            if let Ok(tj) = serde_json::from_str::<TranscriptJson>(&content) {
                return Some(render_transcript_from_json(&tj));
            }
        }
    }

    // Fallback: legacy transcript.md
    let md_path = recording_dir.join("transcript.md");
    if md_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&md_path) {
            if content.starts_with("---") {
                let parts: Vec<&str> = content.splitn(3, "---").collect();
                if parts.len() >= 3 {
                    return Some(parts[2].trim().to_string());
                }
            }
            return Some(content);
        }
    }

    None
}

/// Read transcript body for internal consumers (summarize, templates, etc.)
fn read_transcript_body(recording_id: &str) -> Result<String, String> {
    render_transcript_text(recording_id)
        .ok_or_else(|| "No transcript found. Please transcribe the recording first.".to_string())
}

/// Summarize a recording's transcript using the configured AI provider
#[tauri::command]
pub async fn summarize_recording(
    recording_id: String,
    provider: Option<String>,
) -> Result<String, String> {
    let settings = load_settings();
    let recording_dir = get_data_dir().join(&recording_id);

    let transcript = read_transcript_body(&recording_id)?;

    // Determine which provider to use
    let use_provider = provider
        .map(|p| match p.as_str() {
            "OpenAI" => TranscriptionProvider::OpenAI,
            "Google" => TranscriptionProvider::Google,
            "Anthropic" => TranscriptionProvider::Anthropic,
            _ => settings.transcription.provider.clone(),
        })
        .unwrap_or(settings.transcription.provider.clone());

    let summary = match use_provider {
        TranscriptionProvider::OpenAI => {
            let api_key = settings.transcription.api_keys.openai
                .ok_or("OpenAI API key not configured")?;
            cloud_ai::summarize_with_gpt4o(&api_key, &transcript, None).await?
        },
        TranscriptionProvider::Google => {
            let api_key = settings.transcription.api_keys.google
                .ok_or("Google API key not configured")?;
            cloud_ai::summarize_with_gemini(&api_key, &transcript).await?
        },
        TranscriptionProvider::Anthropic => {
            let api_key = settings.transcription.api_keys.anthropic
                .ok_or("Anthropic API key not configured")?;
            cloud_ai::process_with_claude(&api_key,
                "Create a comprehensive summary of this transcript. Include main topics, key points, decisions, and action items.\n\nTranscript:\n{transcript}",
                &transcript
            ).await?
        },
        TranscriptionProvider::LocalWhisper | TranscriptionProvider::FluidAudio => {
            return Err("Local transcription providers cannot generate summaries. Please configure a cloud AI provider (OpenAI, Google, or Anthropic).".to_string());
        },
    };

    // Save summary
    std::fs::write(recording_dir.join("summary.md"), &summary)
        .map_err(|e| format!("Failed to save summary: {}", e))?;

    Ok(summary)
}

/// Process a transcript with a specific template
#[tauri::command]
pub async fn process_with_template(
    recording_id: String,
    template_name: String,
    provider: Option<String>,
) -> Result<String, String> {
    let settings = load_settings();
    let recording_dir = get_data_dir().join(&recording_id);

    let transcript = read_transcript_body(&recording_id)?;

    // Load template
    let template = crate::templates::get_template_internal(&template_name)?;

    // Determine provider
    let use_provider = provider
        .map(|p| match p.as_str() {
            "OpenAI" => TranscriptionProvider::OpenAI,
            "Google" => TranscriptionProvider::Google,
            "Anthropic" => TranscriptionProvider::Anthropic,
            _ => settings.transcription.provider.clone(),
        })
        .unwrap_or(settings.transcription.provider.clone());

    let result = match use_provider {
        TranscriptionProvider::OpenAI => {
            let api_key = settings.transcription.api_keys.openai
                .ok_or("OpenAI API key not configured")?;
            cloud_ai::process_with_gpt4o(&api_key, &template.prompt, &transcript).await?
        },
        TranscriptionProvider::Google => {
            let api_key = settings.transcription.api_keys.google
                .ok_or("Google API key not configured")?;
            cloud_ai::process_with_gemini(&api_key, &template.prompt, &transcript).await?
        },
        TranscriptionProvider::Anthropic => {
            let api_key = settings.transcription.api_keys.anthropic
                .ok_or("Anthropic API key not configured")?;
            cloud_ai::process_with_claude(&api_key, &template.prompt, &transcript).await?
        },
        TranscriptionProvider::LocalWhisper | TranscriptionProvider::FluidAudio => {
            return Err("Local transcription providers cannot process templates. Please configure a cloud AI provider.".to_string());
        },
    };

    // Save result based on output format
    let filename = match template.output_format.as_str() {
        "json" => format!("{}.json", template_name),
        _ => format!("{}.md", template_name),
    };
    std::fs::write(recording_dir.join(&filename), &result)
        .map_err(|e| format!("Failed to save result: {}", e))?;

    Ok(result)
}

#[tauri::command]
pub async fn get_transcript(recording_id: String) -> Result<Option<String>, String> {
    Ok(render_transcript_text(&recording_id))
}

/// Export transcript as markdown with frontmatter (for Save button)
#[tauri::command]
pub async fn export_transcript_md(
    app_handle: tauri::AppHandle,
    recording_id: String,
) -> Result<(), String> {
    let recording_dir = get_data_dir().join(&recording_id);
    let json_path = recording_dir.join("transcript.json");

    // Build markdown content
    let md_content = if json_path.exists() {
        let content = std::fs::read_to_string(&json_path)
            .map_err(|e| format!("Failed to read transcript: {}", e))?;
        let tj: TranscriptJson = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse transcript: {}", e))?;

        let body = render_transcript_from_json(&tj);
        let metadata = TranscriptMetadata {
            source: tj.source,
            model: tj.model,
            created_at: tj.created_at,
            duration_sec: tj.duration_sec,
            language: tj.language,
            segments_count: tj.segments.as_ref().map(|s| s.len()),
            speaker_count: tj.speaker_count,
        };
        let frontmatter = serde_yaml::to_string(&metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        format!("---\n{}---\n\n{}", frontmatter, body)
    } else {
        // Fallback: legacy .md
        let md_path = recording_dir.join("transcript.md");
        if md_path.exists() {
            std::fs::read_to_string(&md_path)
                .map_err(|e| format!("Failed to read transcript: {}", e))?
        } else {
            return Err("No transcript found".to_string());
        }
    };

    // Use Tauri save dialog (async via oneshot channel to avoid blocking tokio worker)
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app_handle.dialog()
        .file()
        .add_filter("Markdown", &["md"])
        .set_file_name("transcript.md")
        .save_file(move |file_path| {
            let _ = tx.send(file_path);
        });

    let file_path = rx.await.map_err(|_| "Dialog channel closed unexpectedly")?;

    if let Some(path) = file_path {
        let dest = path.as_path()
            .ok_or("Invalid file path selected")?;
        std::fs::write(dest, &md_content)
            .map_err(|e| format!("Failed to save file: {}", e))?;
    }

    Ok(())
}

fn convert_ogg_to_wav(ogg_path: &std::path::Path, wav_path: &std::path::Path) -> Result<(), String> {
    use lewton::inside_ogg::OggStreamReader;
    use hound::{WavWriter, WavSpec};
    use rubato::{SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction, Resampler};
    use std::fs::File;

    const TARGET_RATE: u32 = 16000;

    let ogg_file = File::open(ogg_path).map_err(|e| e.to_string())?;
    let mut ogg_reader = OggStreamReader::new(ogg_file).map_err(|e| e.to_string())?;

    let src_rate = ogg_reader.ident_hdr.audio_sample_rate;
    let src_channels = ogg_reader.ident_hdr.audio_channels as u16;

    let spec = WavSpec {
        channels: 1,
        sample_rate: TARGET_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut wav_writer = WavWriter::create(wav_path, spec).map_err(|e| e.to_string())?;

    // Collect all decoded mono samples as f32
    let mut all_mono: Vec<f32> = Vec::new();
    while let Some(packet) = ogg_reader.read_dec_packet_generic::<Vec<Vec<i16>>>().map_err(|e| e.to_string())? {
        if packet.is_empty() { continue; }
        let frames = packet[0].len();
        for i in 0..frames {
            let mut sum: i32 = 0;
            for ch in 0..src_channels as usize {
                if ch < packet.len() && i < packet[ch].len() {
                    sum += packet[ch][i] as i32;
                }
            }
            all_mono.push((sum / src_channels as i32) as f32 / 32768.0);
        }
    }

    if src_rate == TARGET_RATE {
        // No resampling needed
        for s in &all_mono {
            wav_writer.write_sample((*s * 32768.0).clamp(-32768.0, 32767.0) as i16)
                .map_err(|e| e.to_string())?;
        }
    } else {
        // High-quality sinc interpolation resampling (matches mic pipeline approach)
        let chunk_size = 1024usize;
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };
        let resample_ratio = TARGET_RATE as f64 / src_rate as f64;
        let mut resampler = SincFixedIn::<f32>::new(
            resample_ratio,
            2.0,
            params,
            chunk_size,
            1, // mono
        ).map_err(|e| format!("Failed to create resampler: {}", e))?;

        let mut pos = 0;
        while pos + chunk_size <= all_mono.len() {
            let chunk = vec![&all_mono[pos..pos + chunk_size]];
            let output = resampler.process(&chunk, None)
                .map_err(|e| format!("Resampling error: {}", e))?;
            for s in &output[0] {
                wav_writer.write_sample((*s * 32768.0).clamp(-32768.0, 32767.0) as i16)
                    .map_err(|e| e.to_string())?;
            }
            pos += chunk_size;
        }

        // Process remaining frames
        if pos < all_mono.len() {
            let chunk = vec![&all_mono[pos..]];
            let output = resampler.process_partial(Some(&chunk), None)
                .map_err(|e| format!("Resampling error: {}", e))?;
            for s in &output[0] {
                wav_writer.write_sample((*s * 32768.0).clamp(-32768.0, 32767.0) as i16)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

fn run_whisper_transcription(model_path: &std::path::Path, wav_path: &std::path::Path) -> Result<String, String> {
    use whisper_rs::{WhisperContext, FullParams, SamplingStrategy, WhisperContextParameters};
    use hound::WavReader;
    
    let ctx = WhisperContext::new_with_params(
        model_path.to_str().unwrap(), 
        WhisperContextParameters::default()
    ).map_err(|e| e.to_string())?;
    
    let mut wav_reader = WavReader::open(wav_path).map_err(|e| e.to_string())?;
    let samples: Vec<f32> = wav_reader.samples::<i16>().map(|s| s.unwrap() as f32 / 32768.0).collect();
    
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(None);
    params.set_translate(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    
    let mut state = ctx.create_state().map_err(|e| e.to_string())?;
    state.full(params, &samples).map_err(|e| e.to_string())?;
    
    let mut transcript = String::new();
    for i in 0..state.full_n_segments().unwrap_or(0) {
        if let Ok(text) = state.full_get_segment_text(i) {
            transcript.push_str(&text);
            transcript.push(' ');
        }
    }
    
    Ok(transcript.trim().to_string())
}
