// Legacy per-type Slack integration commands — the new Connections model
// (see `docs/connections-model.md`) replaces these with `save_connection` /
// `test_connection`. The Keychain helper `get_slack_token` is still called
// from the Slack connector, so the module stays. Module-level allow keeps
// the build quiet for the rest of the dead surface.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SlackIntegration {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackAuthTestResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    team: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SlackChannel {
    pub id: String,
    pub name: String,
    pub is_private: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SlackMember {
    pub id: String,
    pub name: String,
    pub display_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackConversationsResponse {
    ok: bool,
    #[serde(default)]
    channels: Vec<SlackChannelRaw>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackChannelRaw {
    id: String,
    name: String,
    #[serde(default)]
    is_private: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackUsersListResponse {
    ok: bool,
    #[serde(default)]
    members: Vec<SlackUserRaw>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackUserRaw {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    deleted: bool,
    #[serde(default)]
    is_bot: bool,
    #[serde(default)]
    profile: Option<SlackUserProfile>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackUserProfile {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    real_name: Option<String>,
}

/// Save Slack token using shared credential helper (dev-mode bypass in debug)
fn save_slack_token(integration_id: &str, token: &str) -> Result<(), String> {
    super::save_token(&format!("slack:{}", integration_id), token)
}

/// Get Slack token using shared credential helper (dev-mode bypass in debug)
pub fn get_slack_token(integration_id: &str) -> Result<String, String> {
    super::get_token(&format!("slack:{}", integration_id))
}

/// Delete Slack token using shared credential helper (dev-mode bypass in debug)
fn delete_slack_token(integration_id: &str) -> Result<(), String> {
    super::delete_token(&format!("slack:{}", integration_id))
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackTeamInfoResponse {
    ok: bool,
    #[serde(default)]
    team: Option<SlackTeamInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackTeamInfo {
    #[serde(default)]
    icon: Option<SlackTeamIcon>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackTeamIcon {
    #[serde(default)]
    image_44: Option<String>,
}

/// Fetch workspace icon URL via team.info
async fn fetch_workspace_icon(token: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://slack.com/api/team.info")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .ok()?;
    let info: SlackTeamInfoResponse = resp.json().await.ok()?;
    info.team?.icon?.image_44
}

/// Test Slack connection and fetch workspace info
async fn test_slack_connection(token: &str) -> Result<SlackAuthTestResponse, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://slack.com/api/auth.test")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    resp.json::<SlackAuthTestResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// List all Slack integrations
#[tauri::command]
pub fn list_slack_integrations(
    settings: tauri::State<'_, std::sync::Arc<std::sync::Mutex<crate::config::AppSettings>>>
) -> HashMap<String, SlackIntegration> {
    let settings = settings.lock().unwrap();
    settings.integrations.slack.clone()
}

/// Add new Slack integration
#[tauri::command]
pub async fn add_slack_integration(
    id: String,
    token: String,
    settings: tauri::State<'_, std::sync::Arc<std::sync::Mutex<crate::config::AppSettings>>>
) -> Result<String, String> {
    // Validate token by calling auth.test
    let auth_result = test_slack_connection(&token).await?;

    if !auth_result.ok {
        return Err(auth_result.error.unwrap_or_else(|| "Invalid token".to_string()));
    }

    // Auto-derive name from workspace
    let name = auth_result.team.clone().unwrap_or_else(|| "Slack".to_string());

    // Fetch workspace icon (best-effort, don't fail if unavailable)
    let icon_url = fetch_workspace_icon(&token).await;

    // Save token via shared credential helper
    save_slack_token(&id, &token)?;

    // Save metadata to settings
    let mut settings = settings.lock().unwrap();
    settings.integrations.slack.insert(id, SlackIntegration {
        name: name.clone(),
        workspace_name: auth_result.team,
        workspace_url: auth_result.url,
        icon_url,
    });

    // Persist settings
    crate::config::save_settings_to_disk(&mut settings.clone())?;

    Ok(name)
}

/// Remove Slack integration
#[tauri::command]
pub fn remove_slack_integration(
    id: String,
    settings: tauri::State<'_, std::sync::Arc<std::sync::Mutex<crate::config::AppSettings>>>
) -> Result<(), String> {
    // Delete token via shared credential helper
    delete_slack_token(&id)?;

    // Remove from settings
    let mut settings = settings.lock().unwrap();
    settings.integrations.slack.remove(&id);

    // Persist settings
    crate::config::save_settings_to_disk(&mut settings.clone())?;

    Ok(())
}

/// Test Slack integration connection
#[tauri::command]
pub async fn test_slack_integration(id: String) -> Result<String, String> {
    let token = get_slack_token(&id)?;
    let auth_result = test_slack_connection(&token).await?;

    if !auth_result.ok {
        return Err(auth_result.error.unwrap_or_else(|| "Connection failed".to_string()));
    }

    Ok(auth_result.team.unwrap_or_else(|| "Connected".to_string()))
}

/// List Slack channels for a specific integration
#[tauri::command]
pub async fn list_slack_channels(integration_id: String) -> Result<Vec<SlackChannel>, String> {
    let token = get_slack_token(&integration_id)?;

    let client = reqwest::Client::new();
    let resp = client
        .get("https://slack.com/api/conversations.list")
        .header("Authorization", format!("Bearer {}", token))
        .query(&[
            ("types", "public_channel,private_channel"),
            ("exclude_archived", "true"),
            ("limit", "1000"),
        ])
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let result: SlackConversationsResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if !result.ok {
        return Err(result.error.unwrap_or_else(|| "Failed to fetch channels".to_string()));
    }

    Ok(result.channels.into_iter().map(|ch| SlackChannel {
        id: ch.id,
        name: ch.name,
        is_private: ch.is_private,
    }).collect())
}

/// List Slack workspace members for a specific integration
#[tauri::command]
pub async fn list_slack_members(integration_id: String) -> Result<Vec<SlackMember>, String> {
    let token = get_slack_token(&integration_id)?;

    let client = reqwest::Client::new();
    let resp = client
        .get("https://slack.com/api/users.list")
        .header("Authorization", format!("Bearer {}", token))
        .query(&[("limit", "1000")])
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let result: SlackUsersListResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if !result.ok {
        return Err(result.error.unwrap_or_else(|| "Failed to fetch members".to_string()));
    }

    Ok(result.members.into_iter()
        .filter(|u| !u.deleted && !u.is_bot)
        .map(|u| {
            let display_name = u.profile
                .as_ref()
                .and_then(|p| p.display_name.as_deref().filter(|s| !s.is_empty()))
                .or_else(|| u.profile.as_ref().and_then(|p| p.real_name.as_deref().filter(|s| !s.is_empty())))
                .unwrap_or(&u.name)
                .to_string();
            SlackMember {
                id: u.id,
                name: u.name,
                display_name,
            }
        })
        .collect())
}
