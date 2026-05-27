use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use tokio::sync::Mutex as TokioMutex;

use crate::config::{load_settings, AppSettings, Connection, ConnectionRole, ConnectionType};
use crate::connectors;
use crate::pipelines::{
    load_pipelines, validate_pipeline, PipelineProgressPayload, PipelineState, PipelineStatus,
    PipelineStep, StepStatus,
};
use crate::storage::get_data_dir;
use crate::transcription::{render_transcript_from_json, TranscriptJson};

lazy_static::lazy_static! {
    /// Per-(recording, pipeline) execution lock.
    /// Ensures runs of the same pipeline on the same recording execute sequentially.
    static ref PIPELINE_EXEC_LOCKS: std::sync::Mutex<HashMap<String, Arc<TokioMutex<()>>>> =
        std::sync::Mutex::new(HashMap::new());
}

struct LockMapGuard {
    key: String,
}

impl Drop for LockMapGuard {
    fn drop(&mut self) {
        if let Ok(mut locks) = PIPELINE_EXEC_LOCKS.lock() {
            if let Some(entry) = locks.get(&self.key) {
                if Arc::strong_count(entry) <= 2 {
                    locks.remove(&self.key);
                }
            }
        }
    }
}

/// Known topic heading labels (case-insensitive match for Latin)
const TOPIC_LABELS: &[&str] = &["topic", "тема", "tema"];

/// Default app friendly name used for `{app}` substitution when the recording
/// has none on disk (manual / dictation / pre-refactor metadata).
const DEFAULT_APP_NAME: &str = "NBP";

/// Extract a short title from pipeline step output.
/// Looks for topic section heading, inline topic label, or first content heading.
fn extract_title_from_output(content: &str) -> Option<String> {
    // Skip frontmatter
    let body = if content.starts_with("---") {
        content.splitn(3, "---").nth(2).unwrap_or(content)
    } else {
        content
    };

    let lines: Vec<&str> = body.lines().collect();

    // Pass 1: Look for topic heading — take the next non-empty line as title
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Match: "## Topic", "**Topic**", "**Тема**", "# Topic", etc.
        let heading = trimmed.trim_start_matches('#').trim();
        // Also strip bold markers: **Topic** → Topic
        let clean = heading.trim_matches('*').trim();
        let clean_lower = clean.to_lowercase();

        let is_topic_heading = TOPIC_LABELS.iter().any(|&label| clean_lower == label);

        if is_topic_heading {
            // Check for inline value: "**Topic**: value" or "**Topic:** value"
            // Find if there's text after a colon on the same line
            if let Some(colon_pos) = trimmed.find(':') {
                let after_colon = trimmed[colon_pos + 1..].trim().trim_matches('*').trim();
                if !after_colon.is_empty() {
                    return Some(after_colon.chars().take(100).collect());
                }
            }
            // Otherwise take the next non-empty line
            for next_line in lines.iter().skip(i + 1) {
                let next = next_line.trim();
                if next.is_empty() || next == "---" { continue; }
                let title = next.trim_start_matches(|c: char| c == '-' || c == '*' || c == ' ')
                    .trim_matches(|c: char| c == '*')
                    .trim();
                if !title.is_empty() {
                    return Some(title.chars().take(100).collect());
                }
            }
        }
    }

    // Pass 2: First non-generic heading (skip things like "Summary", "Резюме", etc.)
    const SKIP_HEADINGS: &[&str] = &[
        "summary", "резюме", "краткое резюме", "краткое содержание",
        "краткое саммари разговора", "краткое резюме разговора",
        "краткое содержание разговора", "краткое содержание транскрипта",
        "резюме созвона", "резюме разговора",
    ];
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let heading = trimmed.trim_start_matches('#').trim();
            if !heading.is_empty() && !SKIP_HEADINGS.iter().any(|&s| heading.to_lowercase() == s) {
                return Some(heading.chars().take(100).collect());
            }
        }
    }

    None
}

/// Get the pipeline output directory for a recording
fn get_pipeline_output_dir(recording_id: &str, pipeline_name: &str, run_index: usize) -> PathBuf {
    let dir_name = if run_index == 0 {
        pipeline_name.to_string()
    } else {
        format!("{}.run-{}", pipeline_name, run_index)
    };
    get_data_dir()
        .join(recording_id)
        .join("pipelines")
        .join(dir_name)
}

