// Save-to-folder connector. The one built-in destination: write the step's
// content (processing_result or transcript — the engine renders step.template
// to decide which) to a file in a user-chosen local folder.
//
// Step config: { "folder_path": "~/Documents/Meetings" }  (WHERE — required)
// The WHAT is encoded in step.template (`{processing_result}` | `{transcript}`)
// and arrives here already rendered as `content`.

use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

/// Expand a leading `~` / `~/` to $HOME.
fn expand_tilde(p: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    if p == "~" {
        return PathBuf::from(home);
    }
    if let Some(rest) = p.strip_prefix("~/") {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(p)
}

/// Make a filesystem-safe filename fragment.
fn sanitize(value: &str) -> String {
    let mapped: String = value
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let trimmed = mapped.trim_matches('-');
    if trimmed.is_empty() { "pipeline".to_string() } else { trimmed.to_string() }
}

/// Append `-001`, `-002`, … until the path is free, so a second run on the
/// same day doesn't clobber the first.
fn uniquify(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file").to_string();
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("md").to_string();
    for i in 1..1000 {
        let candidate = parent.join(format!("{}-{:03}.{}", stem, i, ext));
        if !candidate.exists() {
            return candidate;
        }
    }
    path
}

/// Save `content` to `<folder>/<date>-<pipeline>.md`, atomically. Writes the
/// `<step>.md` artifact (body = "Saved to <path>") the engine reads back.
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    content: &str,
    config: &serde_json::Value,
    pipeline_name: &str,
    output_dir: &Path,
    step_name: &str,
    step_input: &str,
    description: Option<&str>,
) -> Result<PathBuf, String> {
    let folder_path = config
        .get("folder_path")
        .or_else(|| config.get("folder"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("Save step needs a folder — set 'Folder' in the step editor.")?;

    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let date = Utc::now().format("%Y-%m-%d").to_string();
    let stem = format!("{}-{}", date, sanitize(pipeline_name));
    let target = uniquify(expand_tilde(folder_path).join(format!("{}.md", stem)));

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory '{}': {}", parent.display(), e))?;
    }

    let temp = target.with_file_name(format!(
        ".{}.tmp",
        target.file_name().unwrap_or_default().to_string_lossy()
    ));
    fs::write(&temp, content)
        .map_err(|e| format!("Failed to write '{}': {}", temp.display(), e))?;
    fs::rename(&temp, &target)
        .map_err(|e| format!("Failed to finalize '{}': {}", target.display(), e))?;

    // Step artifact — same shape the other connectors emit so get_step_outputs
    // reports status; body becomes the chained {processing_result} + run toast.
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;
    let completed_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let artifact = output_dir.join(format!("{}.md", step_name));
    let md = format!(
        "---\nname: {}\ndescription: \"{}\"\nconnector: save_local\ninput: {}\nstatus: done\ncreated_at: {}\ncompleted_at: {}\nerror: null\ntarget_path: {}\n---\n\nSaved to {}\n",
        step_name,
        description.unwrap_or("").replace('"', "\\\""),
        step_input,
        created_at,
        completed_at,
        target.display(),
        target.display(),
    );
    let temp_md = output_dir.join(format!(".{}.md.tmp", step_name));
    fs::write(&temp_md, &md)
        .map_err(|e| format!("Failed to write metadata: {}", e))?;
    fs::rename(&temp_md, &artifact)
        .map_err(|e| format!("Failed to finalize metadata: {}", e))?;

    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize() {
        assert_eq!(sanitize("Meeting Notes!"), "Meeting-Notes");
        assert_eq!(sanitize("  "), "pipeline");
        assert_eq!(sanitize("a/b:c"), "a-b-c");
    }

    #[test]
    fn test_expand_tilde() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_tilde("~/x"), PathBuf::from(&home).join("x"));
        assert_eq!(expand_tilde("/abs/x"), PathBuf::from("/abs/x"));
    }
}
