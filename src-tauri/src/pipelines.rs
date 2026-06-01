use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use crate::config::{get_config_dir, StepType};
use crate::storage::get_data_dir;

/// A single step in a pipeline.
///
/// Self-contained: the step carries its own type + inline config (CLI binary
/// & model, or shell cwd/env/timeout). No separate Connection object. The
/// chain is strictly linear — `{processing_result}` is the immediately
/// previous step's output; step 1 sees an empty `{processing_result}`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PipelineStep {
    pub name: String,
    /// Which connector dispatches this step.
    #[serde(alias = "connection_type")]
    pub step_type: StepType,
    /// Free-form text with `{transcript}` / `{app}` / `{processing_result}`
    /// placeholders (CLI agent) or a bash script body (Shell). Engine renders
    /// placeholders for CLI; Shell gets raw values via NBP_* env vars.
    pub template: String,
    /// Inline non-secret config for this step's connector.
    ///   CLI:   { cli, model?, timeout_secs?, working_directory? }
    ///   Shell: { cwd, shell?, env?, timeout_secs? }
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Pipeline definition (stored in pipelines.json)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pipeline {
    pub name: String,
    pub description: String,
    /// Defaulted so a legacy entry missing `steps` loads as a zero-step
    /// (label-only) pipeline instead of being dropped by the loose loader.
    #[serde(default)]
    pub steps: Vec<PipelineStep>,
    /// When true, this pipeline runs automatically after every recording
    /// finishes transcribing (in addition to any explicitly-assigned pipelines).
    #[serde(default)]
    pub auto_run: bool,
    #[serde(default = "default_now")]
    pub created_at: String,
    #[serde(default = "default_now")]
    pub updated_at: String,
}

fn default_now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Pipeline execution status
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
    Waiting,  // Assigned but transcript not ready
    Running,  // Currently executing
    Done,     // All steps completed successfully
    Partial,  // Stopped due to step failure
}

/// Pipeline execution state stored in recording metadata
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PipelineState {
    /// Unique run ID (UUID)
    #[serde(default = "generate_run_id")]
    pub id: String,
    pub name: String,
    pub status: PipelineStatus,
    #[serde(default)]
    pub run_index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_step: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn generate_run_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Step execution status for UI updates
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StepStatus {
    pub name: String,
    pub status: String, // "pending", "running", "done", "failed", "skipped"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub augmented_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

/// Pipeline progress event payload
#[derive(Serialize, Clone, Debug)]
pub struct PipelineProgressPayload {
    pub recording_id: String,
    pub pipeline_name: String,
    pub step_name: String,
    pub step_index: usize,
    pub total_steps: usize,
    pub status: String,
}

/// Get the path to pipelines.json
fn get_pipelines_path() -> PathBuf {
    get_config_dir().join("pipelines.json")
}

/// Migrate pipelines.json from old data dir to config dir (one-time)
fn migrate_pipelines_if_needed() {
    let old_path = get_data_dir().join("pipelines.json");
    let new_path = get_config_dir().join("pipelines.json");
    if old_path.exists() && !new_path.exists() {
        let _ = fs::rename(&old_path, &new_path);
    }
}

/// Validate a pipeline definition.
pub fn validate_pipeline(pipeline: &Pipeline) -> Result<(), String> {
    // Pipeline must have non-empty name
    if pipeline.name.trim().is_empty() {
        return Err("Pipeline name cannot be empty".to_string());
    }

    // Pipeline name must be filesystem-safe (no slashes, colons, null bytes)
    if pipeline.name.contains('/') || pipeline.name.contains('\\')
        || pipeline.name.contains('\0') || pipeline.name.contains(':')
    {
        return Err("Pipeline name contains invalid characters (/, \\, :, or null)".to_string());
    }

    let mut defined_steps: Vec<String> = Vec::new();

    for (i, step) in pipeline.steps.iter().enumerate() {
        // Step must have non-empty name
        if step.name.trim().is_empty() {
            return Err(format!("Step {} has empty name", i + 1));
        }

        // Step name must be filesystem-safe
        if step.name.contains('/') || step.name.contains('\\')
            || step.name.contains('\0') || step.name.contains(':')
        {
            return Err(format!(
                "Step '{}' name contains invalid characters (/, \\, :, or null)",
                step.name
            ));
        }

        // Check for duplicate step names
        if defined_steps.contains(&step.name) {
            return Err(format!("Duplicate step name '{}'", step.name));
        }

        defined_steps.push(step.name.clone());
    }

    Ok(())
}

/// Load all pipelines from disk.
///
/// Legacy migration (Option A): pre-simplification pipelines carried steps of
/// now-removed delivery types (notion / slack / telegram / webhook /
/// save_local) and a `connection_id` field. We parse loosely first and drop
/// any step whose type isn't a current [`StepType`] (`cli_agent` / `shell`) —
/// the leftover cli/shell steps survive, unknown fields like `connection_id`
/// are ignored by serde. A pipeline that still won't parse is skipped (logged)
/// rather than nuking the whole file. I/O errors still bubble up.
pub fn load_pipelines() -> Result<HashMap<String, Pipeline>, String> {
    migrate_pipelines_if_needed();
    let path = get_pipelines_path();

    if !path.exists() {
        return Ok(HashMap::new());
    }

    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read pipelines.json: {}", e))?;

    // Loose parse so one bad pipeline doesn't take down the rest.
    let map: HashMap<String, serde_json::Value> = match serde_json::from_str(&raw) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("pipelines.json is not a valid object ({}); starting with an empty list.", e);
            return Ok(HashMap::new());
        }
    };

    let mut pipelines = HashMap::new();
    for (name, value) in map {
        match prune_and_parse_pipeline(value) {
            Ok(p) => {
                pipelines.insert(name, p);
            }
            Err(e) => {
                log::warn!("Skipping pipeline '{}' — failed to parse ({}).", name, e);
            }
        }
    }
    Ok(pipelines)
}