/// Read the recording's transcript into memory.
///
/// Prefers `transcript.json` (source of truth) rendered to plain text; falls
/// back to legacy `transcript.md` for old recordings. Returns Err when neither
/// exists — caller treats that as "transcribe first".
fn load_transcript_text(recording_id: &str) -> Result<String, String> {
    let dir = get_data_dir().join(recording_id);
    let json_path = dir.join("transcript.json");
    if json_path.exists() {
        let raw = fs::read_to_string(&json_path)
            .map_err(|e| format!("Failed to read transcript.json: {}", e))?;
        let tj: TranscriptJson = serde_json::from_str(&raw)
            .map_err(|e| format!("Failed to parse transcript.json: {}", e))?;
        return Ok(render_transcript_from_json(&tj));
    }
    let md_path = dir.join("transcript.md");
    if md_path.exists() {
        let raw = fs::read_to_string(&md_path)
            .map_err(|e| format!("Failed to read transcript.md: {}", e))?;
        // Strip frontmatter so the placeholder substitutes only the body
        return Ok(connectors::strip_frontmatter(&raw).to_string());
    }
    Err("No transcript found. Transcribe the recording first.".to_string())
}

/// Resolve the friendly app name for the `{app}` placeholder.
///
/// Single source of truth: the call detector writes it into
/// `metadata.app_friendly_name` at recording start; everything else (manual,
/// dictation, pre-refactor metadata) gets the default. We deliberately do NOT
/// resolve bundle_id → friendly_name at run time — the mapping is incomplete
/// and the value is already known at capture time.
fn resolve_app_name(recording_id: &str) -> String {
    crate::storage::read_metadata(recording_id)
        .ok()
        .and_then(|m| m.app_friendly_name.clone())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_APP_NAME.to_string())
}

/// Substitute the three template placeholders. Missing keys silently render
/// as the empty string — see `docs/connections-model.md` I/O contract.
fn render_template(template: &str, transcript: &str, app: &str, processing_result: &str) -> String {
    template
        .replace("{transcript}", transcript)
        .replace("{app}", app)
        .replace("{processing_result}", processing_result)
}

/// Build a `HashMap<id, &Connection>` for O(1) lookup during step dispatch.
fn index_connections(settings: &AppSettings) -> HashMap<String, &Connection> {
    settings
        .connections
        .iter()
        .map(|c| (c.id.clone(), c))
        .collect()
}

/// Result of a single dispatched step.
///
/// `body` is what the engine threads into the next Processing step's
/// `{processing_result}` placeholder. We re-read it from the artifact rather
/// than changing connector signatures to return strings — minimum diff for
/// the legacy connectors (Slack/Notion/Webhook/Save) which keep writing
/// their own `.md` artifact with frontmatter.
struct StepOutcome {
    body: String,
}

/// Dispatch one step to the appropriate connector.
///
/// New-model contract: the engine renders `step.template` into a plain string,
/// merges the Connection's stored config with the injected fields each
/// connector expects (e.g. `prompt` for cli_agent, `integration_id` mapped from
/// the connection id for Slack/Notion), writes the rendered input to a
/// transient sibling file, and hands all of that to the existing connector
/// `execute` function. Each connector still writes its own `output_dir/<step>.md`
/// artifact; we read it back to extract the body for chaining.
#[allow(clippy::too_many_arguments)]
async fn dispatch_step(
    step: &PipelineStep,
    connection: &Connection,
    rendered_input: &str,
    output_dir: &Path,
    pipeline_name: &str,
    recording_id: &str,
) -> Result<StepOutcome, String> {
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    // Transient input file the legacy connectors can read.
    // Hidden (`.<step>.input.txt`) so it doesn't pollute the artifact listing
    // exposed by get_step_outputs (which only iterates `<step>.md`).
    let input_path = output_dir.join(format!(".{}.input.txt", step.name));
    fs::write(&input_path, rendered_input)
        .map_err(|e| format!("Failed to write step input file: {}", e))?;

    // Use the step name itself as the `input` field in artifact frontmatter —
    // it's metadata-only now, no longer wired to data flow. The previous step's
    // name would be ambiguous when the user omits `{processing_result}`.
    let step_input_label = if rendered_input.is_empty() { "empty" } else { "rendered-template" };

    let artifact_path = match step.connection_type {
        ConnectionType::CliAgent => {
            // Engine has already substituted `{transcript}` etc. into rendered_input.
            // Feed it as the entire prompt by injecting `prompt` into the config the
            // connector reads, and pass an empty sibling file so the connector's
            // own `{transcript}` re-substitution is a no-op (rendered already has no placeholders).
            let mut cfg = connection.config.clone();
            ensure_object(&mut cfg);
            cfg.as_object_mut()
                .expect("ensured above")
                .insert("prompt".to_string(), serde_json::Value::String(rendered_input.to_string()));
            // Overwrite any stored prompt_template — the user's template lives at
            // step level now, the connection only carries CLI/model/timeout.
            cfg.as_object_mut()
                .and_then(|o| o.remove("prompt_template"));

            connectors::cli_agent::execute(
                &input_path,
                &cfg,
                output_dir,
                &step.name,
                step_input_label,
                step.description.as_deref(),
            )
            .await?
        }
        ConnectionType::Shell => {
            connectors::shell::execute(
                &input_path,
                &connection.config,
                output_dir,
                &step.name,
                step_input_label,
                step.description.as_deref(),
            )
            .await?
        }
        ConnectionType::Slack => {
            let cfg = with_integration_id(&connection.config, &connection.id);
            connectors::slack::execute(
                &input_path,
                &cfg,
                output_dir,
                &step.name,
                step_input_label,
                step.description.as_deref(),
            )
            .await?
        }
        ConnectionType::Notion => {
            let cfg = with_integration_id(&connection.config, &connection.id);
            connectors::notion::execute_structured(
                &input_path,
                &cfg,
                output_dir,
                &step.name,
                step_input_label,
                step.description.as_deref(),
            )
            .await
            .map_err(|e| e.to_string())?
        }
        ConnectionType::Telegram => {
            let cfg = with_integration_id(&connection.config, &connection.id);
            connectors::telegram::execute(
                &input_path,
                &cfg,
                output_dir,
                &step.name,
                step_input_label,
                step.description.as_deref(),
            )
            .await?
        }
        ConnectionType::Webhook => {
            connectors::webhook::execute(
                &input_path,
                &connection.config,
                output_dir,
                &step.name,
                step_input_label,
                step.description.as_deref(),
            )
            .await?
        }
        ConnectionType::SaveLocal => {
            let cfg = with_integration_id(&connection.config, &connection.id);
            connectors::save::execute(
                &input_path,
                &cfg,
                output_dir,
                &step.name,
                step_input_label,
                step.description.as_deref(),
                pipeline_name,
                recording_id,
            )
            .await?
        }
        ConnectionType::Llm => {
            return Err(format!(
                "Connection type Llm (direct LLM API) is hidden in this UI version. \
                 Step '{}' references a Connection of that type — swap it for a CLI agent.",
                step.name
            ));
        }
    };

    // Read the artifact body back so it can feed the next step's
    // `{processing_result}`. Strip the frontmatter the connector wrote.
    let body = fs::read_to_string(&artifact_path)
        .map(|raw| connectors::strip_frontmatter(&raw).trim().to_string())
        .unwrap_or_default();

    // Clean up the transient input file — only kept for the connector's read.
    let _ = fs::remove_file(&input_path);

    Ok(StepOutcome { body })
}

