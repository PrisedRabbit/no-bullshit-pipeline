use std::path::{Path, PathBuf};
use std::fmt;
use std::fs;
use chrono::Utc;
use crate::integrations::linear::{
    load_linear_profile, get_linear_token, LinearIntegrationProfile,
};

// ──────────────────────────────────────────────────────────────────────────────
// Error types
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum LinearErrorKind {
    /// JSON extraction or structural validation failed.
    /// `raw_output` contains the complete LLM output (not truncated).
    JsonParse { message: String, raw_output: String },
    /// API errors, config errors, authentication failures, etc.
    Other(String),
}

#[derive(Debug)]
pub struct LinearError {
    pub kind: LinearErrorKind,
}

impl fmt::Display for LinearError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            LinearErrorKind::JsonParse { message, .. } => write!(f, "{}", message),
            LinearErrorKind::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<LinearError> for String {
    fn from(e: LinearError) -> String {
        e.to_string()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Config
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct LinearConnectorConfig {
    integration_id: String,
}

impl LinearConnectorConfig {
    fn from_value(config: &serde_json::Value) -> Result<Self, LinearError> {
        let integration_id = config
            .get("integration_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LinearError {
                kind: LinearErrorKind::Other(
                    "Linear connector config missing 'integration_id'. \
                     Add integration_id to the step config in the pipeline definition."
                        .to_string(),
                ),
            })?
            .to_string();
        Ok(LinearConnectorConfig { integration_id })
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// JSON extraction from LLM output (single object, not array)
// ──────────────────────────────────────────────────────────────────────────────

fn extract_json_object(content: &str) -> Result<serde_json::Value, LinearError> {
    let body = crate::connectors::strip_frontmatter(content);
    let trimmed = body.trim();

    // Try direct parse (bare JSON object)
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if val.is_object() {
            return Ok(val);
        }
    }

    // Try extracting from ```json ... ``` fence
    if let Some(fence_start) = trimmed.find("```json") {
        let after_fence = &trimmed[fence_start + 7..];
        if let Some(fence_end) = after_fence.find("```") {
            let json_str = after_fence[..fence_end].trim();
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                if val.is_object() {
                    return Ok(val);
                }
            }
        }
    }

    // Try extracting from ``` (no language tag) fence
    if let Some(fence_start) = trimmed.find("```\n") {
        let after_fence = &trimmed[fence_start + 4..];
        if let Some(fence_end) = after_fence.find("```") {
            let json_str = after_fence[..fence_end].trim();
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                if val.is_object() {
                    return Ok(val);
                }
            }
        }
    }

    let preview = &trimmed[..trimmed.len().min(500)];
    Err(LinearError {
        kind: LinearErrorKind::JsonParse {
            message: format!(
                "Linear connector: could not parse LLM output as JSON object.\n\
                 Expected a JSON object like: {{\"title\": \"...\", ...}}\n\
                 Raw LLM output (first 500 chars): {}",
                preview
            ),
            raw_output: trimmed.to_string(),
        },
    })
}

// ──────────────────────────────────────────────────────────────────────────────
// LLM output validation
// ──────────────────────────────────────────────────────────────────────────────

const LINEAR_KNOWN_FIELDS: &[&str] = &[
    "title", "description", "priority", "status", "labels", "assignee",
];

