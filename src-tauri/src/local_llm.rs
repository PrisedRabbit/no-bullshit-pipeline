use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use crate::config::{get_llm_models_dir, load_settings};

// ── Model Registry ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LlmModelInfo {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub filename: String,
    pub url: String,
    pub size_mb: u64,
    pub params: String,
    pub downloaded: bool,
    pub path: String,
}

struct ModelDef {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    filename: &'static str,
    url: &'static str,
    size_mb: u64,
    params: &'static str,
}

const MODELS: &[ModelDef] = &[
    ModelDef {
        id: "phi-3.5-mini",
        name: "Phi-3.5 Mini",
        desc: "Fast, lightweight. English only.",
        filename: "Phi-3.5-mini-instruct-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/Phi-3.5-mini-instruct-GGUF/resolve/main/Phi-3.5-mini-instruct-Q4_K_M.gguf",
        size_mb: 2400,
        params: "3.8B",
    },
    ModelDef {
        id: "llama-3.1-8b",
        name: "Llama 3.1 8B",
        desc: "Strong all-rounder. English-first, basic multilingual.",
        filename: "Meta-Llama-3.1-8B-Instruct-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/Meta-Llama-3.1-8B-Instruct-GGUF/resolve/main/Meta-Llama-3.1-8B-Instruct-Q4_K_M.gguf",
        size_mb: 4920,
        params: "8B",
    },
    ModelDef {
        id: "mistral-7b",
        name: "Mistral 7B Instruct",
        desc: "Great for structured output. English-first.",
        filename: "Mistral-7B-Instruct-v0.3-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/Mistral-7B-Instruct-v0.3-GGUF/resolve/main/Mistral-7B-Instruct-v0.3-Q4_K_M.gguf",
        size_mb: 4370,
        params: "7B",
    },
    ModelDef {
        id: "qwen2.5-7b",
        name: "Qwen 2.5 7B",
        desc: "Best multilingual — RU, EN, ZH, 29+ languages.",
        filename: "Qwen2.5-7B-Instruct-Q4_K_M.gguf",
        url: "https://huggingface.co/bartowski/Qwen2.5-7B-Instruct-GGUF/resolve/main/Qwen2.5-7B-Instruct-Q4_K_M.gguf",
        size_mb: 4680,
        params: "7B",
    },
];

#[tauri::command]
pub fn get_llm_models_info() -> Result<Vec<LlmModelInfo>, String> {
    let models_dir = get_llm_models_dir();
    if !models_dir.exists() {
        std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    }

    Ok(MODELS
        .iter()
        .map(|m| {
            let local_path = models_dir.join(m.filename);
            let downloaded = local_path.exists();
            LlmModelInfo {
                id: m.id.to_string(),
                name: m.name.to_string(),
                desc: m.desc.to_string(),
                filename: m.filename.to_string(),
                url: m.url.to_string(),
                size_mb: m.size_mb,
                params: m.params.to_string(),
                downloaded,
                path: local_path.to_string_lossy().to_string(),
            }
        })
        .collect())
}

// ── Download / Delete ───────────────────────────────────────────────────────

#[derive(Clone, Serialize)]
struct LlmDownloadProgress {
    model_id: String,
    downloaded: u64,
    total: u64,
    percent: f64,
}

#[tauri::command]
pub async fn download_llm_model(
    app_handle: tauri::AppHandle,
    model_id: String,
) -> Result<String, String> {
    use futures_util::StreamExt;
    use tauri::Emitter;
    use tokio::io::AsyncWriteExt;

    let model_def = MODELS
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("Unknown model: {}", model_id))?;

    let models_dir = get_llm_models_dir();
    if !models_dir.exists() {
        std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    }

    let file_path = models_dir.join(model_def.filename);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client.get(model_def.url).send().await.map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("Download failed: HTTP {}", res.status()));
    }

    let total_size = res.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(&file_path)
        .await
        .map_err(|e| e.to_string())?;
    let mut stream = res.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_emit = Instant::now();

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        if last_emit.elapsed().as_millis() > 200 || downloaded == total_size {
            let percent = if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };
            let _ = app_handle.emit(
                "llm_download_progress",
                LlmDownloadProgress {
                    model_id: model_id.clone(),
                    downloaded,
                    total: total_size,
                    percent,
                },
            );
            last_emit = Instant::now();
        }
    }

    // Hash the downloaded file and save sidecar
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    if let Err(e) = compute_and_cache_sha256(&file_path) {
        eprintln!("Warning: failed to cache SHA-256 sidecar: {}", e);
    }

    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn delete_llm_model(model_id: String) -> Result<(), String> {
    let model_def = MODELS
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("Unknown model: {}", model_id))?;

    let file_path = get_llm_models_dir().join(model_def.filename);
    if file_path.exists() {
        std::fs::remove_file(&file_path).map_err(|e| e.to_string())?;
    }
    // Remove SHA-256 sidecar if it exists
    let sidecar = sha256_sidecar_path(&file_path);
    if sidecar.exists() {
        let _ = std::fs::remove_file(&sidecar);
    }

    // If this was the selected model, clear the selection
    let mut settings = load_settings();
    if settings.local_llm.model_id.as_deref() == Some(&model_id) {
        settings.local_llm.model_id = None;
        settings.local_llm.enabled = false;
        let _ = crate::config::save_settings(settings);
    }

    Ok(())
}