/// Ensure the JSON value is an object so we can insert into it.
fn ensure_object(v: &mut serde_json::Value) {
    if !v.is_object() {
        *v = serde_json::Value::Object(serde_json::Map::new());
    }
}

/// Bridge for legacy connectors that expect `integration_id` in their step
/// config — pass the Connection's id under that key so their Keychain lookup
/// (`{type}:{integration_id}`) resolves to the token saved under the new
/// `{type}:{connection_id}` scheme (same shape, decision #5 in the spec).
fn with_integration_id(connection_config: &serde_json::Value, connection_id: &str) -> serde_json::Value {
    let mut cfg = connection_config.clone();
    ensure_object(&mut cfg);
    if let Some(obj) = cfg.as_object_mut() {
        obj.insert(
            "integration_id".to_string(),
            serde_json::Value::String(connection_id.to_string()),
        );
    }
    cfg
}

/// Write a `<step>.md` artifact for a skipped step so `get_step_outputs`
/// reports the right status without needing the connector to have run.
fn write_skipped_artifact(
    output_dir: &Path,
    step: &PipelineStep,
    reason: &str,
) {
    let _ = fs::create_dir_all(output_dir);
    let output_path = output_dir.join(format!("{}.md", step.name));
    if output_path.exists() {
        return;
    }
    let now = Utc::now().to_rfc3339();
    let reason_escaped = reason.replace('"', "\\\"");
    let connector_label = format!("{:?}", step.connection_type).to_lowercase();
    let content = format!(
        "---\nname: {}\ndescription: \"{}\"\nconnector: {}\nconnection_id: {}\nstatus: skipped\ncreated_at: {}\ncompleted_at: {}\nerror: \"{}\"\n---\n\n## Skipped\n{}\n",
        step.name,
        step.description.as_deref().unwrap_or("").replace('"', "\\\""),
        connector_label,
        step.connection_id,
        now, now,
        reason_escaped,
        reason,
    );
    let _ = fs::write(&output_path, content);
}

/// Write a failure artifact when the engine itself (not the connector) decides
/// a step can't run — e.g. missing Connection, type mismatch, hidden type.
fn write_engine_failure_artifact(
    output_dir: &Path,
    step: &PipelineStep,
    error: &str,
) {
    let _ = fs::create_dir_all(output_dir);
    let output_path = output_dir.join(format!("{}.md", step.name));
    let now = Utc::now().to_rfc3339();
    let err_escaped = error.replace('"', "\\\"").replace('\n', " ");
    let connector_label = format!("{:?}", step.connection_type).to_lowercase();
    let content = format!(
        "---\nname: {}\ndescription: \"{}\"\nconnector: {}\nconnection_id: {}\nstatus: failed\ncreated_at: {}\ncompleted_at: {}\nerror: \"{}\"\n---\n\n## Error\n{}\n",
        step.name,
        step.description.as_deref().unwrap_or("").replace('"', "\\\""),
        connector_label,
        step.connection_id,
        now, now,
        err_escaped,
        error,
    );
    let _ = fs::write(&output_path, content);
}

