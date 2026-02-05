use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use security_framework::passwords::{set_generic_password, get_generic_password, delete_generic_password};

const KEYCHAIN_SERVICE: &str = "com.skopanev.nbp";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SlackIntegration {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IntegrationsConfig {
    #[serde(default)]
    pub slack: HashMap<String, SlackIntegration>,
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

/// Save Slack token to macOS Keychain
fn save_slack_token(integration_id: &str, token: &str) -> Result<(), String> {
    let account = format!("slack:{}", integration_id);
    set_generic_password(KEYCHAIN_SERVICE, &account, token.as_bytes())
        .map_err(|e| format!("Failed to save token to Keychain: {}", e))
}

/// Get Slack token from macOS Keychain
pub fn get_slack_token(integration_id: &str) -> Result<String, String> {
    let account = format!("slack:{}", integration_id);
    let password = get_generic_password(KEYCHAIN_SERVICE, &account)
        .map_err(|e| format!("Token not found in Keychain: {}", e))?;
    String::from_utf8(password.to_vec())
        .map_err(|e| format!("Invalid token encoding: {}", e))
}

/// Delete Slack token from macOS Keychain
fn delete_slack_token(integration_id: &str) -> Result<(), String> {
    let account = format!("slack:{}", integration_id);
    delete_generic_password(KEYCHAIN_SERVICE, &account)
        .map_err(|e| format!("Failed to delete token from Keychain: {}", e))
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
    name: String,
    token: String,
    settings: tauri::State<'_, std::sync::Arc<std::sync::Mutex<crate::config::AppSettings>>>
) -> Result<(), String> {
    // Validate token by calling auth.test
    let auth_result = test_slack_connection(&token).await?;
    
    if !auth_result.ok {
        return Err(auth_result.error.unwrap_or_else(|| "Invalid token".to_string()));
    }

    // Save token to Keychain
    save_slack_token(&id, &token)?;

    // Save metadata to settings
    let mut settings = settings.lock().unwrap();
    settings.integrations.slack.insert(id, SlackIntegration {
        name,
        workspace_name: auth_result.team,
        workspace_url: auth_result.url,
    });

    // Persist settings
    crate::config::save_settings(settings.clone())?;

    Ok(())
}

/// Remove Slack integration
#[tauri::command]
pub fn remove_slack_integration(
    id: String,
    settings: tauri::State<'_, std::sync::Arc<std::sync::Mutex<crate::config::AppSettings>>>
) -> Result<(), String> {
    // Delete token from Keychain
    delete_slack_token(&id)?;

    // Remove from settings
    let mut settings = settings.lock().unwrap();
    settings.integrations.slack.remove(&id);

    // Persist settings
    crate::config::save_settings(settings.clone())?;

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
