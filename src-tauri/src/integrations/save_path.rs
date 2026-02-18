// Save path integration module — types, profile I/O, and Tauri commands
//
// Save path profiles are stored as `~/.nbp/integrations/save-path-{id}.json`.
// This follows the exact same file I/O pattern as notion.rs.

use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use uuid::Uuid;

// ──────────────────────────────────────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────────────────────────────────────

/// A named save path integration profile.
/// Stored as `~/.nbp/integrations/save-path-{id}.json`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SavePathProfile {
    pub id: String,
    pub name: String,
    pub path: String,
}

// ──────────────────────────────────────────────────────────────────────────────
// Tauri commands
// ──────────────────────────────────────────────────────────────────────────────

/// Add a new named save path integration.
/// Validates that name and path are non-empty.
/// Returns the new profile ID on success.
#[tauri::command]
pub fn add_save_path_integration(name: String, path: String) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Save path name cannot be empty.".to_string());
    }
    if path.trim().is_empty() {
        return Err("Save path cannot be empty.".to_string());
    }

    let id = Uuid::new_v4().to_string();

    let profile = SavePathProfile {
        id: id.clone(),
        name: name.trim().to_string(),
        path: path.trim().to_string(),
    };

    save_save_path_profile(&profile)?;

    Ok(id)
}

/// List all saved save path integration profiles.
/// Reads all `save-path-*.json` files from the integrations directory.
/// Returns an empty vector if the directory does not exist.
#[tauri::command]
pub fn list_save_path_integrations() -> Result<Vec<SavePathProfile>, String> {
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

        if file_name.starts_with("save-path-") && file_name.ends_with(".json") {
            let data = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read save path profile '{}': {}", file_name, e))?;
            let profile: SavePathProfile = serde_json::from_str(&data)
                .map_err(|e| format!("Failed to parse save path profile '{}': {}", file_name, e))?;
            profiles.push(profile);
        }
    }

    Ok(profiles)
}

/// Update the name and path of an existing save path integration.
/// Returns an error if the profile with the given ID is not found.
#[tauri::command]
pub fn update_save_path_integration(id: String, name: String, path: String) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Save path name cannot be empty.".to_string());
    }
    if path.trim().is_empty() {
        return Err("Save path cannot be empty.".to_string());
    }

    // Load the existing profile to confirm it exists
    let mut profile = load_save_path_profile(&id)?;

    // Update fields
    profile.name = name.trim().to_string();
    profile.path = path.trim().to_string();

    save_save_path_profile(&profile)?;

    Ok(())
}

/// Remove a save path integration profile by ID.
/// Treats "not found" as success (idempotent), matching the pattern of remove_notion_integration.
#[tauri::command]
pub fn remove_save_path_integration(id: String) -> Result<(), String> {
    let file_path = crate::config::get_integrations_dir()
        .join(format!("save-path-{}.json", id));

    match fs::remove_file(&file_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Failed to remove save path profile '{}': {}", id, e)),
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Profile I/O helpers (internal)
// ──────────────────────────────────────────────────────────────────────────────

/// Save a save path profile to disk.
/// Writes to `~/.nbp/integrations/save-path-{id}.json` with permissions 0o600.
fn save_save_path_profile(profile: &SavePathProfile) -> Result<(), String> {
    let dir = crate::config::get_integrations_dir();
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create integrations directory: {}", e))?;

    let file_path = dir.join(format!("save-path-{}.json", profile.id));
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("Failed to serialize save path profile: {}", e))?;

    fs::write(&file_path, &json)
        .map_err(|e| format!("Failed to write save path profile: {}", e))?;

    // Set permissions to 0o600 (user read/write only)
    let mut perms = fs::metadata(&file_path)
        .map_err(|e| format!("Failed to read save path profile metadata: {}", e))?
        .permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&file_path, perms)
        .map_err(|e| format!("Failed to set save path profile permissions: {}", e))?;

    Ok(())
}

/// Load a save path profile from disk by its ID.
fn load_save_path_profile(id: &str) -> Result<SavePathProfile, String> {
    let file_path = crate::config::get_integrations_dir()
        .join(format!("save-path-{}.json", id));

    let data = fs::read_to_string(&file_path).map_err(|_| {
        format!(
            "Save path integration '{}' not found.",
            id
        )
    })?;

    serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse save path profile '{}': {}", id, e))
}