/// Execute a pipeline for a recording.
///
/// New-model flow (see `docs/connections-model.md`):
///   1. Load transcript text + app friendly name into memory.
///   2. For each step: resolve its referenced Connection from settings, render
///      `{transcript}` / `{app}` / `{processing_result}` into `step.template`,
///      dispatch to the matching connector, read the artifact body back as the
///      next step's `{processing_result}`.
///   3. Processing-step failure halts downstream; Delivery failure logs and
///      continues with the previous `{processing_result}` unchanged.
pub async fn execute_pipeline_internal(
    recording_id: &str,
    pipeline_name: &str,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<PipelineStatus, String> {
    let key = format!("{}:{}", recording_id, pipeline_name);

    // Serialize execution: if the same pipeline is already running on this recording, wait.
    let lock = {
        let mut locks = PIPELINE_EXEC_LOCKS.lock().unwrap();
        locks.entry(key.clone()).or_insert_with(|| Arc::new(TokioMutex::new(()))).clone()
    };
    let _map_guard = LockMapGuard { key };
    let _exec_guard = lock.lock().await;

    // Load pipeline definition — if deleted since assignment, mark as Partial and bail
    let pipelines = load_pipelines()?;
    let pipeline = match pipelines.get(pipeline_name) {
        Some(p) => p.clone(),
        None => {
            let error_msg = format!("Pipeline '{}' was deleted before execution", pipeline_name);
            eprintln!("[pipeline] {}", error_msg);
            update_pipeline_state(
                recording_id,
                pipeline_name,
                PipelineStatus::Partial,
                None,
                Some(&error_msg),
            )?;
            return Ok(PipelineStatus::Partial);
        }
    };

    validate_pipeline(&pipeline)?;

    // Zero-step pipeline = label only; skip execution, return Done immediately.
    if pipeline.steps.is_empty() {
        update_pipeline_state(recording_id, pipeline_name, PipelineStatus::Done, None, None)?;
        return Ok(PipelineStatus::Done);
    }

    // Load transcript into memory — fail fast if absent.
    let transcript = load_transcript_text(recording_id)?;
    let app_name = resolve_app_name(recording_id);

    // Load Connections once; index by id for O(1) per-step lookup.
    let settings = load_settings();
    let connections = index_connections(&settings);

    // Resolve run_index from the active (waiting/running) pipeline state.
    let run_index = {
        let states = read_pipeline_states(recording_id);
        states.iter()
            .filter(|s| s.name == pipeline_name && s.status != PipelineStatus::Done && s.status != PipelineStatus::Partial)
            .map(|s| s.run_index)
            .last()
            .unwrap_or(0)
    };

    update_pipeline_state(recording_id, pipeline_name, PipelineStatus::Running, None, None)?;

    let output_dir = get_pipeline_output_dir(recording_id, pipeline_name, run_index);
    let total_steps = pipeline.steps.len();

    // Threaded state across the step loop.
    let mut prev_processing_output = String::new();
    let mut failed_or_skipped: HashSet<String> = HashSet::new();
    let mut has_failure = false;
    let mut first_error: Option<String> = None;

    for (i, step) in pipeline.steps.iter().enumerate() {
        // Emit running event before any work so the UI flips the row state.
        emit_progress(app_handle, recording_id, pipeline_name, step, i, total_steps, "running");
        update_pipeline_state(
            recording_id,
            pipeline_name,
            PipelineStatus::Running,
            Some(i),
            None,
        )?;

        // Look up the referenced Connection. Missing / type-mismatched →
        // engine failure with an artifact, then halt-or-continue based on role.
        let conn = match connections.get(&step.connection_id) {
            Some(c) if c.connection_type == step.connection_type => *c,
            Some(c) => {
                let err = format!(
                    "Step '{}': Connection '{}' has type {:?} but step expects {:?}",
                    step.name, c.id, c.connection_type, step.connection_type
                );
                write_engine_failure_artifact(&output_dir, step, &err);
                handle_step_failure(
                    &err,
                    step,
                    i,
                    &pipeline.steps,
                    &mut first_error,
                    &mut has_failure,
                    &mut failed_or_skipped,
                    app_handle,
                    recording_id,
                    pipeline_name,
                    total_steps,
                );
                if step.connection_type.role() == ConnectionRole::Processing {
                    break;
                }
                continue;
            }
            None => {
                let err = format!(
                    "Step '{}': Connection '{}' not found. \
                     Open Settings > Connections to recreate it or remove this step.",
                    step.name, step.connection_id
                );
                write_engine_failure_artifact(&output_dir, step, &err);
                handle_step_failure(
                    &err,
                    step,
                    i,
                    &pipeline.steps,
                    &mut first_error,
                    &mut has_failure,
                    &mut failed_or_skipped,
                    app_handle,
                    recording_id,
                    pipeline_name,
                    total_steps,
                );
                if step.connection_type.role() == ConnectionRole::Processing {
                    break;
                }
                continue;
            }
        };

        // Render template + dispatch.
        let rendered = render_template(&step.template, &transcript, &app_name, &prev_processing_output);
        let step_result = dispatch_step(
            step,
            conn,
            &rendered,
            &output_dir,
            pipeline_name,
            recording_id,
        )
        .await;

        match step_result {
            Ok(outcome) => {
                emit_progress(app_handle, recording_id, pipeline_name, step, i, total_steps, "done");
                if step.connection_type.role() == ConnectionRole::Processing {
                    prev_processing_output = outcome.body;
                }
            }
            Err(err) => {
                handle_step_failure(
                    &err,
                    step,
                    i,
                    &pipeline.steps,
                    &mut first_error,
                    &mut has_failure,
                    &mut failed_or_skipped,
                    app_handle,
                    recording_id,
                    pipeline_name,
                    total_steps,
                );
                if step.connection_type.role() == ConnectionRole::Processing {
                    // Processing failure halts downstream — same as legacy semantics,
                    // since Processing output feeds chained steps.
                    break;
                }
                // Delivery failure: log + continue with prev_processing_output unchanged.
            }
        }
    }

    // Write skipped artifacts for any steps the engine marked but never visited
    // (the loop `break`ed past them on processing failure).
    for step in &pipeline.steps {
        if failed_or_skipped.contains(&step.name) {
            write_skipped_artifact(
                &output_dir,
                step,
                "Skipped because a preceding processing step failed",
            );
        }
    }

    let final_status = if has_failure {
        PipelineStatus::Partial
    } else {
        PipelineStatus::Done
    };

    update_pipeline_state(
        recording_id,
        pipeline_name,
        final_status.clone(),
        None,
        first_error.as_deref(),
    )?;

    // Auto-title: if recording still has default title, extract one from the first
    // processing step's output that succeeded.
    if final_status == PipelineStatus::Done || final_status == PipelineStatus::Partial {
        if let Ok(meta) = crate::storage::read_metadata(recording_id) {
            let needs_title = meta.title.is_empty()
                || meta.title == "Untitled Recording"
                || meta.title.starts_with("Recording ");
            if needs_title {
                if let Some(step) = pipeline.steps.iter().find(|s| {
                    s.connection_type.role() == ConnectionRole::Processing
                        && !failed_or_skipped.contains(&s.name)
                }) {
                    let step_output = output_dir.join(format!("{}.md", step.name));
                    if let Ok(content) = fs::read_to_string(&step_output) {
                        if let Some(title) = extract_title_from_output(&content) {
                            let _ = crate::storage::update_title(recording_id, title);
                        }
                    }
                }
            }
        }
    }

    Ok(final_status)
}

/// Common bookkeeping for a step that just failed: record the error, mark
/// downstream steps as skipped if it's a Processing step, emit the UI event.
#[allow(clippy::too_many_arguments)]
fn handle_step_failure(
    error: &str,
    step: &PipelineStep,
    step_index: usize,
    all_steps: &[PipelineStep],
    first_error: &mut Option<String>,
    has_failure: &mut bool,
    failed_or_skipped: &mut HashSet<String>,
    app_handle: Option<&tauri::AppHandle>,
    recording_id: &str,
    pipeline_name: &str,
    total_steps: usize,
) {
    if first_error.is_none() {
        *first_error = Some(error.to_string());
    }
    *has_failure = true;
    failed_or_skipped.insert(step.name.clone());

    if step.connection_type.role() == ConnectionRole::Processing {
        for remaining in all_steps.iter().skip(step_index + 1) {
            failed_or_skipped.insert(remaining.name.clone());
        }
    }

    emit_progress(
        app_handle,
        recording_id,
        pipeline_name,
        step,
        step_index,
        total_steps,
        "failed",
    );
}

fn emit_progress(
    app_handle: Option<&tauri::AppHandle>,
    recording_id: &str,
    pipeline_name: &str,
    step: &PipelineStep,
    step_index: usize,
    total_steps: usize,
    status: &str,
) {
    if let Some(app) = app_handle {
        use tauri::Emitter;
        let _ = app.emit(
            "pipeline-progress",
            PipelineProgressPayload {
                recording_id: recording_id.to_string(),
                pipeline_name: pipeline_name.to_string(),
                step_name: step.name.clone(),
                step_index,
                total_steps,
                status: status.to_string(),
            },
        );
    }
}

/// Acquire an exclusive file lock using platform-appropriate mechanism.
/// On macOS/Unix, uses flock(2). Returns a guard that releases the lock on drop.
struct FileLockGuard {
    _lock_file: fs::File,
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
        })
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        // Lock is released when _lock_file is dropped (fd closed).
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

    // Find the active (non-completed) entry for this pipeline, or create a new one.
    // Search from the end so we target the most recently assigned run when multiple
    // active entries exist (e.g. a previous run still Running when a new run starts).
    if let Some(state) = pipeline_states.iter_mut().rev().find(|s| s.name == pipeline_name && s.status != PipelineStatus::Done && s.status != PipelineStatus::Partial) {
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
            id: uuid::Uuid::new_v4().to_string(),
            name: pipeline_name.to_string(),
            status,
            run_index: 0,
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

    // This rewrites metadata.json outside storage::write_metadata, so the
    // list_recordings cache won't see the new pipeline state otherwise.
    crate::storage::invalidate_list_cache();
    Ok(())
}

