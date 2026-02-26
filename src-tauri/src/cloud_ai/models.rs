use serde::{Deserialize, Serialize};
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref PROVIDER_MODELS_CACHE_LOCK: Mutex<()> = Mutex::new(());
}

const MODELS_CACHE_TTL_SECS: i64 = 24 * 60 * 60;

/// A model available from a provider API
#[derive(Debug, Serialize, Clone)]
pub struct ProviderModel {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub deprecated: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProviderModelsResponse {
    pub models: Vec<ProviderModel>,
    pub cached: bool,
    pub fetched_at: Option<i64>,
    pub stale: bool,
    pub error: Option<String>,
}

// ===== OpenAI =====

#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
    owned_by: String,
}

fn openai_capabilities(id: &str) -> Option<Vec<String>> {
    // Check transcription patterns first — models like gpt-4o-transcribe must
    // not be caught by the broad gpt-* chat prefix below.
    if id.starts_with("whisper-") || id.contains("-transcribe") {
        Some(vec!["transcription".to_string()])
    } else if id.starts_with("gpt-")
        || id.starts_with("o1-")
        || id.starts_with("o3-")
        || id.starts_with("o4-")
        || id.starts_with("chatgpt-")
    {
        Some(vec!["chat".to_string()])
    } else if id.starts_with("text-embedding-") {
        Some(vec!["embedding".to_string()])
    } else if id.starts_with("tts-") {
        Some(vec!["text-to-speech".to_string()])
    } else if id.starts_with("dall-e-") {
        Some(vec!["image".to_string()])
    } else {
        None
    }
}

async fn fetch_openai_models(api_key: &str) -> Result<Vec<ProviderModel>, String> {
    let client = reqwest::Client::new();

    let response = client
        .get("https://api.openai.com/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error ({})", response.status()));
    }

    let body: OpenAIModelsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let mut models: Vec<ProviderModel> = body
        .data
        .into_iter()
        .filter(|m| !m.owned_by.starts_with("user-"))
        .filter_map(|m| {
            let caps = openai_capabilities(&m.id)?;
            if !caps.iter().any(|c| c == "chat" || c == "transcription") {
                return None;
            }
            Some(ProviderModel {
                name: m.id.clone(),
                id: m.id,
                capabilities: caps,
                deprecated: false,
            })
        })
        .collect();

    models.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(models)
}

// ===== Anthropic =====

#[derive(Debug, Deserialize)]
struct AnthropicModelsResponse {
    data: Vec<AnthropicModel>,
}

#[derive(Debug, Deserialize)]
struct AnthropicModel {
    id: String,
    display_name: String,
}

async fn fetch_anthropic_models(api_key: &str) -> Result<Vec<ProviderModel>, String> {
    let client = reqwest::Client::new();

    let response = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error ({})", response.status()));
    }

    let body: AnthropicModelsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let models = body
        .data
        .into_iter()
        .map(|m| {
            let deprecated =
                m.id.starts_with("claude-2") || m.id.starts_with("claude-instant");
            ProviderModel {
                name: m.display_name,
                id: m.id,
                capabilities: vec!["chat".to_string()],
                deprecated,
            }
        })
        .collect();

    Ok(models)
}

// ===== Google =====

#[derive(Debug, Deserialize)]
struct GoogleModelsResponse {
    models: Vec<GoogleModel>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleModel {
    name: String,
    display_name: String,
    #[serde(default)]
    supported_generation_methods: Vec<String>,
}

fn google_capabilities(methods: &[String]) -> Vec<String> {
    let mut caps = vec![];
    if methods.iter().any(|m| m == "generateContent") {
        caps.push("chat".to_string());
    }
    if methods.iter().any(|m| m == "embedContent") {
        caps.push("embedding".to_string());
    }
    caps
}

async fn fetch_google_models(api_key: &str) -> Result<Vec<ProviderModel>, String> {
    let client = reqwest::Client::new();

    let response = client
        .get("https://generativelanguage.googleapis.com/v1beta/models")
        .query(&[("key", api_key)])
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error ({})", response.status()));
    }

    let body: GoogleModelsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let models = body
        .models
        .into_iter()
        .filter_map(|m| {
            let caps = google_capabilities(&m.supported_generation_methods);
            if caps.is_empty() {
                return None;
            }
            let id = m
                .name
                .strip_prefix("models/")
                .unwrap_or(&m.name)
                .to_string();
            Some(ProviderModel {
                name: m.display_name,
                id,
                capabilities: caps,
                deprecated: false,
            })
        })
        .collect();

    Ok(models)
}

// ===== Ollama =====

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
}

const OLLAMA_DEFAULT_URL: &str = "http://localhost:11434";

fn normalize_ollama_base_url(url: &str) -> String {
    let trimmed = url.trim();
    trimmed.trim_end_matches('/').to_string()
}

