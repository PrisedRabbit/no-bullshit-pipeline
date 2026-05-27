// Connections — CRUD Tauri commands for the unified flat Connection list
// (see `docs/connections-model.md`). Each Connection is fully self-contained
// (type + non-secret config); its secret (if the type wants one) lives in
// the existing Keychain helper under `{type}:{connection_id}`.
//
// This module replaces the scattered `add_*_integration` / `list_*_profiles`
// per-type commands. The old commands stay for now (Phase 2 deletes them
// along with the Models + Integrations tabs).

use serde::{Deserialize, Serialize};

use crate::config::{load_settings, save_settings_to_disk, Connection, ConnectionType};

/// Account-key prefix per ConnectionType for the Keychain helper. Mirrors
/// the existing scheme — see `connectors/telegram.rs` and friends.
fn keychain_prefix(t: &ConnectionType) -> Option<&'static str> {
    match t {
        ConnectionType::Slack => Some("slack"),
        ConnectionType::Notion => Some("notion"),
        ConnectionType::Telegram => Some("telegram"),
        ConnectionType::Webhook => Some("webhook"),
        // Shell / SaveLocal / CliAgent — no secret to store.
        ConnectionType::Shell | ConnectionType::SaveLocal | ConnectionType::CliAgent => None,
    }
}

fn keychain_key(t: &ConnectionType, id: &str) -> Option<String> {
    keychain_prefix(t).map(|p| format!("{}:{}", p, id))
}

/// Pipelines that reference a Connection — surfaced to the UI before a
/// destructive delete so the user can choose force-delete or cancel.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DeleteReport {
    pub referenced_by_pipelines: Vec<String>,
}

/// List every configured Connection in insertion order.
#[tauri::command]
pub fn list_connections() -> Result<Vec<Connection>, String> {
    Ok(load_settings().connections)
}

/// Create or update a Connection.
///
/// - Empty `connection.id` → assign a fresh UUID.
/// - If `token` is `Some` and the type wants a secret, save it in Keychain
///   keyed by `{type}:{id}`. `None` leaves any existing token untouched, so
///   the UI can update non-secret fields without re-prompting the password.
/// - Non-secret fields persist into `settings.json` under `connections`.
#[tauri::command]
pub fn save_connection(
    mut connection: Connection,
    token: Option<String>,
) -> Result<Connection, String> {
    if connection.name.trim().is_empty() {
        return Err("Connection name cannot be empty".to_string());
    }
    if connection.id.trim().is_empty() {
        connection.id = uuid::Uuid::new_v4().to_string();
        connection.created_at =
            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    }

    // Persist secret if provided. Types without a secret silently ignore
    // a stray token (frontend may pass empty/None either way).
    if let (Some(secret), Some(key)) = (token.as_deref().filter(|s| !s.is_empty()),
                                        keychain_key(&connection.connection_type, &connection.id))
    {
        crate::integrations::save_token(&key, secret)?;
    }

    let mut settings = load_settings();
    if let Some(existing) = settings
        .connections
        .iter_mut()
        .find(|c| c.id == connection.id)
    {
        // Preserve original created_at on update; everything else is replaced
        // wholesale (UI sends the full Connection).
        connection.created_at = existing.created_at.clone();
        *existing = connection.clone();
    } else {
        settings.connections.push(connection.clone());
    }
    save_settings_to_disk(&mut settings)?;
    Ok(connection)
}