/// Read pipeline states directly from metadata JSON file on disk.
/// This avoids the RecordingMetadata struct (which doesn't include a pipelines field)
/// by reading the raw JSON and extracting the pipeline states.
///
/// Migrates old entries that lack an `id` field: assigns a stable UUID and persists it,
/// so subsequent reads (e.g. delete) can match by the same ID.
fn read_pipeline_states(recording_id: &str) -> Vec<PipelineState> {
    let recording_dir = get_data_dir().join(recording_id);
    let metadata_path = recording_dir.join("metadata.json");

    let Ok(content) = fs::read_to_string(&metadata_path) else { return Vec::new() };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else { return Vec::new() };
    let Some(pipelines) = json.get("pipelines") else { return Vec::new() };

    // Backfill missing `id` fields so serde default doesn't generate a new UUID each read
    if let Some(arr) = pipelines.as_array() {
        let needs_migration = arr.iter().any(|e| {
            e.get("id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty())
        });
        if needs_migration {
            // Acquire lock BEFORE re-reading to avoid overwriting concurrent changes
            let lock_path = recording_dir.join(".metadata.lock");
            if let Ok(_lock) = FileLockGuard::acquire(&lock_path) {
                // Re-read file under lock to get the latest state
                if let Ok(locked_content) = fs::read_to_string(&metadata_path) {
                    if let Ok(mut locked_json) = serde_json::from_str::<serde_json::Value>(&locked_content) {
                        if let Some(locked_arr) = locked_json.get("pipelines").and_then(|v| v.as_array()) {
                            let mut migrated = locked_arr.clone();
                            for entry in &mut migrated {
                                let missing = entry.get("id").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty());
                                if missing {
                                    if let Some(obj) = entry.as_object_mut() {
                                        obj.insert("id".to_string(), serde_json::Value::String(uuid::Uuid::new_v4().to_string()));
                                    }
                                }
                            }
                            locked_json["pipelines"] = serde_json::Value::Array(migrated);
                            if let Ok(updated) = serde_json::to_string_pretty(&locked_json) {
                                let temp_path = metadata_path.with_extension("json.tmp");
                                if fs::write(&temp_path, &updated).is_ok()
                                    && fs::rename(&temp_path, &metadata_path).is_ok()
                                {
                                    crate::storage::invalidate_list_cache();
                                }
                            }
                            // Return states from the locked read (authoritative)
                            return locked_json.get("pipelines")
                                .and_then(|v| serde_json::from_value::<Vec<PipelineState>>(v.clone()).ok())
                                .unwrap_or_default();
                        }
                    }
                }
            }
        }
    }

    json.get("pipelines")
        .and_then(|v| serde_json::from_value::<Vec<PipelineState>>(v.clone()).ok())
        .unwrap_or_default()
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
    run_index: Option<usize>,
) -> Result<Vec<StepStatus>, String> {
    let output_dir = get_pipeline_output_dir(&recording_id, &pipeline_name, run_index.unwrap_or(0));

    if !output_dir.exists() {
        return Ok(Vec::new());
    }

    // Load pipeline definition to get step order
    // If pipeline was deleted, return empty — the pipeline state error field tells the story
    let pipelines = load_pipelines()?;
    let Some(pipeline) = pipelines.get(&pipeline_name) else {
        return Ok(Vec::new());
    };

    let mut statuses = Vec::new();

    for step in &pipeline.steps {
        let step_file = output_dir.join(format!("{}.md", step.name));
        if step_file.exists() {
            let content = fs::read_to_string(&step_file).unwrap_or_default();
            // Parse status, error, and duration from frontmatter
            let (status, error, duration_secs, output) = parse_step_status(&content);
            statuses.push(StepStatus {
                name: step.name.clone(),
                status,
                error,
                duration_secs,
                // augmented_prompt was an N+1 look-ahead artifact (LLM step
                // composing a Notion/Linear schema spec). Gone in the new model
                // — users put any schema hint in their template directly.
                augmented_prompt: None,
                output,
            });
        } else {
            statuses.push(StepStatus {
                name: step.name.clone(),
                status: "pending".to_string(),
                error: None,
                duration_secs: None,
                augmented_prompt: None,
                output: None,
            });
        }
    }

    Ok(statuses)
}