fn validate_llm_output_for_linear(
    obj: &serde_json::Value,
    _profile: &LinearIntegrationProfile,
    raw_output: &str,
) -> Result<(), LinearError> {
    let map = match obj.as_object() {
        Some(m) => m,
        None => {
            return Err(LinearError {
                kind: LinearErrorKind::JsonParse {
                    message: format!(
                        "Linear connector: parsed value is not a JSON object.\n\
                         Raw LLM output (first 500 chars): {}",
                        &raw_output[..raw_output.len().min(500)]
                    ),
                    raw_output: raw_output.to_string(),
                },
            });
        }
    };

    let has_known_field = map
        .keys()
        .any(|k| LINEAR_KNOWN_FIELDS.contains(&k.to_lowercase().as_str()));

    if !has_known_field {
        return Err(LinearError {
            kind: LinearErrorKind::JsonParse {
                message: format!(
                    "Linear connector: JSON object has no keys matching known Linear fields.\n\
                     Expected at least one of: {}\n\
                     Got keys: {}\n\
                     Raw LLM output (first 500 chars): {}",
                    LINEAR_KNOWN_FIELDS.join(", "),
                    map.keys().cloned().collect::<Vec<_>>().join(", "),
                    &raw_output[..raw_output.len().min(500)]
                ),
                raw_output: raw_output.to_string(),
            },
        });
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Field resolution functions (case-insensitive matching against profile data)
// ──────────────────────────────────────────────────────────────────────────────

fn resolve_status(value: &str, profile: &LinearIntegrationProfile) -> Option<String> {
    profile
        .workflow_states
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(value))
        .map(|s| s.id.clone())
}

fn resolve_priority(value: &serde_json::Value, profile: &LinearIntegrationProfile) -> Option<i32> {
    if let Some(s) = value.as_str() {
        // String label like "High"
        return profile
            .priorities
            .iter()
            .find(|p| p.label.eq_ignore_ascii_case(s))
            .map(|p| p.priority);
    }
    if let Some(n) = value.as_i64() {
        // Numeric priority 0-4
        if (0..=4).contains(&n) {
            return Some(n as i32);
        }
    }
    if let Some(n) = value.as_f64() {
        let int_val = n as i64;
        if (0..=4).contains(&int_val) {
            return Some(int_val as i32);
        }
    }
    None
}

fn resolve_labels(value: &serde_json::Value, profile: &LinearIntegrationProfile) -> Vec<String> {
    let label_names: Vec<&str> = match value {
        serde_json::Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
        serde_json::Value::String(s) => vec![s.as_str()],
        _ => return Vec::new(),
    };

    label_names
        .iter()
        .filter_map(|name| {
            profile
                .labels
                .iter()
                .find(|l| l.name.eq_ignore_ascii_case(name))
                .map(|l| l.id.clone())
        })
        .collect()
}

fn resolve_assignee(value: &str, profile: &LinearIntegrationProfile) -> Option<String> {
    profile
        .members
        .iter()
        .find(|m| {
            m.name.eq_ignore_ascii_case(value) || m.display_name.eq_ignore_ascii_case(value)
        })
        .map(|m| m.id.clone())
}

// ──────────────────────────────────────────────────────────────────────────────
// GraphQL mutation builder
// ──────────────────────────────────────────────────────────────────────────────

fn build_issue_input(
    obj: &serde_json::Value,
    profile: &LinearIntegrationProfile,
) -> Result<serde_json::Value, LinearError> {
    let map = obj.as_object().ok_or_else(|| LinearError {
        kind: LinearErrorKind::Other(
            "JSON item is not an object — expected a JSON object with Linear issue fields"
                .to_string(),
        ),
    })?;

    let title = map
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LinearError {
            kind: LinearErrorKind::Other(
                "Linear connector: JSON object missing required 'title' field".to_string(),
            ),
        })?;

    let mut input = serde_json::json!({
        "teamId": profile.team_id,
        "title": title,
    });

    if let Some(desc) = map.get("description").and_then(|v| v.as_str()) {
        input["description"] = serde_json::Value::String(desc.to_string());
    }

    if let Some(status_val) = map.get("status").and_then(|v| v.as_str()) {
        if let Some(state_id) = resolve_status(status_val, profile) {
            input["stateId"] = serde_json::Value::String(state_id);
        }
    }

    if let Some(priority_val) = map.get("priority") {
        if let Some(priority_int) = resolve_priority(priority_val, profile) {
            input["priority"] = serde_json::json!(priority_int);
        }
    }

    if let Some(labels_val) = map.get("labels") {
        let label_ids = resolve_labels(labels_val, profile);
        if !label_ids.is_empty() {
            input["labelIds"] = serde_json::json!(label_ids);
        }
    }

    if let Some(assignee_val) = map.get("assignee").and_then(|v| v.as_str()) {
        if let Some(assignee_id) = resolve_assignee(assignee_val, profile) {
            input["assigneeId"] = serde_json::Value::String(assignee_id);
        }
    }

    Ok(input)
}

// ──────────────────────────────────────────────────────────────────────────────
// GraphQL mutation execution
// ──────────────────────────────────────────────────────────────────────────────

struct CreatedIssue {
    id: String,
    identifier: String,
    title: String,
    url: String,
}

