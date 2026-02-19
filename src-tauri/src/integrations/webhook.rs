// Webhook integration module — types, profile I/O, and Tauri commands
//
// Webhook profiles are stored as `~/.nbp/integrations/webhook-{id}.json`.
// This follows the exact same file I/O pattern as save_path.rs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use uuid::Uuid;

// ──────────────────────────────────────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────────────────────────────────────

/// A named webhook endpoint integration profile.
/// Stored as `~/.nbp/integrations/webhook-{id}.json`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WebhookProfile {
    pub id: String,
    pub name: String,
    pub url: String,
    pub method: String,
    #[serde(default = "default_body_format")]
    pub body_format: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_timeout")]
    pub timeout_sec: u64,
}

fn default_body_format() -> String {
    "json".to_string()
}

fn default_timeout() -> u64 {
    30
}

// ──────────────────────────────────────────────────────────────────────────────
// Tauri commands
// ──────────────────────────────────────────────────────────────────────────────

/// Add a new named webhook endpoint integration.
/// Returns the new profile ID on success.
#[tauri::command]
pub fn add_webhook_integration(
    name: String,
    url: String,
    method: String,
    body_format: Option<String>,
    headers: Option<HashMap<String, String>>,
    timeout_sec: Option<u64>,
) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Webhook name cannot be empty.".to_string());
    }
    if url.trim().is_empty() {
        return Err("Webhook URL cannot be empty.".to_string());
    }

    let method = method.to_uppercase();
    if !["POST", "PUT", "PATCH"].contains(&method.as_str()) {
        return Err(format!(
            "Unsupported method '{}'. Must be POST, PUT, or PATCH.",
            method
        ));
    }

    let id = Uuid::new_v4().to_string();

    let profile = WebhookProfile {
        id: id.clone(),
        name: name.trim().to_string(),
        url: url.trim().to_string(),
        method,
        body_format: body_format.unwrap_or_else(|| "json".to_string()),
        headers: headers.unwrap_or_default(),
        timeout_sec: timeout_sec.unwrap_or(30).clamp(5, 300),
    };

    save_webhook_profile(&profile)?;

    Ok(id)
}

/// List all webhook integration profiles.
/// Reads all `webhook-{id}.json` files from the integrations directory.
/// Returns an empty vector if the directory does not exist.
#[tauri::command]
pub fn list_webhook_profiles() -> Result<Vec<WebhookProfile>, String> {
    let dir = crate::config::get_integrations_dir();

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read integrations directory: {}", e))?;

    let mut profiles = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if file_name.starts_with("webhook-") && file_name.ends_with(".json") {
            let data = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read webhook profile '{}': {}", file_name, e))?;
            let profile: WebhookProfile = serde_json::from_str(&data)
                .map_err(|e| format!("Failed to parse webhook profile '{}': {}", file_name, e))?;
            profiles.push(profile);
        }
    }

    Ok(profiles)
}

/// Remove a webhook integration profile by ID.
/// Treats "not found" as success (idempotent).
#[tauri::command]
pub fn remove_webhook_integration(id: String) -> Result<(), String> {
    let file_path = crate::config::get_integrations_dir()
        .join(format!("webhook-{}.json", id));

    match fs::remove_file(&file_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Failed to remove webhook profile '{}': {}", id, e)),
    }
}

/// Test a webhook integration by sending a test request.
#[tauri::command]
pub async fn test_webhook_integration(id: String) -> Result<String, String> {
    let profile = load_webhook_profile(&id)?;

    let timeout = std::time::Duration::from_secs(profile.timeout_sec.min(10));

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut builder = match profile.method.as_str() {
        "POST" => client.post(&profile.url),
        "PUT" => client.put(&profile.url),
        "PATCH" => client.patch(&profile.url),
        _ => return Err(format!("Unsupported method: {}", profile.method)),
    };

    builder = builder.header("User-Agent", "NBP/0.4.0");

    for (key, value) in &profile.headers {
        builder = builder.header(key.as_str(), value.as_str());
    }

    builder = if profile.body_format == "text" {
        builder
            .header("Content-Type", "text/plain")
            .body("NBP webhook test")
    } else {
        builder
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({"source": "nbp", "type": "test"}))
    };

    match builder.send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                Ok(format!("Connected — HTTP {}", status.as_u16()))
            } else {
                Err(format!(
                    "HTTP {} — check your endpoint URL",
                    status.as_u16()
                ))
            }
        }
        Err(e) if e.is_timeout() => Err("Request timed out".to_string()),
        Err(e) if e.is_connect() => Err("Connection failed — check the URL".to_string()),
        Err(e) => Err(format!("Request failed: {}", e)),
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Profile I/O helpers (internal + pub for connector use)
// ──────────────────────────────────────────────────────────────────────────────

/// Load a webhook profile from disk by its ID.
pub fn load_webhook_profile(id: &str) -> Result<WebhookProfile, String> {
    let file_path = crate::config::get_integrations_dir()
        .join(format!("webhook-{}.json", id));

    let data = fs::read_to_string(&file_path)
        .map_err(|_| format!("Webhook integration '{}' not found.", id))?;

    serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse webhook profile '{}': {}", id, e))
}

fn save_webhook_profile(profile: &WebhookProfile) -> Result<(), String> {
    let dir = crate::config::get_integrations_dir();
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create integrations directory: {}", e))?;

    let file_path = dir.join(format!("webhook-{}.json", profile.id));
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("Failed to serialize webhook profile: {}", e))?;

    fs::write(&file_path, &json)
        .map_err(|e| format!("Failed to write webhook profile: {}", e))?;

    // Set permissions to 0o600 (user read/write only)
    let mut perms = fs::metadata(&file_path)
        .map_err(|e| format!("Failed to read webhook profile metadata: {}", e))?
        .permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&file_path, perms)
        .map_err(|e| format!("Failed to set webhook profile permissions: {}", e))?;

    Ok(())
}