/// Parse step status from frontmatter
/// Returns (status, error, duration_secs, output)
fn parse_step_status(content: &str) -> (String, Option<String>, Option<f64>, Option<String>) {
    if let Some(stripped) = content.strip_prefix("---")
        && let Some(end_idx) = stripped.find("---")
    {
        let frontmatter = &stripped[..end_idx];
        let mut status = "pending".to_string();
        let mut error = None;
        let mut created_at: Option<String> = None;
        let mut completed_at: Option<String> = None;

        for line in frontmatter.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("status:") {
                status = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("error:") {
                let err_val = val.trim();
                if err_val != "null" {
                    error = Some(err_val.trim_matches('"').to_string());
                }
            } else if let Some(val) = line.strip_prefix("created_at:") {
                let ts = val.trim().trim_matches('"').to_string();
                if !ts.is_empty() && ts != "null" {
                    created_at = Some(ts);
                }
            } else if let Some(val) = line.strip_prefix("completed_at:") {
                let ts = val.trim().trim_matches('"').to_string();
                if !ts.is_empty() && ts != "null" {
                    completed_at = Some(ts);
                }
            }
        }

        let duration_secs = match (&created_at, &completed_at) {
            (Some(c), Some(d)) => {
                let start = chrono::DateTime::parse_from_rfc3339(c).ok();
                let end = chrono::DateTime::parse_from_rfc3339(d).ok();
                match (start, end) {
                    (Some(s), Some(e)) => {
                        let dur = e.signed_duration_since(s);
                        Some(dur.num_milliseconds() as f64 / 1000.0)
                    }
                    _ => None,
                }
            }
            _ => None,
        };

        // Extract body content after the closing frontmatter delimiter
        let body_start = end_idx + 3; // skip past "---"
        let body = stripped[body_start..].trim();
        let output = if body.is_empty() { None } else { Some(body.to_string()) };

        return (status, error, duration_secs, output);
    }

    ("pending".to_string(), None, None, None)
}

