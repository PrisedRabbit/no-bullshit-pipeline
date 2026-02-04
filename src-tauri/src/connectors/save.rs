use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

/// Save connector config parsed from step config JSON
#[derive(Debug)]
struct SaveConfig {
    path: String,
}

impl SaveConfig {
    fn from_value(config: &serde_json::Value) -> Result<Self, String> {
        let path = config
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Save connector config missing 'path'")?
            .to_string();

        Ok(SaveConfig { path })
    }
}

/// Context for path variable substitution
pub struct SubstitutionContext {
    pub pipeline_name: String,
    pub recording_id: String,
    pub step_name: String,
}

/// Resolve path variables and expand ~ to home directory
pub fn resolve_path(raw_path: &str, ctx: &SubstitutionContext) -> PathBuf {
    let now = Utc::now();

    let resolved = raw_path
        .replace("{date}", &now.format("%Y-%m-%d").to_string())
        .replace("{time}", &now.format("%H-%M-%S").to_string())
        .replace("{pipeline-name}", &ctx.pipeline_name)
        .replace("{recording-id}", &ctx.recording_id)
        .replace("{step-name}", &ctx.step_name);

    // Expand ~ to home directory
    if let Some(after_tilde) = resolved.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(after_tilde)
    } else {
        PathBuf::from(resolved)
    }
}

/// Validate that a resolved target path is safe to write to.
/// Rejects paths that attempt directory traversal or target sensitive locations.
fn validate_target_path(path: &Path) -> Result<(), String> {
    let path_str = path.to_string_lossy();

    // Reject paths containing .. (directory traversal)
    for component in path.components() {
        if let std::path::Component::ParentDir = component {
            return Err(format!(
                "Target path contains directory traversal (..): {}",
                path_str
            ));
        }
    }

    // Reject paths targeting system directories
    let forbidden_prefixes = ["/System", "/Library", "/usr", "/bin", "/sbin", "/etc", "/var"];
    for prefix in &forbidden_prefixes {
        if path_str.starts_with(prefix) {
            return Err(format!(
                "Target path targets a protected system directory: {}",
                path_str
            ));
        }
    }

    Ok(())
}

