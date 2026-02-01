use serde::{Deserialize, Serialize};

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent";

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Debug, Deserialize, Clone)]
struct GeminiResponsePart {
    text: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Debug, Deserialize, Clone)]
struct GeminiCandidate {
    content: GeminiResponseContent,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    error: Option<GeminiError>,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    message: String,
    #[allow(dead_code)]
    code: Option<i32>,
}

/// Process text with Google Gemini Flash 1.5 (1M token context)
/// Ideal for very long transcripts (hour+ recordings)
pub async fn process_with_gemini(api_key: &str, prompt: &str, text: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let full_prompt = prompt.replace("{transcript}", text);

    let request = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart { text: full_prompt }],
        }],
    };

    let url = format!("{}?key={}", GEMINI_API_URL, api_key);

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let status = response.status();
    let body = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        // Try to parse error
        if let Ok(resp) = serde_json::from_str::<GeminiResponse>(&body) {
            if let Some(error) = resp.error {
                return Err(format!("Google API error: {}", error.message));
            }
        }
        return Err(format!("API request failed ({}): {}", status, body));
    }

    let result: GeminiResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if let Some(error) = result.error {
        return Err(format!("Google API error: {}", error.message));
    }

    result
        .candidates
        .and_then(|c| c.first().cloned())
        .and_then(|c| c.content.parts.first().cloned())
        .and_then(|p| p.text)
        .ok_or_else(|| "No response from Gemini".to_string())
}

/// Summarize text with Gemini (optimized for long transcripts)
pub async fn summarize_with_gemini(api_key: &str, text: &str) -> Result<String, String> {
    let prompt = "Create a comprehensive summary of this transcript. Include:\n\
        - Main topics discussed\n\
        - Key points and decisions\n\
        - Action items if any\n\
        - Important quotes or statements\n\n\
        Transcript:\n{transcript}";

    process_with_gemini(api_key, prompt, text).await
}
