// Notion integration module — types, profile I/O, and (formerly) Tauri commands.
// Per-type integration commands are gone — Connections model replaces them.
// `load_notion_profile` + `get_notion_token` are still called by the Notion
// connector (its execute path still depends on a synced profile — see the
// note in connectors/notion.rs about the Notion type being temporarily
// hidden from the new UI pending a connector rewrite).
#![allow(dead_code)]

use notion_client::endpoints::Client;
use notion_client::endpoints::search::title::request::{
    Filter, FilterProperty, FilterValue, SearchByTitleRequest,
};
use notion_client::objects::database::DatabaseProperty;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

/// Minimal database info returned from list_notion_databases.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotionDatabaseInfo {
    pub id: String,
    pub name: String,
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
#[tauri::command]
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

// ──────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ──────────────────────────────────────────────────────────────────────────────

/// Build a Notion client from a raw API token (used during add_notion_integration validation).
fn make_client_from_token(token: String) -> Result<Client, String> {
    Client::new(token, None)
        .map_err(|e| format!("Failed to initialize Notion client: {:?}", e))
}

/// Build a Notion client from a stored integration token.
fn make_client(integration_id: &str) -> Result<Client, String> {
    let token = get_notion_token(integration_id)?;
    make_client_from_token(token)
}

// ──────────────────────────────────────────────────────────────────────────────
// Tauri commands
// ──────────────────────────────────────────────────────────────────────────────

/// Validate a Notion API key and create a new integration profile.
/// Returns the new integration ID on success.
/// The API key is never stored in the profile JSON — only in the dev bypass or Keychain.
#[tauri::command]
pub async fn add_notion_integration(api_key: String) -> Result<String, String> {
    // Validate by calling the bot user endpoint — if this fails, the key is invalid
    let client = make_client_from_token(api_key.clone())?;
    let bot_user = client
        .users
        .retrieve_your_tokens_bot_user()
        .await
        .map_err(|e| format!("Invalid Notion API key: {:?}", e))?;

    // Auto-derive name and icon from bot user
    let name = bot_user.name.unwrap_or_else(|| "Notion".to_string());
    let icon_url = bot_user.avatar_url;

    // Generate a stable UUID for this integration
    let id = uuid::Uuid::new_v4().to_string();

    // Store the token securely (dev-mode file or Keychain)
    save_notion_token(&id, &api_key)?;

    // Create an initial empty profile
    let profile = NotionIntegrationProfile {
        id: id.clone(),
        name,
        database_id: String::new(),
        database_name: String::new(),
        properties: Vec::new(),
        people_mappings: Vec::new(),
        workspace_users: Vec::new(),
        synced_at: String::new(),
        icon_url,
    };

    // Persist the profile to disk
    save_notion_profile(&profile)?;

    Ok(id)
}

/// List all Notion databases that the integration has been connected to.
/// Returns a helpful error if no databases are found (prompting the user to
/// share the integration with a database in the Notion UI).
#[tauri::command]
pub async fn list_notion_databases(
    integration_id: String,
) -> Result<Vec<NotionDatabaseInfo>, String> {
    let client = make_client(&integration_id)?;

    let request = SearchByTitleRequest {
        filter: Some(Filter {
            property: FilterProperty::Object,
            value: FilterValue::Database,
        }),
        query: None,
        sort: None,
        start_cursor: None,
        page_size: Some(100),
    };

    let response = client
        .search
        .search_by_title(request)
        .await
        .map_err(|e| format!("Failed to list databases: {:?}", e))?;

    // Extract database entries from the search results.
    // The response contains Vec<PageOrDatabase> — we extract id/title from each.
    let mut databases: Vec<NotionDatabaseInfo> = Vec::new();

    for item in &response.results {
        // Use serde_json to extract id and title from each result item
        // to avoid depending on the exact PageOrDatabase enum structure.
        let value = serde_json::to_value(item)
            .map_err(|e| format!("Failed to serialize search result: {}", e))?;

        let object_type = value.get("object").and_then(|v| v.as_str()).unwrap_or("");
        if object_type != "database" {
            continue;
        }

        let id = match value.get("id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };

        // Database titles are in `title` array — extract plain_text from first item
        let name = extract_title_text(&value);

        databases.push(NotionDatabaseInfo { id, name });
    }

    if databases.is_empty() {
        return Err(
            "No databases found. In Notion, open your database, click '...' menu, \
             then 'Connections', and add your integration."
                .to_string(),
        );
    }

    Ok(databases)
}

/// Sync the schema of a Notion database into the integration profile.
/// Reads all database properties and workspace users, then saves the updated profile.
/// Returns the updated profile so the frontend can display the synced schema.
#[tauri::command]
pub async fn sync_notion_schema(
    integration_id: String,
    database_id: String,
    database_name: String,
) -> Result<NotionIntegrationProfile, String> {
    let client = make_client(&integration_id)?;

    // Fetch the database schema
    let database = client
        .databases
        .retrieve_a_database(&database_id)
        .await
        .map_err(|e| format!("Failed to read database schema: {:?}", e))?;

    // Convert database properties to our internal type
    let mut properties: Vec<NotionPropertyDef> = Vec::new();
    for (prop_name, prop) in &database.properties {
        let prop_def = convert_database_property(prop_name, prop);
        properties.push(prop_def);
    }

    // Sort properties for deterministic output
    properties.sort_by(|a, b| a.name.cmp(&b.name));

    // Fetch all workspace users with pagination
    let mut workspace_users: Vec<WorkspaceUser> = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let response = client
            .users
            .list_all_users(cursor.as_deref(), Some(100))
            .await
            .map_err(|e| format!("Failed to list workspace users: {:?}", e))?;

        for user in &response.results {
            workspace_users.push(WorkspaceUser {
                id: user.id.clone(),
                name: user.name.clone(),
            });
        }

        if response.has_more {
            cursor = response.next_cursor.clone();
        } else {
            break;
        }
    }

    // Load the existing profile to preserve people_mappings, name, and icon
    let (existing_people_mappings, existing_name, existing_icon) = match load_notion_profile(&integration_id) {
        Ok(existing) => (existing.people_mappings, existing.name, existing.icon_url),
        Err(_) => (Vec::new(), database_name.clone(), None),
    };

    // Build the updated profile
    let profile = NotionIntegrationProfile {
        id: integration_id.clone(),
        name: existing_name,
        database_id,
        database_name,
        properties,
        people_mappings: existing_people_mappings,
        workspace_users,
        synced_at: chrono::Utc::now().to_rfc3339(),
        icon_url: existing_icon,
    };

    // Persist the updated profile
    save_notion_profile(&profile)?;

    Ok(profile)
}