// ── Freshness Check ─────────────────────────────────────────────────────────

fn sha256_sidecar_path(model_path: &PathBuf) -> PathBuf {
    model_path.with_extension("gguf.sha256")
}

/// Compute SHA-256 of a file using streaming reads (no full-file allocation).
fn compute_sha256(path: &PathBuf) -> Result<String, String> {
    use sha2::{Sha256, Digest};
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut reader = std::io::BufReader::with_capacity(8 * 1024 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 8 * 1024 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Compute SHA-256 and write to sidecar file.
fn compute_and_cache_sha256(model_path: &PathBuf) -> Result<String, String> {
    let hash = compute_sha256(model_path)?;
    let sidecar = sha256_sidecar_path(model_path);
    std::fs::write(&sidecar, &hash).map_err(|e| e.to_string())?;
    Ok(hash)
}

/// Read cached SHA-256 from sidecar, or compute + cache if missing.
fn get_local_sha256(model_path: &PathBuf) -> Result<String, String> {
    let sidecar = sha256_sidecar_path(model_path);
    if sidecar.exists() {
        let cached = std::fs::read_to_string(&sidecar).map_err(|e| e.to_string())?;
        let cached = cached.trim().to_string();
        if cached.len() == 64 {
            return Ok(cached);
        }
    }
    compute_and_cache_sha256(model_path)
}

/// Fetch the remote SHA-256 from HuggingFace via HEAD request ETag header.
fn fetch_remote_sha256(url: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client.head(url).send().map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("HEAD request failed: HTTP {}", res.status()));
    }

    let etag = res
        .headers()
        .get("etag")
        .or_else(|| res.headers().get("x-linked-etag"))
        .ok_or("No ETag header in response")?
        .to_str()
        .map_err(|e| e.to_string())?;

    // HuggingFace ETag format: "sha256:abc123..." or "\"abc123...\""
    let hash = etag.trim_matches('"');
    let hash = hash.strip_prefix("sha256:").unwrap_or(hash);
    Ok(hash.to_string())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LlmFreshnessEntry {
    pub status: String, // "up_to_date", "update_available", "error"
    pub detail: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LlmFreshnessReport {
    pub models: HashMap<String, LlmFreshnessEntry>,
    pub checked: usize,
    pub failed: usize,
}

#[tauri::command]
pub fn check_all_llm_freshness() -> Result<LlmFreshnessReport, String> {
    let models_dir = get_llm_models_dir();
    let mut results: HashMap<String, LlmFreshnessEntry> = HashMap::new();
    let mut checked: usize = 0;
    let mut failed: usize = 0;

    for model_def in MODELS {
        let local_path = models_dir.join(model_def.filename);
        if !local_path.exists() {
            continue; // skip models that aren't downloaded
        }

        let local_hash = match get_local_sha256(&local_path) {
            Ok(h) => h,
            Err(e) => {
                failed += 1;
                results.insert(model_def.id.to_string(), LlmFreshnessEntry {
                    status: "error".to_string(),
                    detail: Some(format!("Local hash failed: {}", e)),
                });
                continue;
            }
        };

        let remote_hash = match fetch_remote_sha256(model_def.url) {
            Ok(h) => h,
            Err(e) => {
                failed += 1;
                results.insert(model_def.id.to_string(), LlmFreshnessEntry {
                    status: "error".to_string(),
                    detail: Some(format!("Remote check failed: {}", e)),
                });
                continue;
            }
        };

        checked += 1;
        let status = if local_hash == remote_hash {
            "up_to_date"
        } else {
            "update_available"
        };

        results.insert(model_def.id.to_string(), LlmFreshnessEntry {
            status: status.to_string(),
            detail: None,
        });
    }

    Ok(LlmFreshnessReport {
        models: results,
        checked,
        failed,
    })
}

// ── Inference Engine ────────────────────────────────────────────────────────

struct CachedModel {
    model: llama_cpp_2::model::LlamaModel,
    backend: llama_cpp_2::llama_backend::LlamaBackend,
    model_id: String,
    last_used: Instant,
}

// SAFETY: LlamaModel and LlamaBackend are internally thread-safe in llama.cpp.
// We only access the cached model behind a Mutex, so this is safe.
unsafe impl Send for CachedModel {}

lazy_static::lazy_static! {
    static ref CACHED_MODEL: Mutex<Option<CachedModel>> = Mutex::new(None);
}

/// Resolve model_id → local file path
fn resolve_model_path(model_id: &str) -> Result<PathBuf, String> {
    let model_def = MODELS
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("Unknown model: {}", model_id))?;

    let path = get_llm_models_dir().join(model_def.filename);
    if !path.exists() {
        return Err(format!(
            "Model '{}' not downloaded. Download it in Settings first.",
            model_def.name
        ));
    }
    Ok(path)
}

/// Run local LLM inference. Caches the loaded model between calls.
pub fn run_inference(prompt: &str, max_tokens: u32) -> Result<String, String> {
    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::llama_batch::LlamaBatch;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaModel};
    use llama_cpp_2::sampling::LlamaSampler;
    use std::num::NonZeroU32;

    let settings = load_settings();
    let model_id = settings
        .local_llm
        .model_id
        .as_deref()
        .ok_or("No local LLM model selected. Configure in Settings.")?;

    let model_path = resolve_model_path(model_id)?;
    let gpu_layers = settings.local_llm.gpu_layers;
    let context_size = settings.local_llm.context_size;

    // Load or reuse cached model
    let mut cache = CACHED_MODEL.lock().map_err(|e| e.to_string())?;

    let needs_load = match &*cache {
        Some(c) => c.model_id != model_id,
        None => true,
    };

    if needs_load {
        eprintln!("Loading local LLM: {} ...", model_id);

        let backend = LlamaBackend::init().map_err(|e| format!("Backend init failed: {}", e))?;

        let model_params =
            LlamaModelParams::default().with_n_gpu_layers(gpu_layers);

        let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)
            .map_err(|e| format!("Failed to load model: {}", e))?;

        eprintln!("Model loaded: {} params, {} ctx", model.n_params(), model.n_ctx_train());

        *cache = Some(CachedModel {
            model,
            backend,
            model_id: model_id.to_string(),
            last_used: Instant::now(),
        });
    }

    let cached = cache.as_mut().unwrap();
    cached.last_used = Instant::now();

    // Format prompt with chat template
    let messages = vec![
        LlamaChatMessage::new("user".to_string(), prompt.to_string())
            .map_err(|e| format!("Failed to create message: {}", e))?,
    ];

    let formatted = match cached.model.chat_template(None) {
        Ok(template) => cached
            .model
            .apply_chat_template(&template, &messages, true)
            .map_err(|e| format!("Chat template error: {}", e))?,
        Err(_) => {
            // Fallback: raw prompt if model has no chat template
            prompt.to_string()
        }
    };

    // Tokenize
    let tokens = cached
        .model
        .str_to_token(&formatted, AddBos::Always)
        .map_err(|e| format!("Tokenization failed: {}", e))?;

    // Create context
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(context_size));

    let mut ctx = cached
        .model
        .new_context(&cached.backend, ctx_params)
        .map_err(|e| format!("Context creation failed: {}", e))?;

    // Create batch and add prompt tokens
    let n_tokens = tokens.len();
    let mut batch = LlamaBatch::new(context_size as usize, 1);

    for (i, token) in tokens.iter().enumerate() {
        let is_last = i == n_tokens - 1;
        batch
            .add(*token, i as i32, &[0], is_last)
            .map_err(|e| format!("Batch add failed: {}", e))?;
    }

    // Decode prompt
    ctx.decode(&mut batch)
        .map_err(|e| format!("Prompt decode failed: {}", e))?;

    // Generate
    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::temp(0.3),
        LlamaSampler::top_p(0.9, 1),
        LlamaSampler::penalties(64, 1.1, 0.0, 0.0),
        LlamaSampler::greedy(),
    ]);

    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut output = String::new();
    let mut n_cur = n_tokens as i32;

    for _ in 0..max_tokens {
        let new_token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(new_token);

        if cached.model.is_eog_token(new_token) {
            break;
        }

        let token_str = cached
            .model
            .token_to_piece(new_token, &mut decoder, false, None)
            .map_err(|e| format!("Token decode failed: {}", e))?;

        output.push_str(&token_str);

        batch.clear();
        batch
            .add(new_token, n_cur, &[0], true)
            .map_err(|e| format!("Batch add failed: {}", e))?;

        ctx.decode(&mut batch)
            .map_err(|e| format!("Decode failed: {}", e))?;

        n_cur += 1;
    }

    Ok(output.trim().to_string())
}

/// Process text with a prompt using the local LLM (matches cloud_ai interface)
pub fn process_with_local(prompt: &str, text: &str) -> Result<String, String> {
    let full_prompt = if text.is_empty() {
        prompt.to_string()
    } else {
        prompt.replace("{transcript}", text)
    };

    // Estimate max output tokens: 4096 for typical processing tasks
    run_inference(&full_prompt, 4096)
}

/// Summarize text using the local LLM
pub fn summarize_with_local(text: &str) -> Result<String, String> {
    let prompt = format!(
        "Create a concise summary of this transcript. Include:\n\
        - Main topics discussed\n\
        - Key points and decisions\n\
        - Action items if any\n\n\
        Transcript:\n{}",
        text
    );
    run_inference(&prompt, 4096)
}
