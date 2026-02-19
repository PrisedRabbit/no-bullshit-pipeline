// Linear integration module — types, profile I/O, GraphQL client, and Tauri commands

use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;

// ──────────────────────────────────────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinearWorkflowState {
    pub id: String,
    pub name: String,
    pub type_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinearLabel {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinearMember {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub email: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinearPriority {
    pub priority: i32,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinearTeamInfo {
    pub id: String,
    pub name: String,
}

/// Full integration profile for a single Linear team connection.
/// Stored as `~/.nbp/integrations/linear-{id}.json`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinearIntegrationProfile {
    pub id: String,
    pub name: String,
    pub team_id: String,
    pub team_name: String,
    #[serde(default)]
    pub workflow_states: Vec<LinearWorkflowState>,
    #[serde(default)]
    pub labels: Vec<LinearLabel>,
    #[serde(default)]
    pub members: Vec<LinearMember>,
    #[serde(default)]
    pub priorities: Vec<LinearPriority>,
    pub synced_at: String,
}

// ──────────────────────────────────────────────────────────────────────────────
// Profile I/O
// ──────────────────────────────────────────────────────────────────────────────

pub fn save_linear_profile(profile: &LinearIntegrationProfile) -> Result<(), String> {
    let dir = crate::config::get_integrations_dir();
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create integrations directory: {}", e))?;

    let path = dir.join(format!("linear-{}.json", profile.id));
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("Failed to serialize Linear profile: {}", e))?;

    fs::write(&path, &json)
        .map_err(|e| format!("Failed to write Linear profile: {}", e))?;

    let mut perms = fs::metadata(&path)
        .map_err(|e| format!("Failed to read profile metadata: {}", e))?
        .permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&path, perms)
        .map_err(|e| format!("Failed to set profile permissions: {}", e))?;

    Ok(())
}

pub fn load_linear_profile(integration_id: &str) -> Result<LinearIntegrationProfile, String> {
    let path = crate::config::get_integrations_dir()
        .join(format!("linear-{}.json", integration_id));

    let data = fs::read_to_string(&path).map_err(|_| {
        format!(
            "Linear integration '{}' profile not found. Sync schema in Settings > Integrations.",
            integration_id
        )
    })?;

    serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse Linear profile '{}': {}", integration_id, e))
}

pub fn delete_linear_profile(integration_id: &str) -> Result<(), String> {
    let path = crate::config::get_integrations_dir()
        .join(format!("linear-{}.json", integration_id));

    fs::remove_file(&path)
        .map_err(|e| format!("Failed to delete Linear profile '{}': {}", integration_id, e))
}

// ──────────────────────────────────────────────────────────────────────────────
// Token helper wrappers (delegate to shared dev-mode-aware helpers in mod.rs)
// ──────────────────────────────────────────────────────────────────────────────

pub fn save_linear_token(integration_id: &str, token: &str) -> Result<(), String> {
    super::save_token(&format!("linear:{}", integration_id), token)
}

pub fn get_linear_token(integration_id: &str) -> Result<String, String> {
    super::get_token(&format!("linear:{}", integration_id))
}

pub fn delete_linear_token(integration_id: &str) -> Result<(), String> {
    super::delete_token(&format!("linear:{}", integration_id))
}

// ──────────────────────────────────────────────────────────────────────────────
// Internal GraphQL helper
// ──────────────────────────────────────────────────────────────────────────────

async fn graphql_request(
    token: &str,
    query: &str,
    variables: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();

    let mut body = serde_json::json!({ "query": query });
    if let Some(vars) = variables {
        body["variables"] = vars;
    }

    let response = client
        .post("https://api.linear.app/graphql")
        .header("Content-Type", "application/json")
        .header("Authorization", token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Linear API request failed: {}", e))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read Linear API response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Linear API returned {}: {}", status, text));
    }

    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse Linear API response: {}", e))?;

    if let Some(errors) = json.get("errors") {
        if let Some(arr) = errors.as_array() {
            if !arr.is_empty() {
                let messages: Vec<String> = arr
                    .iter()
                    .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                    .map(|s| s.to_string())
                    .collect();
                return Err(format!("Linear API errors: {}", messages.join("; ")));
            }
        }
    }

    json.get("data")
        .cloned()
        .ok_or_else(|| "Linear API response missing 'data' field".to_string())
}

// ──────────────────────────────────────────────────────────────────────────────
// Tauri commands
// ──────────────────────────────────────────────────────────────────────────────

/// Validate a Linear API key and create a new integration profile.
/// Returns the new integration ID on success.
#[tauri::command]
pub async fn add_linear_integration(name: String, api_key: String) -> Result<String, String> {
    // Validate by calling the viewer query — if this fails, the key is invalid
    graphql_request(&api_key, "query { viewer { id name email } }", None)
        .await
        .map_err(|e| format!("Invalid Linear API key: {}", e))?;

    let id = uuid::Uuid::new_v4().to_string();

    save_linear_token(&id, &api_key)?;

    let profile = LinearIntegrationProfile {
        id: id.clone(),
        name,
        team_id: String::new(),
        team_name: String::new(),
        workflow_states: Vec::new(),
        labels: Vec::new(),
        members: Vec::new(),
        priorities: Vec::new(),
        synced_at: String::new(),
    };

    save_linear_profile(&profile)?;

    Ok(id)
}

/// Test whether a Linear integration's stored API token is still valid.
/// Returns "Connected" on success, or a descriptive error.
#[tauri::command]
pub async fn test_linear_integration(integration_id: String) -> Result<String, String> {
    let token = get_linear_token(&integration_id)?;

    graphql_request(&token, "query { viewer { id name email } }", None)
        .await
        .map_err(|e| {
            format!(
                "Linear connection failed: {}. The API key may have been revoked.",
                e
            )
        })?;

    Ok("Connected".to_string())
}

