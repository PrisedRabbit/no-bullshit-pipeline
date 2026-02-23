use serde::{Deserialize, Serialize};

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContentBlock {
    text: Option<String>,
    #[serde(rename = "type")]
    block_type: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Option<Vec<ClaudeContentBlock>>,
    error: Option<ClaudeError>,
}

#[derive(Debug, Deserialize)]
struct ClaudeError {
    message: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeErrorResponse {
    error: ClaudeError,
}

/// Process text with a Claude model (200K token context)
/// Best for nuanced extraction and structured data
pub async fn process_with_claude(api_key: &str, prompt: &str, text: &str, model: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let full_prompt = prompt.replace("{transcript}", text);

    let model_id = if model.is_empty() { "claude-sonnet-4-20250514" } else { model };

    let request = ClaudeRequest {
        model: model_id.to_string(),
        max_tokens: 4096,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: full_prompt,
        }],
    };

    let response = client
        .post(CLAUDE_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let status = response.status();
    let body = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        // Try to parse error
        if let Ok(error_resp) = serde_json::from_str::<ClaudeErrorResponse>(&body) {
            return Err(format!("Anthropic API error: {}", error_resp.error.message));
        }
        return Err(format!("API request failed ({}): {}", status, body));
    }

    let result: ClaudeResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if let Some(error) = result.error {
        return Err(format!("Anthropic API error: {}", error.message));
    }

    result
        .content
        .and_then(|blocks| {
            blocks
                .into_iter()
                .filter(|b| b.block_type == "text")
                .find_map(|b| b.text)
        })
        .ok_or_else(|| format!("No response from Anthropic model '{}'", model_id))
}

/// Extract structured data using Claude (Meeting Notes format)
#[allow(dead_code)]
pub async fn extract_meeting_notes(api_key: &str, transcript: &str) -> Result<String, String> {
    let prompt = r#"Extract structured data from this meeting transcript. Return valid JSON with:
{
  "date": "YYYY-MM-DD or 'Not identified'",
  "attendees": ["list of people mentioned or speaking"],
  "agenda_items": ["main topics discussed"],
  "key_decisions": ["decisions made during the meeting"],
  "action_items": [{"owner": "person", "task": "description", "due": "date if mentioned or null"}],
  "follow_ups": ["items that need future attention"]
}

If you cannot identify a section, use "Not identified" for strings or empty arrays for lists.

Transcript:
{transcript}"#;

    process_with_claude(api_key, prompt, transcript, "").await
}

/// Extract structured data using Claude (Brainstorm format)
#[allow(dead_code)]
pub async fn extract_brainstorm(api_key: &str, transcript: &str) -> Result<String, String> {
    let prompt = r#"Analyze this brainstorm session and organize the ideas. Return as clean Markdown:

# Brainstorm Summary

## Topic
[What is being brainstormed]

## Ideas by Theme
Group all ideas into logical themes/categories with numbered lists.

## Top 3 Priorities
The most important or emphasized ideas.

## Next Steps
Any action items or follow-ups mentioned.

Transcript:
{transcript}"#;

    process_with_claude(api_key, prompt, transcript, "").await
}

/// Extract structured data using Claude (Journal format)
#[allow(dead_code)]
pub async fn extract_journal(api_key: &str, transcript: &str) -> Result<String, String> {
    let prompt = r#"Transform this voice journal entry into a formatted diary entry. Write in first person, preserving the personal voice. Format as warm, readable Markdown:

# Journal Entry

**Date:** [Today's date or mentioned date]
**Mood:** [Infer emotional tone: Reflective, Energetic, Grateful, Anxious, Peaceful, etc.] [appropriate emoji]

## Key Thoughts
Main ideas or events discussed.

## Reflections
Any insights or realizations.

## Gratitude
Things expressed thanks for (if any, otherwise omit this section).

Transcript:
{transcript}"#;

    process_with_claude(api_key, prompt, transcript, "").await
}