/// Remove a pipeline run from a recording's metadata and delete its output directory.
#[tauri::command]
pub fn remove_pipeline_run(recording_id: String, run_id: String) -> Result<(), String> {
    let recording_dir = get_data_dir().join(&recording_id);
    let metadata_path = recording_dir.join("metadata.json");
    let lock_path = recording_dir.join(".metadata.lock");

    let _lock = FileLockGuard::acquire(&lock_path)?;

    let content = fs::read_to_string(&metadata_path)
        .map_err(|e| format!("Failed to read metadata: {}", e))?;
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse metadata: {}", e))?;

    // Work with raw JSON array to avoid serde default generating new random UUIDs
    // for legacy entries without persisted `id` fields (which would never match run_id)
    let pipelines_arr = json.get("pipelines")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "No pipelines array in metadata".to_string())?;

    let pos = pipelines_arr.iter().position(|entry| {
        entry.get("id").and_then(|v| v.as_str()) == Some(run_id.as_str())
    });

    let idx = match pos {
        Some(i) => i,
        None => return Err(format!("Pipeline run '{}' not found", run_id)),
    };

    // Extract name and run_index for output dir cleanup before removing
    let removed = &pipelines_arr[idx];
    let removed_name = removed.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let removed_run_index = removed.get("run_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    let mut arr = pipelines_arr.clone();
    arr.remove(idx);
    json["pipelines"] = serde_json::Value::Array(arr);

    let temp_path = metadata_path.with_extension("json.tmp");
    let updated = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
    fs::write(&temp_path, &updated)
        .map_err(|e| format!("Failed to write temp metadata: {}", e))?;
    fs::rename(&temp_path, &metadata_path)
        .map_err(|e| format!("Failed to finalize metadata: {}", e))?;
    crate::storage::invalidate_list_cache();

    // Delete the output directory (best effort)
    let output_dir = get_pipeline_output_dir(&recording_id, &removed_name, removed_run_index);
    let _ = fs::remove_dir_all(&output_dir);

    Ok(())
}