/// Execute the Save connector for a pipeline step
///
/// - `input_path`: path to input file (previous step output)
/// - `config`: connector-specific config from pipeline definition
/// - `output_dir`: directory for step metadata output
/// - `step_name`: name of the current step
/// - `step_input`: input reference name (for frontmatter)
/// - `step_description`: optional description
/// - `pipeline_name`: pipeline name for variable substitution
/// - `recording_id`: recording UUID for variable substitution
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    step_input: &str,
    step_description: Option<&str>,
    pipeline_name: &str,
    recording_id: &str,
) -> Result<PathBuf, String> {
    let save_config = SaveConfig::from_value(config)?;
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    // Read input content
    let content = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input file: {}", e))?;

    // Resolve target path
    let ctx = SubstitutionContext {
        pipeline_name: pipeline_name.to_string(),
        recording_id: recording_id.to_string(),
        step_name: step_name.to_string(),
    };
    let target_path = resolve_path(&save_config.path, &ctx);

    // Validate target path is safe
    validate_target_path(&target_path)?;

    // Check if file exists (for overwrite warning)
    let overwrite = target_path.exists();

    // Create target directory if needed
    if let Some(parent) = target_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory '{}': {}", parent.display(), e))?;
    }

    // Atomic write to target with unique temp name
    let temp_target = target_path.with_file_name(format!(
        ".{}.tmp",
        target_path.file_name().unwrap_or_default().to_string_lossy()
    ));
    fs::write(&temp_target, &content)
        .map_err(|e| format!("Failed to write to '{}': {}", temp_target.display(), e))?;
    fs::rename(&temp_target, &target_path)
        .map_err(|e| format!("Failed to finalize '{}': {}", target_path.display(), e))?;

    // Write step metadata
    let completed_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    let metadata_path = output_dir.join(format!("{}.md", step_name));

    let overwrite_str = if overwrite { "true" } else { "false" };
    let warning = if overwrite {
        "\nWARNING: Existing file was overwritten."
    } else {
        ""
    };

    let metadata_content = format!(
        "---\nname: {}\ndescription: \"{}\"\nconnector: save\ninput: {}\nstatus: done\ncreated_at: {}\ncompleted_at: {}\nerror: null\ntarget_path: {}\noverwrite: {}\n---\n\nFile saved to: {}{}\n",
        step_name,
        step_description.unwrap_or("").replace('"', "\\\""),
        step_input,
        created_at,
        completed_at,
        target_path.display(),
        overwrite_str,
        target_path.display(),
        warning,
    );

    let temp_metadata = output_dir.join(format!(".{}.md.tmp", step_name));
    fs::write(&temp_metadata, &metadata_content)
        .map_err(|e| format!("Failed to write metadata: {}", e))?;
    fs::rename(&temp_metadata, &metadata_path)
        .map_err(|e| format!("Failed to finalize metadata: {}", e))?;

    Ok(metadata_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> SubstitutionContext {
        SubstitutionContext {
            pipeline_name: "meeting-notes".to_string(),
            recording_id: "abc-123-def".to_string(),
            step_name: "save-step".to_string(),
        }
    }

    #[test]
    fn test_resolve_path_date() {
        let ctx = make_ctx();
        let result = resolve_path("~/docs/{date}-notes.md", &ctx);
        let date_str = Utc::now().format("%Y-%m-%d").to_string();
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        assert_eq!(
            result,
            PathBuf::from(home).join(format!("docs/{}-notes.md", date_str))
        );
    }

    #[test]
    fn test_resolve_path_pipeline_name() {
        let ctx = make_ctx();
        let result = resolve_path("/tmp/{pipeline-name}/{recording-id}.md", &ctx);
        assert_eq!(
            result,
            PathBuf::from("/tmp/meeting-notes/abc-123-def.md")
        );
    }

    #[test]
    fn test_resolve_path_step_name() {
        let ctx = make_ctx();
        let result = resolve_path("/tmp/{step-name}.md", &ctx);
        assert_eq!(result, PathBuf::from("/tmp/save-step.md"));
    }

    #[test]
    fn test_resolve_path_tilde_expansion() {
        let ctx = make_ctx();
        let result = resolve_path("~/Documents/test.md", &ctx);
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        assert_eq!(result, PathBuf::from(home).join("Documents/test.md"));
    }

    #[test]
    fn test_resolve_path_absolute() {
        let ctx = make_ctx();
        let result = resolve_path("/absolute/path/file.md", &ctx);
        assert_eq!(result, PathBuf::from("/absolute/path/file.md"));
    }

    #[test]
    fn test_resolve_path_time() {
        let ctx = make_ctx();
        let result = resolve_path("/tmp/{time}.md", &ctx);
        // Just verify it doesn't contain the literal {time}
        assert!(!result.to_string_lossy().contains("{time}"));
    }

    #[test]
    fn test_save_config_from_value() {
        let config = serde_json::json!({
            "path": "~/Documents/{date}-{pipeline-name}.md"
        });
        let save_config = SaveConfig::from_value(&config).unwrap();
        assert_eq!(
            save_config.path,
            "~/Documents/{date}-{pipeline-name}.md"
        );
    }

    #[test]
    fn test_save_config_missing_path() {
        let config = serde_json::json!({});
        let result = SaveConfig::from_value(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("path"));
    }

    #[test]
    fn test_validate_target_path_normal() {
        let path = PathBuf::from("/Users/test/Documents/output.md");
        assert!(validate_target_path(&path).is_ok());
    }

    #[test]
    fn test_validate_target_path_traversal_rejected() {
        let path = PathBuf::from("/Users/test/../../../etc/passwd");
        assert!(validate_target_path(&path).is_err());
    }

    #[test]
    fn test_validate_target_path_system_dir_rejected() {
        let path = PathBuf::from("/System/Library/test.md");
        assert!(validate_target_path(&path).is_err());
    }

    #[test]
    fn test_validate_target_path_home_dir_ok() {
        let path = PathBuf::from("/Users/test/Documents/notes.md");
        assert!(validate_target_path(&path).is_ok());
    }
}