async fn fetch_ollama_models(base_url: Option<&str>) -> Result<Vec<ProviderModel>, String> {
    let normalized = normalize_ollama_base_url(base_url.unwrap_or(OLLAMA_DEFAULT_URL));
    let endpoint = format!("{}/api/tags", normalized);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&endpoint)
        .send()
        .await
        .map_err(|e| {
            let hint = if e.is_timeout() {
                "Connection timed out. Is Ollama responding?"
            } else if e.is_connect() {
                "Connection refused. Is Ollama running?"
            } else {
                "Network error. Check host and port."
            };
            format!("Cannot reach Ollama at {}: {} ({})", endpoint, e, hint)
        })?;

    if !response.status().is_success() {
        let status = response.status();
        return Err(format!(
            "Ollama API returned {} at {}. Is the service healthy?",
            status, endpoint
        ));
    }

    let body: OllamaTagsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response from {}: {}", endpoint, e))?;

    let mut models: Vec<ProviderModel> = body
        .models
        .into_iter()
        .map(|m| {
            let lower = m.name.to_lowercase();
            let capabilities = if lower.contains("embed") {
                vec!["embedding".to_string()]
            } else {
                vec!["chat".to_string()]
            };
            ProviderModel {
                name: m.name.clone(),
                id: m.name,
                capabilities,
                deprecated: false,
            }
        })
        .collect();

    models.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(models)
}

// ===== Tauri Command =====

#[tauri::command]
pub async fn fetch_provider_models(provider: String, force_refresh: Option<bool>) -> Result<ProviderModelsResponse, String> {
    let force = force_refresh.unwrap_or(false);
    let settings = crate::config::load_settings();
    let api_keys = &settings.transcription.api_keys;
    let now = chrono::Utc::now().timestamp();

    let provider_config = settings.providers.get(&provider);
    let cached_models = provider_config.map(|c| c.cached_models.clone()).unwrap_or_default();
    let cached_at = provider_config.and_then(|c| c.models_fetched_at);
    let base_url = provider_config.and_then(|c| c.base_url.as_deref());

    let is_stale = cached_at
        .map(|ts| now - ts > MODELS_CACHE_TTL_SECS)
        .unwrap_or(true);

    if !force && !is_stale && !cached_models.is_empty() {
        let models: Vec<ProviderModel> = cached_models
            .iter()
            .map(|m| ProviderModel {
                id: m.id.clone(),
                name: m.name.clone(),
                capabilities: m.capabilities.clone(),
                deprecated: m.deprecated,
            })
            .collect();
        return Ok(ProviderModelsResponse {
            models,
            cached: true,
            fetched_at: cached_at,
            stale: false,
            error: None,
        });
    }

    let fetch_result = match provider.as_str() {
        "openai" => {
            let key = api_keys.openai.as_ref().ok_or("OpenAI API key not configured")?;
            fetch_openai_models(key).await
        }
        "anthropic" => {
            let key = api_keys.anthropic.as_ref().ok_or("Anthropic API key not configured")?;
            fetch_anthropic_models(key).await
        }
        "google" => {
            let key = api_keys.google.as_ref().ok_or("Google API key not configured")?;
            fetch_google_models(key).await
        }
        "ollama" => fetch_ollama_models(base_url).await,
        other => Err(format!("Unknown provider: {}", other)),
    };

    match fetch_result {
        Ok(models) => {
            save_provider_models_full(&provider, &models, now);
            Ok(ProviderModelsResponse {
                models,
                cached: false,
                fetched_at: Some(now),
                stale: false,
                error: None,
            })
        }
        Err(e) => {
            if !cached_models.is_empty() {
                let models: Vec<ProviderModel> = cached_models
                    .iter()
                    .map(|m| ProviderModel {
                        id: m.id.clone(),
                        name: m.name.clone(),
                        capabilities: m.capabilities.clone(),
                        deprecated: m.deprecated,
                    })
                    .collect();
                Ok(ProviderModelsResponse {
                    models,
                    cached: true,
                    fetched_at: cached_at,
                    stale: true,
                    error: Some(e),
                })
            } else {
                Ok(ProviderModelsResponse {
                    models: vec![],
                    cached: false,
                    fetched_at: None,
                    stale: false,
                    error: Some(e),
                })
            }
        }
    }
}



fn save_provider_models_full(provider: &str, models: &[ProviderModel], fetched_at: i64) {
    let _lock = PROVIDER_MODELS_CACHE_LOCK.lock().unwrap();
    let mut settings = crate::config::load_settings();
    let cached: Vec<crate::config::CachedModel> = models
        .iter()
        .map(|m| crate::config::CachedModel {
            id: m.id.clone(),
            name: m.name.clone(),
            capabilities: m.capabilities.clone(),
            deprecated: m.deprecated,
        })
        .collect();
    let entry = settings
        .providers
        .entry(provider.to_string())
        .or_insert_with(crate::config::ProviderConfig::default);
    entry.models = models.iter().map(|m| m.id.clone()).collect();
    entry.cached_models = cached;
    entry.models_fetched_at = Some(fetched_at);
    let _ = crate::config::save_settings(settings);
}
