use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::PathBuf;
use crate::config::{get_config_dir, ConnectionType};
use crate::storage::get_data_dir;

/// A single step in a pipeline.
///
/// New shape (see `docs/connections-model.md`): the step does NOT carry its
/// own auth / target — it picks a pre-built [`Connection`](crate::config::Connection)
/// by id, and writes a template that gets the 3 placeholders substituted by
/// the engine before being handed to the connector. No `input` field — chain
/// is strictly linear (`{processing_result}` = immediately previous processing
/// step's output).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PipelineStep {
    pub name: String,
    /// Determines which connector dispatches this step + which Connections are
    /// eligible to pick in the UI.
    pub connection_type: ConnectionType,
    /// Id of the Connection in `AppSettings.connections`. Empty is a draft
    /// state from the editor; `validate_pipeline` rejects it.
    pub connection_id: String,
    /// Free-form text with `{transcript}` / `{app}` / `{processing_result}`
    /// placeholders. Engine renders before invoking the connector.
    pub template: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Pipeline definition (stored in pipelines.json)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pipeline {
    pub name: String,
    pub description: String,
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
///
/// Cross-file checks (does `connection_id` resolve to a real Connection? does
/// its type match `connection_type`?) are NOT done here — they need
/// `AppSettings` which we don't pull through this layer. The runner does that
/// at execution time and fails the step with a clear "Connection not found"
/// (see `docs/connections-model.md` closed-decision #8).
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

        // Step must reference a Connection.
        if step.connection_id.trim().is_empty() {
            return Err(format!(
                "Step '{}' has no Connection selected. Pick one in the editor or create a {:?} Connection first.",
                step.name, step.connection_type
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
/// Old-shape pipelines (pre `connections-pipelines` refactor) cannot
/// deserialize into the new `PipelineStep` and would explode the whole load.
/// Per decision #7 in `docs/connections-model.md` we wipe instead of
/// migrating — log the parse error and return an empty map so the user lands
/// in a working app and rebuilds via the new editor. Other I/O errors still
/// bubble up.
pub fn load_pipelines() -> Result<HashMap<String, Pipeline>, String> {
    migrate_pipelines_if_needed();
    let path = get_pipelines_path();

    if !path.exists() {
        return Ok(HashMap::new());
    }

    let file = File::open(&path).map_err(|e| format!("Failed to open pipelines.json: {}", e))?;
    match serde_json::from_reader::<_, HashMap<String, Pipeline>>(file) {
        Ok(pipelines) => Ok(pipelines),
        Err(e) => {
            log::warn!(
                "pipelines.json failed to parse with new Connection-based schema ({}); \
                 starting with an empty list. Recreate pipelines via the editor.",
                e
            );
            Ok(HashMap::new())
        }
    }
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

    fn step(name: &str, ct: ConnectionType, conn_id: &str, template: &str) -> PipelineStep {
        PipelineStep {
            name: name.to_string(),
            connection_type: ct,
            connection_id: conn_id.to_string(),
            template: template.to_string(),
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
                step("summarize", ConnectionType::CliAgent, "conn-claude-1", "Summarize: {transcript}"),
                step("save-to-obsidian", ConnectionType::SaveLocal, "conn-save-1", "{processing_result}"),
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
    fn test_step_with_empty_connection_id_fails() {
        let mut pipeline = make_valid_pipeline();
        pipeline.steps[0].connection_id = "".to_string();
        let result = validate_pipeline(&pipeline);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no Connection selected"));
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
                step("step-a", ConnectionType::CliAgent, "c1", "{transcript}"),
                step("step-a", ConnectionType::SaveLocal, "c2", "{processing_result}"),
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
        assert_eq!(deserialized.steps[0].connection_type, ConnectionType::CliAgent);
        assert_eq!(deserialized.steps[1].connection_type, ConnectionType::SaveLocal);
        assert_eq!(deserialized.steps[0].connection_id, "conn-claude-1");
        assert_eq!(deserialized.steps[0].template, "Summarize: {transcript}");
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
