use serde::{Deserialize, Serialize};
use crate::config::{WhisperModelSize, TranscriptionProvider, get_models_dir, load_settings};
use crate::storage::get_data_dir;
use crate::cloud_ai;
use tauri::Emitter;

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
    _app_handle: tauri::AppHandle,
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

    let result = match settings.transcription.provider {
        TranscriptionProvider::LocalWhisper => {
            // Local Whisper transcription
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
            transcript
        },
        TranscriptionProvider::OpenAI => {
            // OpenAI Whisper-1 API
            let api_key = settings.transcription.api_keys.openai
                .ok_or("OpenAI API key not configured")?;

            cloud_ai::transcribe_with_whisper(&api_key, &audio_path).await?
        },
        TranscriptionProvider::Google => {
            // Google doesn't do transcription directly, but Gemini can process audio
            // For now, we need a transcript first, so fall back to error
            return Err("Google provider requires a transcript first. Use Local Whisper or OpenAI for transcription, then use Google for summarization.".to_string());
        },
        TranscriptionProvider::Anthropic => {
            // Anthropic doesn't do audio transcription
            return Err("Anthropic provider doesn't support audio transcription. Use Local Whisper or OpenAI for transcription, then use Anthropic for structured extraction.".to_string());
        },
    };

    std::fs::write(recording_dir.join("transcript.md"), &result).map_err(|e| e.to_string())?;

    Ok(result)
}

/// Summarize a recording's transcript using the configured AI provider
#[tauri::command]
pub async fn summarize_recording(
    recording_id: String,
    provider: Option<String>,
) -> Result<String, String> {
    let settings = load_settings();
    let recording_dir = get_data_dir().join(&recording_id);
    let transcript_path = recording_dir.join("transcript.md");

    if !transcript_path.exists() {
        return Err("No transcript found. Please transcribe the recording first.".to_string());
    }

    let transcript = std::fs::read_to_string(&transcript_path)
        .map_err(|e| format!("Failed to read transcript: {}", e))?;

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
        TranscriptionProvider::LocalWhisper => {
            return Err("Local Whisper cannot generate summaries. Please configure a cloud AI provider (OpenAI, Google, or Anthropic).".to_string());
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
    let transcript_path = recording_dir.join("transcript.md");

    if !transcript_path.exists() {
        return Err("No transcript found. Please transcribe the recording first.".to_string());
    }

    let transcript = std::fs::read_to_string(&transcript_path)
        .map_err(|e| format!("Failed to read transcript: {}", e))?;

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
        TranscriptionProvider::LocalWhisper => {
            return Err("Local Whisper cannot process templates. Please configure a cloud AI provider.".to_string());
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
    let recording_dir = get_data_dir().join(&recording_id);
    let transcript_path = recording_dir.join("transcript.md");
    
    if transcript_path.exists() {
        let content = std::fs::read_to_string(transcript_path).map_err(|e| e.to_string())?;
        Ok(Some(content))
    } else {
        Ok(None)
    }
}

fn convert_ogg_to_wav(ogg_path: &std::path::Path, wav_path: &std::path::Path) -> Result<(), String> {
    use lewton::inside_ogg::OggStreamReader;
    use hound::{WavWriter, WavSpec};
    use std::fs::File;
    
    let ogg_file = File::open(ogg_path).map_err(|e| e.to_string())?;
    let mut ogg_reader = OggStreamReader::new(ogg_file).map_err(|e| e.to_string())?;
    
    let src_rate = ogg_reader.ident_hdr.audio_sample_rate;
    let src_channels = ogg_reader.ident_hdr.audio_channels as u16;
    
    let spec = WavSpec {
        channels: 1, 
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let mut wav_writer = WavWriter::create(wav_path, spec).map_err(|e| e.to_string())?;
    
    while let Some(packet) = ogg_reader.read_dec_packet_generic::<Vec<Vec<i16>>>().map_err(|e| e.to_string())? {
        if packet.is_empty() { continue; }
        
        let frames = packet[0].len();
        let mut mono = Vec::with_capacity(frames);
        for i in 0..frames {
            let mut sum: i32 = 0;
            for ch in 0..src_channels as usize {
                if ch < packet.len() && i < packet[ch].len() {
                    sum += packet[ch][i] as i32;
                }
            }
            mono.push((sum / src_channels as i32) as i16);
        }

        if src_rate == 48000 {
            for (i, s) in mono.iter().enumerate() {
                if i % 3 == 0 { wav_writer.write_sample(*s).map_err(|e| e.to_string())?; }
            }
        } else if src_rate == 16000 {
            for s in mono { wav_writer.write_sample(s).map_err(|e| e.to_string())?; }
        } else {
            return Err("Unsupported sample rate".to_string());
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
