// Telegram delivery connector — sends rendered template content to a chat
// via the Bot API (`https://api.telegram.org/bot{token}/sendMessage`).
//
// Connection.config shape (non-secret):
//   { "chat_id": "<id>", "parse_mode": "MarkdownV2" | "HTML" | null }
//
// Secret: the bot token lives in Keychain under `telegram:{connection_id}`,
// looked up via the same helper Slack/Notion use (see `integrations/mod.rs`).
// The engine injects `integration_id = connection_id` into config before
// calling `execute`, mirroring the Slack/Notion bridge.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const TELEGRAM_API_BASE: &str = "https://api.telegram.org";
const MAX_MESSAGE_LEN: usize = 4096;
const REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TelegramConnectorConfig {
    /// The Connection id, injected by the engine so `get_telegram_token` can
    /// pull the bot token from Keychain. Same bridge shape as Slack/Notion.
    pub integration_id: String,
    /// Destination chat. Numeric id ("-100123…") or `@channelname`.
    pub chat_id: String,
    /// Optional Telegram parse mode: "MarkdownV2", "HTML", or omitted for
    /// plain text. We pass through verbatim — Telegram validates it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<String>,
    /// Optional: suppress link previews. Defaults to false (Telegram default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_web_page_preview: Option<bool>,
}

#[derive(Serialize)]
struct SendMessageRequest<'a> {
    chat_id: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_mode: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disable_web_page_preview: Option<bool>,
}

#[derive(Deserialize)]
struct SendMessageResponse {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    result: Option<SentMessage>,
}

#[derive(Deserialize)]
struct SentMessage {
    message_id: i64,
}

/// Fetch the bot token from Keychain. Keyed `telegram:{connection_id}` so the
/// scheme matches the rest of the connectors (see `docs/connections-model.md`
/// closed decision #5).
fn get_telegram_token(connection_id: &str) -> Result<String, String> {
    crate::integrations::get_token(&format!("telegram:{}", connection_id))
}

async fn send_message(
    token: &str,
    cfg: &TelegramConnectorConfig,
    text: &str,
) -> Result<i64, String> {
    let url = format!("{}/bot{}/sendMessage", TELEGRAM_API_BASE, token);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let body = SendMessageRequest {
        chat_id: &cfg.chat_id,
        text,
        parse_mode: cfg.parse_mode.as_deref(),
        disable_web_page_preview: cfg.disable_web_page_preview,
    };

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error talking to Telegram: {}", e))?;

    let status = resp.status();
    let parsed: SendMessageResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Telegram response (HTTP {}): {}", status, e))?;

    if !parsed.ok {
        return Err(parsed
            .description
            .unwrap_or_else(|| format!("Telegram returned error (HTTP {})", status)));
    }

    Ok(parsed.result.map(|m| m.message_id).unwrap_or(0))
}

/// Execute Telegram connector for a pipeline step.
///
/// Reads the rendered template from `input_path` (engine wrote it there),
/// truncates to Telegram's 4096-char message limit (logging the cut), POSTs
/// to the Bot API, then writes a `<step>.md` artifact mirroring the Slack
/// shape so `get_step_outputs` reports `done`/`failed` uniformly.
pub async fn execute(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    step_input: &str,
    step_description: Option<&str>,
) -> Result<PathBuf, String> {
    let cfg: TelegramConnectorConfig = serde_json::from_value(config.clone())
        .map_err(|e| format!("Invalid Telegram connector config: {}", e))?;

    let raw = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input for Telegram step: {}", e))?;
    let content = crate::connectors::strip_frontmatter(&raw);

    // Telegram cap: 4096 chars per message. Splitting into multiple messages
    // is doable but adds order/threading concerns — for v1 we truncate with
    // a visible marker; users wanting full long output should pick Slack or
    // a webhook → store path.
    let (text, truncated) = if content.chars().count() > MAX_MESSAGE_LEN {
        let trimmed: String = content.chars().take(MAX_MESSAGE_LEN - 32).collect();
        (format!("{}\n\n…[truncated by NBP]", trimmed), true)
    } else {
        (content.to_string(), false)
    };

    let token = get_telegram_token(&cfg.integration_id)?;
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let send_result = send_message(&token, &cfg, &text).await;
    let completed_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;
    let output_path = output_dir.join(format!("{}.md", step_name));

    match send_result {
        Ok(message_id) => {
            let frontmatter = format!(
                r#"---
name: {}
description: "{}"
connector: telegram
input: {}
status: done
created_at: {}
completed_at: {}
connection_id: {}
chat_id: {}
message_id: {}
truncated: {}
error: null
---

Sent to Telegram chat {} (message {}){}
"#,
                step_name,
                step_description.unwrap_or("Send to Telegram").replace('"', "\\\""),
                step_input,
                created_at,
                completed_at,
                cfg.integration_id,
                cfg.chat_id,
                message_id,
                truncated,
                cfg.chat_id,
                message_id,
                if truncated { " — content truncated to 4096 chars" } else { "" }
            );
            fs::write(&output_path, frontmatter)
                .map_err(|e| format!("Failed to write Telegram artifact: {}", e))?;
            Ok(output_path)
        }
        Err(err) => {
            let err_escaped = err.replace('"', "\\\"").replace('\n', " ");
            let frontmatter = format!(
                r#"---
name: {}
description: "{}"
connector: telegram
input: {}
status: failed
created_at: {}
completed_at: {}
connection_id: {}
chat_id: {}
error: "{}"
---

## Error
{}
"#,
                step_name,
                step_description.unwrap_or("Send to Telegram").replace('"', "\\\""),
                step_input,
                created_at,
                completed_at,
                cfg.integration_id,
                cfg.chat_id,
                err_escaped,
                err,
            );
            let _ = fs::write(&output_path, frontmatter);
            Err(err)
        }
    }
}