/// Delete a Connection.
///
/// - `force = false` (default): if any pipeline step references this
///   connection, returns `DeleteReport.referenced_by_pipelines` populated
///   and DOES NOT delete. UI shows the list and asks for confirmation.
/// - `force = true`: delete anyway; pipelines pointing at this id will
///   fail at run time with a clear "Connection not found" error
///   (see `pipeline_engine.rs` — per spec closed decision #8).
///
/// Always removes the secret from Keychain if one existed for this type.
#[tauri::command]
pub fn delete_connection(id: String, force: Option<bool>) -> Result<DeleteReport, String> {
    let force = force.unwrap_or(false);

    let mut settings = load_settings();
    let conn_type = settings
        .connections
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.connection_type.clone())
        .ok_or_else(|| format!("Connection '{}' not found", id))?;

    // Scan pipelines for references.
    let pipelines = crate::pipelines::load_pipelines().unwrap_or_default();
    let referenced_by_pipelines: Vec<String> = pipelines
        .iter()
        .filter_map(|(name, pl)| {
            if pl.steps.iter().any(|s| s.connection_id == id) {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();

    if !referenced_by_pipelines.is_empty() && !force {
        return Ok(DeleteReport {
            referenced_by_pipelines,
        });
    }

    // Drop the connection + best-effort delete the Keychain secret.
    settings.connections.retain(|c| c.id != id);
    save_settings_to_disk(&mut settings)?;

    if let Some(key) = keychain_key(&conn_type, &id) {
        // Best-effort — not finding a token is fine (types without secrets,
        // or partially-set-up entries).
        let _ = crate::integrations::delete_token(&key);
    }

    Ok(DeleteReport {
        referenced_by_pipelines,
    })
}

/// Lightweight per-type connectivity check.
///
/// Returns a human-readable success label on success; surfaces the underlying
/// error as `Err`. Types without a meaningful test (Shell, SaveLocal,
/// CliAgent, Webhook — webhook would need an arbitrary POST, deferred) report
/// "no remote test available" but don't fail.
#[tauri::command]
pub async fn test_connection(id: String) -> Result<String, String> {
    let settings = load_settings();
    let conn = settings
        .connections
        .iter()
        .find(|c| c.id == id)
        .cloned()
        .ok_or_else(|| format!("Connection '{}' not found", id))?;

    match conn.connection_type {
        ConnectionType::Telegram => {
            let token = crate::integrations::get_token(&format!("telegram:{}", id))
                .map_err(|e| format!("Bot token missing in Keychain: {}", e))?;
            let url = format!("https://api.telegram.org/bot{}/getMe", token);
            let resp = reqwest::Client::new()
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Network error: {}", e))?;
            #[derive(serde::Deserialize)]
            struct GetMe {
                ok: bool,
                #[serde(default)]
                description: Option<String>,
                #[serde(default)]
                result: Option<serde_json::Value>,
            }
            let parsed: GetMe = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse Telegram response: {}", e))?;
            if !parsed.ok {
                return Err(parsed
                    .description
                    .unwrap_or_else(|| "Telegram getMe returned ok=false".to_string()));
            }
            let username = parsed
                .result
                .as_ref()
                .and_then(|v| v.get("username"))
                .and_then(|v| v.as_str())
                .unwrap_or("(no username)");
            Ok(format!("Connected as @{}", username))
        }
        ConnectionType::Slack => {
            // auth.test confirms the token is valid + returns the workspace name.
            let token = crate::integrations::get_token(&format!("slack:{}", id))
                .map_err(|e| format!("Slack token missing in Keychain: {}", e))?;
            let resp = reqwest::Client::new()
                .post("https://slack.com/api/auth.test")
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .map_err(|e| format!("Network error: {}", e))?;
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse Slack response: {}", e))?;
            if body.get("ok") != Some(&serde_json::Value::Bool(true)) {
                let err = body.get("error").and_then(|v| v.as_str()).unwrap_or("auth.test failed");
                return Err(err.to_string());
            }
            let team = body.get("team").and_then(|v| v.as_str()).unwrap_or("(unknown workspace)");
            Ok(format!("Connected to {}", team))
        }
        ConnectionType::Notion => {
            // users.me — cheap + confirms the integration token is valid.
            let token = crate::integrations::get_token(&format!("notion:{}", id))
                .map_err(|e| format!("Notion token missing in Keychain: {}", e))?;
            let resp = reqwest::Client::new()
                .get("https://api.notion.com/v1/users/me")
                .header("Authorization", format!("Bearer {}", token))
                .header("Notion-Version", "2022-06-28")
                .send()
                .await
                .map_err(|e| format!("Network error: {}", e))?;
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("Notion {} — {}", status, body));
            }
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse Notion response: {}", e))?;
            let name = body
                .get("bot")
                .and_then(|b| b.get("workspace_name"))
                .and_then(|v| v.as_str())
                .or_else(|| body.get("name").and_then(|v| v.as_str()))
                .unwrap_or("(workspace name not reported)");
            Ok(format!("Connected as {}", name))
        }
        ConnectionType::Shell
        | ConnectionType::SaveLocal
        | ConnectionType::CliAgent
        | ConnectionType::Webhook => Ok("No remote test for this type — config is local.".to_string()),
    }
}
