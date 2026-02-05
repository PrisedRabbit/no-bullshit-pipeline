use serde::{Deserialize, Serialize};
use crate::integrations::get_slack_token;
use chrono::Utc;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SlackConnectorConfig {
    pub integration_id: String,
    pub target: String,  // #channel, email@example.com, U123456, C123456, or D123456
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_ts: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackPostMessageRequest {
    channel: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_ts: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackPostMessageResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ts: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackUsersLookupResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<SlackUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackUser {
    id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackConversationsOpenResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<SlackChannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackChannel {
    id: String,
}

/// Resolve target to a channel/DM ID
async fn resolve_target(token: &str, target: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    // Already a channel/DM ID (C..., D..., G...)
    if target.starts_with('C') || target.starts_with('D') || target.starts_with('G') {
        return Ok(target.to_string());
    }

    // User ID (U...)
    if target.starts_with('U') {
        // Open DM conversation
        let resp = client
            .post("https://slack.com/api/conversations.open")
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({ "users": target }))
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let result: SlackConversationsOpenResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if !result.ok {
            return Err(result.error.unwrap_or_else(|| "Failed to open DM".to_string()));
        }

        return Ok(result.channel.ok_or("No channel in response")?.id);
    }

    // Email address
    if target.contains('@') && !target.starts_with('#') {
        // Lookup user by email
        let resp = client
            .get("https://slack.com/api/users.lookupByEmail")
            .header("Authorization", format!("Bearer {}", token))
            .query(&[("email", target)])
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let user_result: SlackUsersLookupResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if !user_result.ok {
            return Err(user_result.error.unwrap_or_else(|| "User not found".to_string()));
        }

        let user_id = user_result.user.ok_or("No user in response")?.id;

        // Open DM with user
        let dm_resp = client
            .post("https://slack.com/api/conversations.open")
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({ "users": user_id }))
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let dm_result: SlackConversationsOpenResponse = dm_resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if !dm_result.ok {
            return Err(dm_result.error.unwrap_or_else(|| "Failed to open DM".to_string()));
        }

        return Ok(dm_result.channel.ok_or("No channel in response")?.id);
    }

    // Channel name (#channel or just channel)
    let channel_name = target.strip_prefix('#').unwrap_or(target);
    
    // For channel names, just try to post directly (Slack will resolve it)
    // If the bot is not in the channel, it will fail with not_in_channel error
    Ok(format!("#{}", channel_name))
}

/// Execute Slack connector (internal)
async fn execute_slack_connector(
    config: &SlackConnectorConfig,
    input_content: &str,
) -> Result<String, String> {
    // Get token from Keychain
    let token = get_slack_token(&config.integration_id)?;

    // Resolve target to channel ID
    let channel_id = resolve_target(&token, &config.target).await?;

    // Strip frontmatter if present
    let content = crate::connectors::strip_frontmatter(input_content);

    // Post message
    let client = reqwest::Client::new();
    let resp = client
        .post("https://slack.com/api/chat.postMessage")
        .header("Authorization", format!("Bearer {}", token))
        .json(&SlackPostMessageRequest {
            channel: channel_id.clone(),
            text: content.to_string(),
            thread_ts: config.thread_ts.clone(),
        })
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let result: SlackPostMessageResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if !result.ok {
        return Err(result.error.unwrap_or_else(|| "Failed to post message".to_string()));
    }

    Ok(format!("Posted to Slack: {}", config.target))
}

/// Execute Slack connector (public entry point for pipeline engine)
pub async fn execute(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    input_step: &str,
    description: Option<&str>,
) -> Result<PathBuf, String> {
    // Parse config
    let slack_config: SlackConnectorConfig = serde_json::from_value(config.clone())
        .map_err(|e| format!("Invalid Slack connector config: {}", e))?;

    // Read input content
    let input_content = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input file: {}", e))?;

    // Execute Slack post
    let result = execute_slack_connector(&slack_config, &input_content).await?;

    // Write output markdown with frontmatter
    let output_path = output_dir.join(format!("{}.md", step_name));
    let now = Utc::now().to_rfc3339();
    let frontmatter = format!(
        r#"---
name: {}
description: "{}"
connector: slack
input: {}
status: done
created_at: {}
completed_at: {}
integration_id: {}
target: {}
thread_ts: {}
error: null
---

{}
"#,
        step_name,
        description.unwrap_or("Post to Slack"),
        input_step,
        now,
        now,
        slack_config.integration_id,
        slack_config.target,
        slack_config.thread_ts.as_deref().unwrap_or("null"),
        result
    );

    fs::write(&output_path, frontmatter)
        .map_err(|e| format!("Failed to write output: {}", e))?;

    Ok(output_path)
}
