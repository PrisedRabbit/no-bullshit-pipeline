use serde::{Deserialize, Serialize};

/// A model available from a provider API
#[derive(Debug, Serialize, Clone)]
pub struct ProviderModel {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<String>,
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
    if id.starts_with("gpt-")
        || id.starts_with("o1-")
        || id.starts_with("o3-")
        || id.starts_with("o4-")
        || id.starts_with("chatgpt-")
    {
        Some(vec!["chat".to_string()])
    } else if id.starts_with("whisper-") {
        Some(vec!["transcription".to_string()])
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
            openai_capabilities(&m.id).map(|caps| ProviderModel {
                name: m.id.clone(),
                id: m.id,
                capabilities: caps,
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
        .map(|m| ProviderModel {
            name: m.display_name,
            id: m.id,
            capabilities: vec!["chat".to_string()],
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
            })
        })
        .collect();

    Ok(models)
}

// ===== Tauri Command =====

#[tauri::command]
pub async fn fetch_provider_models(provider: String) -> Result<Vec<ProviderModel>, String> {
    let settings = crate::config::load_settings();
    let api_keys = &settings.transcription.api_keys;

    match provider.as_str() {
        "openai" => {
            let key = api_keys
                .openai
                .as_ref()
                .ok_or("OpenAI API key not configured")?;
            fetch_openai_models(key).await
        }
        "anthropic" => {
            let key = api_keys
                .anthropic
                .as_ref()
                .ok_or("Anthropic API key not configured")?;
            fetch_anthropic_models(key).await
        }
        "google" => {
            let key = api_keys
                .google
                .as_ref()
                .ok_or("Google API key not configured")?;
            fetch_google_models(key).await
        }
        other => Err(format!("Unknown provider: {}", other)),
    }
}