/// Drop steps of removed networked-delivery types (notion / slack / telegram /
/// webhook) from a loosely-parsed pipeline value, then deserialize. The step
/// type lives under `step_type` (new) or `connection_type` (legacy alias);
/// only the current [`StepType`]s survive (`cli_agent` / `shell` /
/// `save_local`). Legacy `save_local` steps carried their folder on a
/// Connection, so they load with empty config — the user re-picks the folder.
fn prune_and_parse_pipeline(mut value: serde_json::Value) -> Result<Pipeline, serde_json::Error> {
    if let Some(steps) = value.get_mut("steps").and_then(|v| v.as_array_mut()) {
        steps.retain(|s| {
            let ty = s
                .get("step_type")
                .or_else(|| s.get("connection_type"))
                .and_then(|v| v.as_str());
            matches!(ty, Some("cli_agent") | Some("shell") | Some("save_local"))
        });
    }
    serde_json::from_value::<Pipeline>(value)
}

/// Save all pipelines to disk
pub fn save_pipelines_to_disk(pipelines: &HashMap<String, Pipeline>) -> Result<(), String> {
    let config_dir = get_config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }

    let path = get_pipelines_path();
    let content =
        serde_json::to_string_pretty(pipelines).map_err(|e| format!("Failed to serialize pipelines: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write pipelines.json: {}", e))?;

    Ok(())
}

/// List all pipeline definitions
#[tauri::command]
pub fn list_pipelines() -> Result<Vec<Pipeline>, String> {
    let pipelines = load_pipelines()?;
    let mut list: Vec<Pipeline> = pipelines.into_values().collect();
    // HashMap iteration order is non-deterministic and shifts whenever the map
    // is mutated (e.g. after save_pipeline), which made the list reshuffle on
    // every reload — toggling one pipeline's auto-run looked like it changed a
    // different row. Sort by name for a stable, predictable order.
    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(list)
}

/// Get a specific pipeline by name
#[tauri::command]
pub fn get_pipeline(name: String) -> Result<Pipeline, String> {
    let pipelines = load_pipelines()?;
    pipelines
        .get(&name)
        .cloned()
        .ok_or_else(|| format!("Pipeline '{}' not found", name))
}

/// Save (create or update) a pipeline definition
#[tauri::command]
pub fn save_pipeline(app: tauri::AppHandle, mut pipeline: Pipeline) -> Result<(), String> {
    validate_pipeline(&pipeline)?;

    let mut pipelines = load_pipelines()?;
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    // Preserve created_at if updating existing pipeline
    if let Some(existing) = pipelines.get(&pipeline.name) {
        pipeline.created_at = existing.created_at.clone();
    } else {
        pipeline.created_at = now.clone();
    }
    pipeline.updated_at = now;
    pipelines.insert(pipeline.name.clone(), pipeline);
    save_pipelines_to_disk(&pipelines)?;

    // Live-update tray submenu so the "Record" list reflects the change
    // without an app restart.
    crate::refresh_tray_menu(&app);
    Ok(())
}

