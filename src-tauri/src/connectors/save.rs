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

/// Make a filesystem-safe filename fragment while staying readable — keeps
/// spaces (fine on macOS) and just neutralises the path-illegal characters.
fn sanitize_name_part(value: &str) -> String {
    let mapped: String = value
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' => '-',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let trimmed = mapped.trim();
    if trimmed.is_empty() { "Recording".to_string() } else { trimmed.to_string() }
}

/// Substitute date/time placeholders in the folder path so a user can route a
/// save into dated subfolders in any order/format they like — e.g.
/// `~/output/{YYYY}/{MM}-{DD}` or `~/notes/{DD}-{MM}-{YY}/{HH}`. The instant is
/// the recording's local start time (same source as the filename), so a save
/// lands in the folder for when the meeting happened. Note `{mm}` (lowercase) is
/// minutes — `{MM}` is month. Unparseable timestamps fall back to now, matching
/// `build_stem`. The rendered values are all zero-padded numbers (no path
/// separators), so this can only add the folder levels the user explicitly typed.
fn render_folder_placeholders(folder: &str, started_at: &str) -> String {
    let when = chrono::DateTime::parse_from_rfc3339(started_at)
        .map(|dt| dt.with_timezone(&chrono::Local))
        .unwrap_or_else(|_| chrono::Local::now());
    folder
        .replace("{YYYY}", &when.format("%Y").to_string())
        .replace("{YY}", &when.format("%y").to_string())
        .replace("{MM}", &when.format("%m").to_string())
        .replace("{DD}", &when.format("%d").to_string())
        .replace("{date}", &when.format("%Y-%m-%d").to_string())
        .replace("{HH}", &when.format("%H").to_string())
        .replace("{mm}", &when.format("%M").to_string())
        .replace("{SS}", &when.format("%S").to_string())
        .replace("{time}", &when.format("%H-%M-%S").to_string())
}

/// Build the human-readable file stem: `<app> <date> <start-time>`, e.g.
/// `Zoom 2026-06-01 14-30`. `started_at` is the recording's RFC3339 creation
/// time; it's shown in local wall-clock so it matches when the meeting
/// actually happened. Falls back to "now" if the timestamp can't be parsed.
fn build_stem(app: &str, started_at: &str) -> String {
    let when = chrono::DateTime::parse_from_rfc3339(started_at)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H-%M").to_string())
        .unwrap_or_else(|_| chrono::Local::now().format("%Y-%m-%d %H-%M").to_string());
    format!("{} {}", sanitize_name_part(app), when)
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

/// Write `content` to `<folder>/<app> <date> <start-time>.md`, atomically, and
/// return the path. No recording context needed — usable from both the
/// pipeline engine and the dictation paste flow. `app` + `started_at` name the
/// file after the recording/session it belongs to.
pub fn save_to_folder(
    content: &str,
    config: &serde_json::Value,
    app: &str,
    started_at: &str,
) -> Result<PathBuf, String> {
    let folder_path = config
        .get("folder_path")
        .or_else(|| config.get("folder"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("Save step needs a folder — set 'Folder' in the step editor.")?;

    let folder_path = render_folder_placeholders(folder_path, started_at);
    let stem = build_stem(app, started_at);
    let target = uniquify(expand_tilde(&folder_path).join(format!("{}.md", stem)));

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
    Ok(target)
}

/// Save `content` to the configured folder and write the `<step>.md` artifact
/// (body = "Saved to <path>") the pipeline engine reads back.
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    content: &str,
    config: &serde_json::Value,
    app: &str,
    started_at: &str,
    output_dir: &Path,
    step_name: &str,
    step_input: &str,
    description: Option<&str>,
) -> Result<PathBuf, String> {
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let target = save_to_folder(content, config, app, started_at)?;

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
    fn test_sanitize_name_part() {
        // Spaces are kept (readable); only path-illegal chars are neutralised.
        assert_eq!(sanitize_name_part("Microsoft Teams"), "Microsoft Teams");
        assert_eq!(sanitize_name_part("a/b:c"), "a-b-c");
        assert_eq!(sanitize_name_part("   "), "Recording");
    }

    #[test]
    fn test_build_stem() {
        let stem = build_stem("Zoom", "2026-06-01T14:30:00Z");
        // App name leads; a date-like "YYYY-MM-DD HH-MM" follows. Exact time is
        // local-tz-dependent, so assert structure not the literal value.
        assert!(stem.starts_with("Zoom "), "got: {stem}");
        assert!(stem.contains("2026-0"), "got: {stem}");
        // Unparseable timestamp falls back to now (still app-prefixed).
        assert!(build_stem("NBP", "not-a-date").starts_with("NBP "));
    }

    #[test]
    fn test_render_folder_placeholders() {
        // Compute expectations via the same local-tz conversion so the test is
        // timezone-independent (the rendered day depends on the runner's tz).
        let at = "2026-06-07T14:30:00Z";
        let when = chrono::DateTime::parse_from_rfc3339(at)
            .unwrap()
            .with_timezone(&chrono::Local);
        let (y, yy, m, d, hh, mi, ss) = (
            when.format("%Y").to_string(),
            when.format("%y").to_string(),
            when.format("%m").to_string(),
            when.format("%d").to_string(),
            when.format("%H").to_string(),
            when.format("%M").to_string(),
            when.format("%S").to_string(),
        );

        // Components substitute independently, so any order/format works.
        assert_eq!(
            render_folder_placeholders("~/out/{YYYY}/{MM}-{DD}", at),
            format!("~/out/{y}/{m}-{d}")
        );
        assert_eq!(
            render_folder_placeholders("~/out/{DD}-{MM}-{YY}", at),
            format!("~/out/{d}-{m}-{yy}")
        );
        assert_eq!(
            render_folder_placeholders("~/out/{date}/nbp", at),
            format!("~/out/{y}-{m}-{d}/nbp")
        );
        // Time components: {mm} is minutes, distinct from {MM} (month).
        assert_eq!(
            render_folder_placeholders("~/out/{HH}-{mm}-{SS}", at),
            format!("~/out/{hh}-{mi}-{ss}")
        );
        assert_eq!(
            render_folder_placeholders("~/out/{MM}/{mm}", at),
            format!("~/out/{m}/{mi}")
        );
        assert_eq!(
            render_folder_placeholders("~/out/{date}/{time}", at),
            format!("~/out/{y}-{m}-{d}/{hh}-{mi}-{ss}")
        );
        // No placeholders → unchanged.
        assert_eq!(render_folder_placeholders("~/Documents/Meetings", at), "~/Documents/Meetings");
        // Unparseable timestamp falls back to today (4-digit year).
        assert_eq!(render_folder_placeholders("{YYYY}", "nope").len(), 4);
    }

    #[test]
    fn test_expand_tilde() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_tilde("~/x"), PathBuf::from(&home).join("x"));
        assert_eq!(expand_tilde("/abs/x"), PathBuf::from("/abs/x"));
    }
}
