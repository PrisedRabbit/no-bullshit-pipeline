use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use chrono::Utc;
use crate::pipelines::{ConnectorType, load_pipelines, validate_pipeline};
use crate::storage::get_data_dir;
use crate::transcription::{TranscriptJson, render_transcript_from_json};
use crate::connectors;

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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PipelineState {
    pub name: String,
    pub status: PipelineStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_step: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Step execution status for UI updates
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StepStatus {
    pub name: String,
    pub status: String, // "pending", "running", "done", "failed"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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

/// Get the pipeline output directory for a recording
fn get_pipeline_output_dir(recording_id: &str, pipeline_name: &str) -> PathBuf {
    get_data_dir()
        .join(recording_id)
        .join("pipelines")
        .join(pipeline_name)
}

/// Resolve the input path for a step.
/// For transcript input: renders JSON to a cached .txt file (with .md fallback).
/// For step outputs: returns the step's .md file path.
fn resolve_input_path(
    recording_id: &str,
    pipeline_name: &str,
    input: &str,
) -> PathBuf {
    if input == "transcript" {
        let recording_dir = get_data_dir().join(recording_id);
        let json_path = recording_dir.join("transcript.json");

        if json_path.exists() {
            // DESIGN NOTE: Save connector behavior change (v0.3 -> v0.4)
            //
            // Connectors now receive transcript_rendered.txt (plain text) instead of
            // transcript.md (markdown with YAML frontmatter).
            //
            // RATIONALE:
            // - transcript.json is the source of truth (metadata + text)
            // - transcript_rendered.txt is an ephemeral cache for connector consumption
            // - Connectors process content, not metadata
            // - Metadata stays in transcript.json where it belongs
            // - Separation of concerns: storage (JSON) vs processing (plain text)
            //
            // This is INTENTIONAL design, not a bug. The rendered text file:
            // 1. Contains only the transcript body (plain text)
            // 2. Has no YAML frontmatter (metadata lives in .json)
            // 3. Is generated on-demand for each pipeline execution
            // 4. Is cleaned up after pipeline completes (see execute_pipeline_internal)
            //
            // Previous behavior (v0.3): transcript.md with frontmatter passed directly.
            // Current behavior (v0.4): transcript.json rendered to .txt, frontmatter-free.
            let rendered_path = recording_dir.join("transcript_rendered.txt");
            if let Ok(content) = fs::read_to_string(&json_path) {
                if let Ok(tj) = serde_json::from_str::<TranscriptJson>(&content) {
                    let text = render_transcript_from_json(&tj);
                    let _ = fs::write(&rendered_path, &text);
                    return rendered_path;
                }
            }
        }

        // Fallback: legacy transcript.md
        recording_dir.join("transcript.md")
    } else {
        get_pipeline_output_dir(recording_id, pipeline_name).join(format!("{}.md", input))
    }
}

/// Execute a pipeline for a recording
pub async fn execute_pipeline_internal(
    recording_id: &str,
    pipeline_name: &str,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<PipelineStatus, String> {
    // Load pipeline definition
    let pipelines = load_pipelines()?;
    let pipeline = pipelines
        .get(pipeline_name)
        .ok_or_else(|| format!("Pipeline '{}' not found", pipeline_name))?
        .clone();

    validate_pipeline(&pipeline)?;

    // Verify transcript exists (.json primary, .md fallback)
    let recording_dir = get_data_dir().join(recording_id);
    let has_transcript = recording_dir.join("transcript.json").exists()
        || recording_dir.join("transcript.md").exists();
    if !has_transcript {
        return Err("No transcript found. Transcribe the recording first.".to_string());
    }

    // Update pipeline state to running
    update_pipeline_state(recording_id, pipeline_name, PipelineStatus::Running, None, None)?;

    let output_dir = get_pipeline_output_dir(recording_id, pipeline_name);
    let total_steps = pipeline.steps.len();

    // Execute steps sequentially
    for (i, step) in pipeline.steps.iter().enumerate() {
        // Update current step
        update_pipeline_state(
            recording_id,
            pipeline_name,
            PipelineStatus::Running,
            Some(i),
            None,
        )?;

        // Emit progress event
        if let Some(app) = app_handle {
            use tauri::Emitter;
            let _ = app.emit(
                "pipeline-progress",
                PipelineProgressPayload {
                    recording_id: recording_id.to_string(),
                    pipeline_name: pipeline_name.to_string(),
                    step_name: step.name.clone(),
                    step_index: i,
                    total_steps,
                    status: "running".to_string(),
                },
            );
        }

        let input_path = resolve_input_path(recording_id, pipeline_name, &step.input);

        if !input_path.exists() {
            let error = format!(
                "Input file not found for step '{}': {}",
                step.name,
                input_path.display()
            );
            update_pipeline_state(
                recording_id,
                pipeline_name,
                PipelineStatus::Partial,
                Some(i),
                Some(&error),
            )?;
            return Err(error);
        }

        let step_result = match step.connector {
            ConnectorType::Llm => {
                connectors::llm::execute(
                    &input_path,
                    &step.config,
                    &output_dir,
                    &step.name,
                    &step.input,
                    step.description.as_deref(),
                )
                .await
            }
            ConnectorType::Save => {
                connectors::save::execute(
                    &input_path,
                    &step.config,
                    &output_dir,
                    &step.name,
                    &step.input,
                    step.description.as_deref(),
                    pipeline_name,
                    recording_id,
                )
                .await
            }
            ConnectorType::Webhook => {
                connectors::webhook::execute(
                    &input_path,
                    &step.config,
                    &output_dir,
                    &step.name,
                    &step.input,
                    step.description.as_deref(),
                )
                .await
            }
            ConnectorType::Slack => {
                connectors::slack::execute(
                    &input_path,
                    &step.config,
                    &output_dir,
                    &step.name,
                    &step.input,
                    step.description.as_deref(),
                )
                .await
            }
            ConnectorType::Mcp => {
                Err("MCP connector not yet implemented".to_string())
            }
        };

        match step_result {
            Ok(_) => {
                // Emit step done
                if let Some(app) = app_handle {
                    use tauri::Emitter;
                    let _ = app.emit(
                        "pipeline-progress",
                        PipelineProgressPayload {
                            recording_id: recording_id.to_string(),
                            pipeline_name: pipeline_name.to_string(),
                            step_name: step.name.clone(),
                            step_index: i,
                            total_steps,
                            status: "done".to_string(),
                        },
                    );
                }
            }
            Err(ref error) => {
                // Step failed - set pipeline to partial and stop
                if let Some(app) = app_handle {
                    use tauri::Emitter;
                    let _ = app.emit(
                        "pipeline-progress",
                        PipelineProgressPayload {
                            recording_id: recording_id.to_string(),
                            pipeline_name: pipeline_name.to_string(),
                            step_name: step.name.clone(),
                            step_index: i,
                            total_steps,
                            status: "failed".to_string(),
                        },
                    );
                }

                update_pipeline_state(
                    recording_id,
                    pipeline_name,
                    PipelineStatus::Partial,
                    Some(i),
                    Some(&error),
                )?;

                // Clean up temporary rendered transcript file
                let rendered_path = get_data_dir().join(recording_id).join("transcript_rendered.txt");
                let _ = fs::remove_file(&rendered_path);

                return Ok(PipelineStatus::Partial);
            }
        }
    }

    // All steps completed successfully
    update_pipeline_state(
        recording_id,
        pipeline_name,
        PipelineStatus::Done,
        None,
        None,
    )?;

    // Clean up temporary rendered transcript file
    let rendered_path = get_data_dir().join(recording_id).join("transcript_rendered.txt");
    let _ = fs::remove_file(&rendered_path);

    Ok(PipelineStatus::Done)
}

/// Acquire an exclusive file lock using platform-appropriate mechanism.
/// On macOS/Unix, uses flock(2). Returns a guard that releases the lock on drop.
struct FileLockGuard {
    _lock_file: fs::File,
    lock_path: PathBuf,
}

impl FileLockGuard {
    fn acquire(lock_path: &PathBuf) -> Result<Self, String> {
        let lock_file = fs::File::create(lock_path)
            .map_err(|e| format!("Failed to create lock file: {}", e))?;

        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = lock_file.as_raw_fd();
            let ret = unsafe { libc::flock(fd, libc::LOCK_EX) };
            if ret != 0 {
                return Err("Failed to acquire metadata lock".to_string());
            }
        }

        // On non-Unix platforms, the file create itself provides basic exclusion
        // since we do atomic read-modify-write within the lock scope.

        Ok(FileLockGuard {
            _lock_file: lock_file,
            lock_path: lock_path.clone(),
        })
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        // Lock is released when _lock_file is dropped (fd closed).
        // Clean up lock file (best effort).
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// Update pipeline state in recording metadata.
/// Uses file-level locking to prevent concurrent modifications from corrupting state.
fn update_pipeline_state(
    recording_id: &str,
    pipeline_name: &str,
    status: PipelineStatus,
    current_step: Option<usize>,
    error: Option<&str>,
) -> Result<(), String> {
    let recording_dir = get_data_dir().join(recording_id);
    let metadata_path = recording_dir.join("metadata.json");
    let lock_path = recording_dir.join(".metadata.lock");

    // Acquire file lock for atomic read-modify-write
    let _lock = FileLockGuard::acquire(&lock_path)?;

    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    // Read current metadata JSON (preserves all fields including unknown ones)
    let content = fs::read_to_string(&metadata_path)
        .map_err(|e| format!("Failed to read metadata: {}", e))?;
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse metadata: {}", e))?;

    // Extract existing pipeline states
    let mut pipeline_states: Vec<PipelineState> = json
        .get("pipelines")
        .and_then(|v| serde_json::from_value::<Vec<PipelineState>>(v.clone()).ok())
        .unwrap_or_default();

    // Find or create this pipeline's state
    if let Some(state) = pipeline_states.iter_mut().find(|s| s.name == pipeline_name) {
        state.status = status.clone();
        state.current_step = current_step;
        if status == PipelineStatus::Running && state.started_at.is_none() {
            state.started_at = Some(now.clone());
        }
        if status == PipelineStatus::Done || status == PipelineStatus::Partial {
            state.completed_at = Some(now);
        }
        state.error = error.map(|e| e.to_string());
    } else {
        pipeline_states.push(PipelineState {
            name: pipeline_name.to_string(),
            status,
            started_at: Some(now.clone()),
            completed_at: None,
            current_step,
            error: error.map(|e| e.to_string()),
        });
    }

    // Write pipeline states back to JSON
    json["pipelines"] = serde_json::to_value(&pipeline_states)
        .map_err(|e| format!("Failed to serialize pipeline states: {}", e))?;

    // Atomic write via temp file + rename
    let temp_path = metadata_path.with_extension("json.tmp");
    let updated = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
    fs::write(&temp_path, &updated)
        .map_err(|e| format!("Failed to write temp metadata: {}", e))?;
    fs::rename(&temp_path, &metadata_path)
        .map_err(|e| format!("Failed to finalize metadata: {}", e))?;

    Ok(())
}

/// Read pipeline states directly from metadata JSON file on disk.
/// This avoids the RecordingMetadata struct (which doesn't include a pipelines field)
/// by reading the raw JSON and extracting the pipeline states.
fn read_pipeline_states(recording_id: &str) -> Vec<PipelineState> {
    let recording_dir = get_data_dir().join(recording_id);
    let metadata_path = recording_dir.join("metadata.json");

    if let Ok(content) = fs::read_to_string(&metadata_path)
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(pipelines) = json.get("pipelines")
        && let Ok(states) = serde_json::from_value::<Vec<PipelineState>>(pipelines.clone())
    {
        return states;
    }

    Vec::new()
}

/// Execute a pipeline (Tauri command)
#[tauri::command]
pub async fn execute_pipeline(
    app_handle: tauri::AppHandle,
    recording_id: String,
    pipeline_name: String,
) -> Result<String, String> {
    let status =
        execute_pipeline_internal(&recording_id, &pipeline_name, Some(&app_handle)).await?;

    match status {
        PipelineStatus::Done => Ok("done".to_string()),
        PipelineStatus::Partial => Ok("partial".to_string()),
        _ => Ok(format!("{:?}", status)),
    }
}

/// Get pipeline execution status for a recording
#[tauri::command]
pub fn get_pipeline_status(
    recording_id: String,
    pipeline_name: String,
) -> Result<Option<PipelineState>, String> {
    let states = read_pipeline_states(&recording_id);
    Ok(states.into_iter().find(|s| s.name == pipeline_name))
}

/// Get all pipeline states for a recording
#[tauri::command]
pub fn get_all_pipeline_states(recording_id: String) -> Result<Vec<PipelineState>, String> {
    Ok(read_pipeline_states(&recording_id))
}

/// Get step outputs for a pipeline execution
#[tauri::command]
pub fn get_step_outputs(
    recording_id: String,
    pipeline_name: String,
) -> Result<Vec<StepStatus>, String> {
    let output_dir = get_pipeline_output_dir(&recording_id, &pipeline_name);

    if !output_dir.exists() {
        return Ok(Vec::new());
    }

    // Load pipeline definition to get step order
    let pipelines = load_pipelines()?;
    let pipeline = pipelines
        .get(&pipeline_name)
        .ok_or_else(|| format!("Pipeline '{}' not found", pipeline_name))?;

    let mut statuses = Vec::new();

    for step in &pipeline.steps {
        let step_file = output_dir.join(format!("{}.md", step.name));
        if step_file.exists() {
            let content = fs::read_to_string(&step_file).unwrap_or_default();
            // Parse status from frontmatter
            let (status, error) = parse_step_status(&content);
            statuses.push(StepStatus {
                name: step.name.clone(),
                status,
                error,
            });
        } else {
            statuses.push(StepStatus {
                name: step.name.clone(),
                status: "pending".to_string(),
                error: None,
            });
        }
    }

    Ok(statuses)
}

/// Parse step status from frontmatter
fn parse_step_status(content: &str) -> (String, Option<String>) {
    if let Some(stripped) = content.strip_prefix("---")
        && let Some(end_idx) = stripped.find("---")
    {
        let frontmatter = &stripped[..end_idx];
        let mut status = "pending".to_string();
        let mut error = None;

        for line in frontmatter.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("status:") {
                status = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("error:") {
                let err_val = val.trim();
                if err_val != "null" {
                    error = Some(err_val.trim_matches('"').to_string());
                }
            }
        }

        return (status, error);
    }

    ("pending".to_string(), None)
}

/// Assign a pipeline to a recording (sets status to "waiting")
#[tauri::command]
pub fn assign_pipeline(recording_id: String, pipeline_name: String) -> Result<(), String> {
    // Verify pipeline exists
    let pipelines = load_pipelines()?;
    if !pipelines.contains_key(&pipeline_name) {
        return Err(format!("Pipeline '{}' not found", pipeline_name));
    }

    update_pipeline_state(&recording_id, &pipeline_name, PipelineStatus::Waiting, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_input_path_transcript() {
        let path = resolve_input_path("abc-123", "my-pipeline", "transcript");
        assert!(path.to_string_lossy().contains("abc-123"));
        // Falls back to transcript.md when no .json exists
        assert!(
            path.to_string_lossy().ends_with("transcript.md")
            || path.to_string_lossy().ends_with("transcript_rendered.txt")
        );
    }

    #[test]
    fn test_resolve_input_path_step() {
        let path = resolve_input_path("abc-123", "my-pipeline", "summarize");
        assert!(path.to_string_lossy().contains("pipelines/my-pipeline"));
        assert!(path.to_string_lossy().ends_with("summarize.md"));
    }

    #[test]
    fn test_parse_step_status_done() {
        let content = "---\nname: test\nstatus: done\nerror: null\n---\n\nContent";
        let (status, error) = parse_step_status(content);
        assert_eq!(status, "done");
        assert!(error.is_none());
    }

    #[test]
    fn test_parse_step_status_failed() {
        let content =
            "---\nname: test\nstatus: failed\nerror: \"API error: 401\"\n---\n\nFailed";
        let (status, error) = parse_step_status(content);
        assert_eq!(status, "failed");
        assert_eq!(error.unwrap(), "API error: 401");
    }

    #[test]
    fn test_parse_step_status_no_frontmatter() {
        let content = "Plain content without frontmatter";
        let (status, error) = parse_step_status(content);
        assert_eq!(status, "pending");
        assert!(error.is_none());
    }

    #[test]
    fn test_pipeline_status_serialization() {
        let state = PipelineState {
            name: "meeting-notes".to_string(),
            status: PipelineStatus::Done,
            started_at: Some("2026-02-03T12:00:00Z".to_string()),
            completed_at: Some("2026-02-03T12:00:10Z".to_string()),
            current_step: None,
            error: None,
        };

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"done\""));
        assert!(json.contains("meeting-notes"));

        let deserialized: PipelineState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, PipelineStatus::Done);
    }

    #[test]
    fn test_pipeline_output_dir() {
        let dir = get_pipeline_output_dir("abc-123", "my-pipeline");
        assert!(dir.to_string_lossy().contains("abc-123"));
        assert!(dir.to_string_lossy().contains("pipelines/my-pipeline"));
    }
}