/// Remove a Linear integration: deletes the stored credential and the profile JSON.
/// Missing credential or missing profile file is treated as success (idempotent).
#[tauri::command]
pub async fn remove_linear_integration(integration_id: String) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();

    match delete_linear_token(&integration_id) {
        Ok(()) => {}
        Err(e) => {
            if !e.contains("not found") && !e.contains("Not found") {
                errors.push(format!("Token deletion failed: {}", e));
            }
        }
    }

    match delete_linear_profile(&integration_id) {
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

/// List all Linear teams accessible with the integration's API key.
#[tauri::command]
pub async fn list_linear_teams(
    integration_id: String,
) -> Result<Vec<LinearTeamInfo>, String> {
    let token = get_linear_token(&integration_id)?;

    let data = graphql_request(&token, "query { teams { nodes { id name } } }", None).await?;

    let nodes = data
        .get("teams")
        .and_then(|t| t.get("nodes"))
        .and_then(|n| n.as_array())
        .ok_or_else(|| "Unexpected response format from Linear teams query".to_string())?;

    let teams: Vec<LinearTeamInfo> = nodes
        .iter()
        .filter_map(|node| {
            let id = node.get("id")?.as_str()?.to_string();
            let name = node.get("name")?.as_str()?.to_string();
            Some(LinearTeamInfo { id, name })
        })
        .collect();

    if teams.is_empty() {
        return Err("No teams found in your Linear workspace".to_string());
    }

    Ok(teams)
}

/// Fetch team schema (workflow states, labels, members) and persist as integration profile.
/// Priorities are hardcoded (Linear's priority levels are fixed).
#[tauri::command]
pub async fn sync_linear_schema(
    integration_id: String,
    team_id: String,
    team_name: String,
) -> Result<LinearIntegrationProfile, String> {
    let token = get_linear_token(&integration_id)?;

    // Fetch workflow states for team
    let states_data = graphql_request(
        &token,
        "query($teamId: String!) { team(id: $teamId) { states { nodes { id name type } } } }",
        Some(serde_json::json!({ "teamId": team_id })),
    )
    .await?;

    let workflow_states: Vec<LinearWorkflowState> = states_data
        .get("team")
        .and_then(|t| t.get("states"))
        .and_then(|s| s.get("nodes"))
        .and_then(|n| n.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|node| {
            let id = node.get("id")?.as_str()?.to_string();
            let name = node.get("name")?.as_str()?.to_string();
            let type_name = node.get("type")?.as_str()?.to_string();
            Some(LinearWorkflowState { id, name, type_name })
        })
        .collect();

    // Fetch labels for team
    let labels_data = graphql_request(
        &token,
        "query($teamId: String!) { team(id: $teamId) { labels { nodes { id name color } } } }",
        Some(serde_json::json!({ "teamId": team_id })),
    )
    .await?;

    let labels: Vec<LinearLabel> = labels_data
        .get("team")
        .and_then(|t| t.get("labels"))
        .and_then(|l| l.get("nodes"))
        .and_then(|n| n.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|node| {
            let id = node.get("id")?.as_str()?.to_string();
            let name = node.get("name")?.as_str()?.to_string();
            let color = node.get("color")?.as_str()?.to_string();
            Some(LinearLabel { id, name, color })
        })
        .collect();

    // Fetch members for team
    let members_data = graphql_request(
        &token,
        "query($teamId: String!) { team(id: $teamId) { members { nodes { id name displayName email } } } }",
        Some(serde_json::json!({ "teamId": team_id })),
    )
    .await?;

    let members: Vec<LinearMember> = members_data
        .get("team")
        .and_then(|t| t.get("members"))
        .and_then(|m| m.get("nodes"))
        .and_then(|n| n.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|node| {
            let id = node.get("id")?.as_str()?.to_string();
            let name = node.get("name")?.as_str()?.to_string();
            let display_name = node.get("displayName")?.as_str()?.to_string();
            let email = node.get("email").and_then(|e| e.as_str()).map(|s| s.to_string());
            Some(LinearMember { id, name, display_name, email })
        })
        .collect();

    // Priorities are static — hardcoded
    let priorities = vec![
        LinearPriority { priority: 0, label: "No priority".into() },
        LinearPriority { priority: 1, label: "Urgent".into() },
        LinearPriority { priority: 2, label: "High".into() },
        LinearPriority { priority: 3, label: "Medium".into() },
        LinearPriority { priority: 4, label: "Low".into() },
    ];

    // Load existing profile to preserve name field
    let existing_name = match load_linear_profile(&integration_id) {
        Ok(existing) => existing.name,
        Err(_) => team_name.clone(),
    };

    let profile = LinearIntegrationProfile {
        id: integration_id,
        name: existing_name,
        team_id,
        team_name,
        workflow_states,
        labels,
        members,
        priorities,
        synced_at: chrono::Utc::now().to_rfc3339(),
    };

    save_linear_profile(&profile)?;

    Ok(profile)
}

/// List all Linear integration profiles from disk.
#[tauri::command]
pub fn list_linear_profiles() -> Result<Vec<LinearIntegrationProfile>, String> {
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

        if file_name.starts_with("linear-") && file_name.ends_with(".json") {
            let data = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read profile '{}': {}", file_name, e))?;
            let profile: LinearIntegrationProfile = serde_json::from_str(&data)
                .map_err(|e| format!("Failed to parse profile '{}': {}", file_name, e))?;
            profiles.push(profile);
        }
    }

    Ok(profiles)
}