/// Assign a pipeline to a recording (sets status to "waiting")
#[tauri::command]
pub fn assign_pipeline(recording_id: String, pipeline_name: String) -> Result<(), String> {
    // Verify pipeline exists
    let pipelines = load_pipelines()?;
    if !pipelines.contains_key(&pipeline_name) {
        return Err(format!("Pipeline '{}' not found", pipeline_name));
    }

    let recording_dir = get_data_dir().join(&recording_id);
    let metadata_path = recording_dir.join("metadata.json");
    let lock_path = recording_dir.join(".metadata.lock");

    let _lock = FileLockGuard::acquire(&lock_path)?;

    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let content = fs::read_to_string(&metadata_path)
        .map_err(|e| format!("Failed to read metadata: {}", e))?;
    let mut json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse metadata: {}", e))?;

    let mut pipeline_states: Vec<PipelineState> = json
        .get("pipelines")
        .and_then(|v| serde_json::from_value::<Vec<PipelineState>>(v.clone()).ok())
        .unwrap_or_default();

    // Count existing runs of this pipeline to determine run_index
    let run_index = pipeline_states.iter().filter(|s| s.name == pipeline_name).count();

    pipeline_states.push(PipelineState {
        id: uuid::Uuid::new_v4().to_string(),
        name: pipeline_name,
        status: PipelineStatus::Waiting,
        run_index,
        started_at: Some(now),
        completed_at: None,
        current_step: None,
        error: None,
    });

    json["pipelines"] = serde_json::to_value(&pipeline_states)
        .map_err(|e| format!("Failed to serialize pipeline states: {}", e))?;

    let temp_path = metadata_path.with_extension("json.tmp");
    let updated = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
    fs::write(&temp_path, &updated)
        .map_err(|e| format!("Failed to write temp metadata: {}", e))?;
    fs::rename(&temp_path, &metadata_path)
        .map_err(|e| format!("Failed to finalize metadata: {}", e))?;
    crate::storage::invalidate_list_cache();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template_all_three_placeholders() {
        let out = render_template(
            "Hi {app}, transcript is:\n{transcript}\n\nPrev: {processing_result}",
            "hello world",
            "Zoom",
            "summary v1",
        );
        assert!(out.contains("Hi Zoom"));
        assert!(out.contains("hello world"));
        assert!(out.contains("Prev: summary v1"));
    }

    #[test]
    fn test_render_template_missing_keys_render_as_empty() {
        // No-placeholder template — passes through verbatim.
        assert_eq!(render_template("static text", "T", "A", "P"), "static text");
    }

    #[test]
    fn test_render_template_empty_processing_result_for_first_step() {
        let out = render_template("[{processing_result}]", "T", "A", "");
        assert_eq!(out, "[]");
    }

    #[test]
    fn test_parse_step_status_done() {
        let content = "---\nname: test\nstatus: done\nerror: null\n---\n\nContent";
        let (status, error, _duration, output) = parse_step_status(content);
        assert_eq!(status, "done");
        assert!(error.is_none());
        assert_eq!(output.unwrap(), "Content");
    }

    #[test]
    fn test_parse_step_status_failed() {
        let content =
            "---\nname: test\nstatus: failed\nerror: \"API error: 401\"\n---\n\nFailed";
        let (status, error, _duration, output) = parse_step_status(content);
        assert_eq!(status, "failed");
        assert_eq!(error.unwrap(), "API error: 401");
        assert_eq!(output.unwrap(), "Failed");
    }

    #[test]
    fn test_parse_step_status_no_frontmatter() {
        let content = "Plain content without frontmatter";
        let (status, error, _duration, output) = parse_step_status(content);
        assert_eq!(status, "pending");
        assert!(error.is_none());
        assert!(output.is_none());
    }

    #[test]
    fn test_pipeline_status_serialization() {
        let state = PipelineState {
            id: "test-id".to_string(),
            name: "meeting-notes".to_string(),
            status: PipelineStatus::Done,
            run_index: 0,
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
    fn test_pipeline_output_dir_isolation() {
        let dir_a = get_pipeline_output_dir("rec-1", "pipeline-a", 0);
        let dir_b = get_pipeline_output_dir("rec-1", "pipeline-b", 0);
        assert_ne!(dir_a, dir_b);
        assert!(dir_a.to_string_lossy().contains("pipelines/pipeline-a"));
        assert!(dir_b.to_string_lossy().contains("pipelines/pipeline-b"));
    }

    #[test]
    fn test_with_integration_id_overwrites_or_inserts() {
        // Insert into an empty object
        let cfg = with_integration_id(&serde_json::json!({}), "abc-123");
        assert_eq!(cfg["integration_id"], "abc-123");

        // Overwrite existing
        let cfg = with_integration_id(&serde_json::json!({"integration_id": "old"}), "new-456");
        assert_eq!(cfg["integration_id"], "new-456");

        // Preserve other fields
        let cfg = with_integration_id(
            &serde_json::json!({"target": "#general", "thread_ts": "1.2"}),
            "slack-1",
        );
        assert_eq!(cfg["target"], "#general");
        assert_eq!(cfg["thread_ts"], "1.2");
        assert_eq!(cfg["integration_id"], "slack-1");
    }
}
