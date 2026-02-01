use serde::{Deserialize, Serialize};
use std::path::Path;

const WHISPER_API_URL: &str = "https://api.openai.com/v1/audio/transcriptions";
const CHAT_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIError {
    error: OpenAIErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorDetail {
    message: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: Option<String>,
    #[allow(dead_code)]
    code: Option<String>,
}

/// Transcribe audio using OpenAI Whisper-1 API
pub async fn transcribe_with_whisper(api_key: &str, audio_path: &Path) -> Result<String, String> {
    let client = reqwest::Client::new();

    // Read the audio file
    let file_bytes = std::fs::read(audio_path)
        .map_err(|e| format!("Failed to read audio file: {}", e))?;

    let file_name = audio_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.ogg")
        .to_string();

    // Create multipart form
    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name)
        .mime_str("audio/ogg")
        .map_err(|e| format!("Failed to create form part: {}", e))?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-1")
        .text("response_format", "json");

    let response = client
        .post(WHISPER_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let status = response.status();
    let body = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        // Try to parse error response
        if let Ok(error) = serde_json::from_str::<OpenAIError>(&body) {
            return Err(format!("OpenAI API error: {}", error.error.message));
        }
        return Err(format!("API request failed ({}): {}", status, body));
    }

    let result: WhisperResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(result.text)
}

/// Summarize text using GPT-4o
pub async fn summarize_with_gpt4o(api_key: &str, text: &str, custom_prompt: Option<&str>) -> Result<String, String> {
    let client = reqwest::Client::new();

    let system_prompt = "You are a helpful assistant that creates concise, well-structured summaries. Focus on key points, decisions, and action items.";

    let user_prompt = custom_prompt.unwrap_or(
        "Create a concise summary of this transcript. Include:\n\
        - Main topics discussed\n\
        - Key points and decisions\n\
        - Action items if any\n\
        - Important quotes or statements\n\n\
        Transcript:\n"
    );

    let request = ChatRequest {
        model: "gpt-4o".to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!("{}{}", user_prompt, text),
            },
        ],
        max_tokens: 4096,
    };

    let response = client
        .post(CHAT_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let status = response.status();
    let body = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        if let Ok(error) = serde_json::from_str::<OpenAIError>(&body) {
            return Err(format!("OpenAI API error: {}", error.error.message));
        }
        return Err(format!("API request failed ({}): {}", status, body));
    }

    let result: ChatResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    result
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| "No response from GPT-4o".to_string())
}

/// Process text with GPT-4o using a custom prompt (for templates)
pub async fn process_with_gpt4o(api_key: &str, prompt: &str, text: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let request = ChatRequest {
        model: "gpt-4o".to_string(),
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: prompt.replace("{transcript}", text),
            },
        ],
        max_tokens: 4096,
    };

    let response = client
        .post(CHAT_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let status = response.status();
    let body = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        if let Ok(error) = serde_json::from_str::<OpenAIError>(&body) {
            return Err(format!("OpenAI API error: {}", error.error.message));
        }
        return Err(format!("API request failed ({}): {}", status, body));
    }

    let result: ChatResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    result
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| "No response from GPT-4o".to_string())
}