async fn create_issue(
    token: &str,
    input: serde_json::Value,
) -> Result<CreatedIssue, LinearError> {
    let query = r#"mutation IssueCreate($input: IssueCreateInput!) {
        issueCreate(input: $input) {
            success
            issue {
                id
                identifier
                title
                url
            }
        }
    }"#;

    let variables = serde_json::json!({ "input": input });

    let data = crate::integrations::linear::graphql_request(token, query, Some(variables))
        .await
        .map_err(|e| LinearError {
            kind: LinearErrorKind::Other(format!("Linear API error: {}", e)),
        })?;

    let issue_create = data
        .get("issueCreate")
        .ok_or_else(|| LinearError {
            kind: LinearErrorKind::Other(
                "Linear API response missing 'issueCreate' field".to_string(),
            ),
        })?;

    let success = issue_create
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !success {
        return Err(LinearError {
            kind: LinearErrorKind::Other("Linear API: issueCreate returned success=false".to_string()),
        });
    }

    let issue = issue_create.get("issue").ok_or_else(|| LinearError {
        kind: LinearErrorKind::Other(
            "Linear API: issueCreate succeeded but 'issue' field is missing".to_string(),
        ),
    })?;

    Ok(CreatedIssue {
        id: issue
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        identifier: issue
            .get("identifier")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        title: issue
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        url: issue
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

// ──────────────────────────────────────────────────────────────────────────────
// Output file helpers
// ──────────────────────────────────────────────────────────────────────────────

fn write_success_output(
    output_path: &Path,
    step_name: &str,
    description: Option<&str>,
    input_step: &str,
    integration_id: &str,
    issue: &CreatedIssue,
) -> Result<(), LinearError> {
    let now = Utc::now().to_rfc3339();

    let content = format!(
        r#"---
name: {}
description: "{}"
connector: linear
input: {}
status: done
created_at: {}
completed_at: {}
integration_id: {}
issues_created: 1
issue_id: {}
issue_identifier: {}
issue_url: {}
error: null
---

Created Linear issue {}: {}
URL: {}
"#,
        step_name,
        description.unwrap_or("Create Linear issue"),
        input_step,
        now,
        now,
        integration_id,
        issue.id,
        issue.identifier,
        issue.url,
        issue.identifier,
        issue.title,
        issue.url,
    );

    fs::write(output_path, content).map_err(|e| LinearError {
        kind: LinearErrorKind::Other(format!("Failed to write output file: {}", e)),
    })
}

fn write_failure_output(
    output_path: &Path,
    step_name: &str,
    description: Option<&str>,
    input_step: &str,
    integration_id: &str,
    error_message: &str,
    raw_llm_output: Option<&str>,
) -> Result<(), LinearError> {
    let now = Utc::now().to_rfc3339();
    let error_escaped = error_message.replace('"', "\\\"").replace('\n', " ");

    let frontmatter = format!(
        "---\nname: {}\ndescription: \"{}\"\nconnector: linear\ninput: {}\nstatus: failed\ncreated_at: {}\ncompleted_at: {}\nintegration_id: {}\nissues_created: 0\nerror: \"{}\"\n---\n",
        step_name,
        description.unwrap_or("Create Linear issue"),
        input_step,
        now,
        now,
        integration_id,
        error_escaped,
    );

    let body = match raw_llm_output {
        Some(raw) => format!(
            "\n## Error\n{}\n\n## Raw AI Output\n{}\n",
            error_message, raw
        ),
        None => format!("\n## Error\n{}\n", error_message),
    };

    let content = format!("{}{}", frontmatter, body);

    fs::write(output_path, content).map_err(|e| LinearError {
        kind: LinearErrorKind::Other(format!("Failed to write failure output file: {}", e)),
    })
}

// ──────────────────────────────────────────────────────────────────────────────
// Core execution logic
// ──────────────────────────────────────────────────────────────────────────────

async fn execute_inner(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    input_step: &str,
    description: Option<&str>,
) -> Result<(PathBuf, CreatedIssue, String), LinearError> {
    let connector_config = LinearConnectorConfig::from_value(config)?;

    let profile = load_linear_profile(&connector_config.integration_id)
        .map_err(|e| LinearError { kind: LinearErrorKind::Other(e) })?;

    let token = get_linear_token(&connector_config.integration_id)
        .map_err(|e| LinearError { kind: LinearErrorKind::Other(e) })?;

    let raw = fs::read_to_string(input_path).map_err(|e| LinearError {
        kind: LinearErrorKind::Other(format!(
            "Failed to read input file '{}': {}",
            input_path.display(),
            e
        )),
    })?;

    let obj = extract_json_object(&raw)?;

    validate_llm_output_for_linear(&obj, &profile, &raw)?;

    let issue_input = build_issue_input(&obj, &profile)?;

    let issue = create_issue(&token, issue_input).await?;

    fs::create_dir_all(output_dir).map_err(|e| LinearError {
        kind: LinearErrorKind::Other(format!("Failed to create output directory: {}", e)),
    })?;

    let output_path = output_dir.join(format!("{}.md", step_name));

    write_success_output(
        &output_path,
        step_name,
        description,
        input_step,
        &connector_config.integration_id,
        &issue,
    )?;

    Ok((output_path, issue, connector_config.integration_id))
}

// ──────────────────────────────────────────────────────────────────────────────
// Execute entry points
// ──────────────────────────────────────────────────────────────────────────────

pub async fn execute_structured(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    input_step: &str,
    description: Option<&str>,
) -> Result<PathBuf, LinearError> {
    execute_inner(input_path, config, output_dir, step_name, input_step, description)
        .await
        .map(|(path, _, _)| path)
}

pub async fn execute_with_raw_preservation(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    input_step: &str,
    description: Option<&str>,
    raw_llm_output: Option<&str>,
) -> Result<PathBuf, String> {
    let result = execute_inner(input_path, config, output_dir, step_name, input_step, description).await;

    match result {
        Ok((path, _, _)) => Ok(path),
        Err(e) => {
            let error_message = e.to_string();

            let _ = fs::create_dir_all(output_dir);

            let output_path = output_dir.join(format!("{}.md", step_name));

            let integration_id = config
                .get("integration_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let _ = write_failure_output(
                &output_path,
                step_name,
                description,
                input_step,
                integration_id,
                &error_message,
                raw_llm_output,
            );

            Err(error_message)
        }
    }
}