/// Update the people mappings (alias-to-user) for a Notion integration.
/// Validates that all referenced Notion user IDs exist in the profile's workspace_users.
/// Returns an error if any mapping references a user ID not in the workspace.
#[tauri::command]
pub async fn update_notion_people_mappings(
    integration_id: String,
    mappings: Vec<PeopleMapping>,
) -> Result<(), String> {
    let mut profile = load_notion_profile(&integration_id)?;

    // Validate every mapping references a known workspace user
    for mapping in &mappings {
        let found = profile
            .workspace_users
            .iter()
            .any(|u| u.id == mapping.notion_user_id);
        if !found {
            return Err(format!(
                "User ID '{}' not found in workspace users. Re-sync the schema to refresh the user list.",
                mapping.notion_user_id
            ));
        }
    }

    profile.people_mappings = mappings;
    save_notion_profile(&profile)?;

    Ok(())
}

/// Test whether a Notion integration's stored API token is still valid.
/// Calls the bot user endpoint — if it succeeds, the token is active.
/// Returns "Connected" on success, or a descriptive error without revealing the token.
#[tauri::command]
pub async fn test_notion_integration(integration_id: String) -> Result<String, String> {
    let client = make_client(&integration_id)?;

    client
        .users
        .retrieve_your_tokens_bot_user()
        .await
        .map_err(|e| {
            format!(
                "Notion connection failed: {:?}. The API key may have been revoked.",
                e
            )
        })?;

    Ok("Connected".to_string())
}

/// Remove a Notion integration: deletes the stored credential and the profile JSON.
/// Attempts both deletions even if one fails — both errors are collected and returned together.
/// Missing credential or missing profile file is treated as success (idempotent).
#[tauri::command]
pub async fn remove_notion_integration(integration_id: String) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();

    // Delete the stored credential (token). Missing token is not an error.
    match delete_notion_token(&integration_id) {
        Ok(()) => {}
        Err(e) => {
            // Log a warning but only propagate if the error is not "not found"
            eprintln!(
                "Warning: could not delete Notion token for '{}': {}",
                integration_id, e
            );
            // Only collect as a real error if not a "not found" style message
            if !e.contains("not found") && !e.contains("Not found") {
                errors.push(format!("Token deletion failed: {}", e));
            }
        }
    }

    // Delete the profile file. Missing file is not an error.
    match delete_notion_profile(&integration_id) {
        Ok(()) => {}
        Err(e) => {
            if !e.contains("not found") && !e.contains("No such file") {
                errors.push(format!("Profile deletion failed: {}", e));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Internal helpers for data conversion
// ──────────────────────────────────────────────────────────────────────────────

/// Extract plain-text title from a Notion API JSON value.
/// Database titles are stored as an array of rich text objects.
fn extract_title_text(value: &serde_json::Value) -> String {
    value
        .get("title")
        .and_then(|t| t.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("plain_text"))
        .and_then(|t| t.as_str())
        .unwrap_or("Untitled")
        .to_string()
}

/// Convert a notion-client `DatabaseProperty` to our internal `NotionPropertyDef`.
/// Handles Select/MultiSelect option extraction and maps all property types to
/// a stable string key used by the prompt augmentation system.
fn convert_database_property(name: &str, prop: &DatabaseProperty) -> NotionPropertyDef {
    match prop {
        DatabaseProperty::Title { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "title".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::RichText { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "rich_text".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::Select { select, .. } => {
            let options = select
                .options
                .iter()
                .map(|o| o.name.clone())
                .collect();
            NotionPropertyDef {
                name: name.to_string(),
                property_type: "select".to_string(),
                select_options: options,
            }
        }
        DatabaseProperty::MultiSelect { multi_select, .. } => {
            let options = multi_select
                .options
                .iter()
                .map(|o| o.name.clone())
                .collect();
            NotionPropertyDef {
                name: name.to_string(),
                property_type: "multi_select".to_string(),
                select_options: options,
            }
        }
        DatabaseProperty::People { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "people".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::Date { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "date".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::Number { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "number".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::Checkbox { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "checkbox".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::Url { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "url".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::Email { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "email".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::PhoneNumber { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "phone_number".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::Formula { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "formula".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::Relation { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "relation".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::Rollup { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "rollup".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::CreatedTime { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "created_time".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::LastEditedTime { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "last_edited_time".to_string(),
            select_options: Vec::new(),
        },
        DatabaseProperty::Status { .. } => NotionPropertyDef {
            name: name.to_string(),
            property_type: "status".to_string(),
            select_options: Vec::new(),
        },
        _ => NotionPropertyDef {
            name: name.to_string(),
            property_type: "unknown".to_string(),
            select_options: Vec::new(),
        },
    }
}
