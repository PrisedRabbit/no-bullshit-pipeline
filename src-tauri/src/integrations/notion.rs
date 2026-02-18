// Notion integration module — types and profile I/O
// Note: No Tauri commands in this file — those come in Plan 02.

use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;

// ──────────────────────────────────────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────────────────────────────────────

/// A Notion property definition discovered during schema sync.
/// `property_type` is one of: "title", "rich_text", "select", "multi_select",
/// "people", "date", "number", "checkbox", "url", "email", "phone_number",
/// "formula", "relation", "rollup", "created_time", "last_edited_time", "status".
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotionPropertyDef {
    pub name: String,
    pub property_type: String,
    #[serde(default)]
    pub select_options: Vec<String>,
}

/// A mapping from a local alias (e.g. "me") to a Notion workspace user.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PeopleMapping {
    pub alias: String,
    pub notion_user_id: String,
    pub display_name: String,
}

/// A Notion workspace user discovered during schema sync.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorkspaceUser {
    pub id: String,
    pub name: Option<String>,
}

/// Full integration profile for a single Notion database connection.
/// Stored as `~/.nbp/integrations/notion-{id}.json`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotionIntegrationProfile {
    pub id: String,
    pub name: String,
    pub database_id: String,
    pub database_name: String,
    #[serde(default)]
    pub properties: Vec<NotionPropertyDef>,
    #[serde(default)]
    pub people_mappings: Vec<PeopleMapping>,
    #[serde(default)]
    pub workspace_users: Vec<WorkspaceUser>,
    pub synced_at: String,
}

// ──────────────────────────────────────────────────────────────────────────────
// Profile I/O
// ──────────────────────────────────────────────────────────────────────────────

/// Save a Notion integration profile to disk.
/// Writes to `~/.nbp/integrations/notion-{id}.json` with permissions 0o600.
pub fn save_notion_profile(profile: &NotionIntegrationProfile) -> Result<(), String> {
    let dir = crate::config::get_integrations_dir();
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create integrations directory: {}", e))?;

    let path = dir.join(format!("notion-{}.json", profile.id));
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("Failed to serialize Notion profile: {}", e))?;

    fs::write(&path, &json)
        .map_err(|e| format!("Failed to write Notion profile: {}", e))?;

    // Set permissions to 0o600 (user read/write only)
    let mut perms = fs::metadata(&path)
        .map_err(|e| format!("Failed to read profile metadata: {}", e))?
        .permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&path, perms)
        .map_err(|e| format!("Failed to set profile permissions: {}", e))?;

    Ok(())
}

/// Load a Notion integration profile from disk by its ID.
pub fn load_notion_profile(integration_id: &str) -> Result<NotionIntegrationProfile, String> {
    let path = crate::config::get_integrations_dir()
        .join(format!("notion-{}.json", integration_id));

    let data = fs::read_to_string(&path).map_err(|_| {
        format!(
            "Notion integration '{}' profile not found. Sync schema in Settings > Integrations.",
            integration_id
        )
    })?;

    serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse Notion profile '{}': {}", integration_id, e))
}

/// Delete a Notion integration profile from disk.
pub fn delete_notion_profile(integration_id: &str) -> Result<(), String> {
    let path = crate::config::get_integrations_dir()
        .join(format!("notion-{}.json", integration_id));

    fs::remove_file(&path)
        .map_err(|e| format!("Failed to delete Notion profile '{}': {}", integration_id, e))
}

/// List all Notion integration profiles from disk.
/// Reads all `notion-*.json` files from the integrations directory.
pub fn list_notion_profiles() -> Result<Vec<NotionIntegrationProfile>, String> {
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

        if file_name.starts_with("notion-") && file_name.ends_with(".json") {
            let data = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read profile '{}': {}", file_name, e))?;
            let profile: NotionIntegrationProfile = serde_json::from_str(&data)
                .map_err(|e| format!("Failed to parse profile '{}': {}", file_name, e))?;
            profiles.push(profile);
        }
    }

    Ok(profiles)
}

// ──────────────────────────────────────────────────────────────────────────────
// Token helper wrappers (delegate to shared dev-mode-aware helpers in mod.rs)
// ──────────────────────────────────────────────────────────────────────────────

/// Save Notion API token for an integration.
pub fn save_notion_token(integration_id: &str, token: &str) -> Result<(), String> {
    super::save_token(&format!("notion:{}", integration_id), token)
}

/// Retrieve Notion API token for an integration.
pub fn get_notion_token(integration_id: &str) -> Result<String, String> {
    super::get_token(&format!("notion:{}", integration_id))
}

/// Delete Notion API token for an integration.
pub fn delete_notion_token(integration_id: &str) -> Result<(), String> {
    super::delete_token(&format!("notion:{}", integration_id))
}