/// Delete a pipeline definition
#[tauri::command]
pub fn delete_pipeline(app: tauri::AppHandle, name: String) -> Result<(), String> {
    let mut pipelines = load_pipelines()?;
    if pipelines.remove(&name).is_none() {
        return Err(format!("Pipeline '{}' not found", name));
    }
    save_pipelines_to_disk(&pipelines)?;

    crate::refresh_tray_menu(&app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(name: &str, st: StepType, template: &str) -> PipelineStep {
        PipelineStep {
            name: name.to_string(),
            step_type: st,
            template: template.to_string(),
            config: serde_json::Value::Null,
            description: None,
        }
    }

    fn make_valid_pipeline() -> Pipeline {
        Pipeline {
            name: "test-pipeline".to_string(),
            description: "A test pipeline".to_string(),
            auto_run: false,
            created_at: String::new(),
            updated_at: String::new(),
            steps: vec![
                step("summarize", StepType::CliAgent, "Summarize: {transcript}"),
                step("format", StepType::Shell, "echo \"$NBP_PROCESSING_RESULT\""),
            ],
        }
    }

    #[test]
    fn test_valid_pipeline_passes_validation() {
        let pipeline = make_valid_pipeline();
        assert!(validate_pipeline(&pipeline).is_ok());
    }

    #[test]
    fn test_empty_name_fails() {
        let mut pipeline = make_valid_pipeline();
        pipeline.name = "".to_string();
        let result = validate_pipeline(&pipeline);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("name cannot be empty"));
    }

    #[test]
    fn test_empty_steps_passes() {
        let mut pipeline = make_valid_pipeline();
        pipeline.steps = vec![];
        let result = validate_pipeline(&pipeline);
        assert!(result.is_ok(), "Zero-step pipelines should be valid (labels)");
    }

    #[test]
    fn test_step_with_empty_name_fails() {
        let mut pipeline = make_valid_pipeline();
        pipeline.steps[0].name = "".to_string();
        let result = validate_pipeline(&pipeline);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty name"));
    }

    #[test]
    fn test_duplicate_step_names_fails() {
        let pipeline = Pipeline {
            name: "dup-pipeline".to_string(),
            description: "test".to_string(),
            auto_run: false,
            created_at: String::new(),
            updated_at: String::new(),
            steps: vec![
                step("step-a", StepType::CliAgent, "{transcript}"),
                step("step-a", StepType::Shell, "{processing_result}"),
            ],
        };
        let result = validate_pipeline(&pipeline);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Duplicate step name"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let pipeline = make_valid_pipeline();
        let json = serde_json::to_string_pretty(&pipeline).unwrap();
        let deserialized: Pipeline = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, pipeline.name);
        assert_eq!(deserialized.steps.len(), pipeline.steps.len());
        assert_eq!(deserialized.steps[0].step_type, StepType::CliAgent);
        assert_eq!(deserialized.steps[1].step_type, StepType::Shell);
        assert_eq!(deserialized.steps[0].template, "Summarize: {transcript}");
    }

    #[test]
    fn test_legacy_connection_type_alias_deserializes() {
        // Old pipelines.json used `connection_type`; serde alias keeps them readable.
        let json = r#"{
            "name": "legacy",
            "description": "",
            "steps": [
                { "name": "s1", "connection_type": "cli_agent", "template": "{transcript}" }
            ]
        }"#;
        let p: Pipeline = serde_json::from_str(json).unwrap();
        assert_eq!(p.steps[0].step_type, StepType::CliAgent);
    }

    #[test]
    fn test_prune_drops_legacy_delivery_steps_keeps_cli_shell() {
        // Legacy pipeline mixing a CLI step (connection_type alias + dead
        // connection_id field) with a Notion delivery step. The Notion step
        // must be dropped; the CLI step survives with its template intact.
        let json = serde_json::json!({
            "name": "mixed",
            "description": "",
            "steps": [
                { "name": "summarize", "connection_type": "cli_agent", "connection_id": "dead-id", "template": "{transcript}" },
                { "name": "to-notion", "connection_type": "notion", "connection_id": "abc", "template": "{processing_result}" },
                { "name": "post", "step_type": "shell", "template": "echo hi" }
            ]
        });
        let p = prune_and_parse_pipeline(json).unwrap();
        assert_eq!(p.steps.len(), 2, "notion step should be dropped");
        assert_eq!(p.steps[0].name, "summarize");
        assert_eq!(p.steps[0].step_type, StepType::CliAgent);
        assert_eq!(p.steps[0].template, "{transcript}");
        assert_eq!(p.steps[1].name, "post");
        assert_eq!(p.steps[1].step_type, StepType::Shell);
    }

    #[test]
    fn test_pipeline_name_with_slash_fails() {
        let mut pipeline = make_valid_pipeline();
        pipeline.name = "bad/name".to_string();
        let result = validate_pipeline(&pipeline);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid characters"));
    }
}
