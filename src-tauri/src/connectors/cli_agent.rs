use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

/// Execute a CLI agent (Claude Code or Codex) as a pipeline step.
///
/// Config fields:
///   cli             - "claude" or "codex" (required)
///   prompt          - Prompt text sent along with the input (required)
///   working_directory - Working directory for the subprocess (optional, default: home dir)
///   timeout_secs    - Timeout in seconds (optional, default: 300)
pub async fn execute(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    step_input: &str,
    step_description: Option<&str>,
) -> Result<PathBuf, String> {
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let cli = config
        .get("cli")
        .and_then(|v| v.as_str())
        .ok_or("CLI agent config missing 'cli' (must be 'claude' or 'codex')")?;

    if cli != "claude" && cli != "codex" {
        return Err(format!(
            "CLI agent 'cli' must be 'claude' or 'codex', got '{}'",
            cli
        ));
    }

    let prompt = config
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or("CLI agent config missing 'prompt'")?;

    let timeout_secs = config
        .get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(300);

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let working_directory = config
        .get("working_directory")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|p| expand_tilde(p, &home))
        .unwrap_or_else(|| PathBuf::from(&home));

    // Read input content
    let raw_content = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input file: {}", e))?;
    let content = super::strip_frontmatter(&raw_content);

    // Build the full prompt: user prompt + input content
    let full_prompt = format!("{}\n\n{}", prompt, content);

    // Build command based on CLI type
    let mut cmd = match cli {
        "claude" => {
            let mut c = Command::new("claude");
            c.arg("-p").arg(&full_prompt);
            c
        }
        "codex" => {
            let mut c = Command::new("codex");
            c.arg("exec").arg(&full_prompt);
            c
        }
        _ => unreachable!(),
    };

    cmd.current_dir(&working_directory);
    // Prevent interactive prompts
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        format!(
            "Failed to spawn '{}': {} (is it installed and in PATH?)",
            cli, e
        )
    })?;

    let pid = child.id();

    // Take stdout/stderr handles before waiting (wait_with_output takes ownership)
    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();

    // Enforce timeout: wait with timeout, then kill if exceeded
    let wait_result = tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        child.wait(),
    )
    .await;

    let status = match wait_result {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => {
            return write_error(
                output_dir, step_name, step_input, step_description,
                &created_at, cli, &format!("Process I/O error: {}", e),
            );
        }
        Err(_timeout) => {
            // Timeout expired — kill the subprocess and reap it
            let _ = child.kill().await;
            let _ = child.wait().await;
            return write_error(
                output_dir, step_name, step_input, step_description,
                &created_at, cli,
                &format!(
                    "CLI agent timed out after {}s (pid: {:?})",
                    timeout_secs, pid
                ),
            );
        }
    };

    // Read captured output
    let stdout_bytes = if let Some(mut h) = stdout_handle {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        let _ = h.read_to_end(&mut buf).await;
        buf
    } else {
        Vec::new()
    };
    let stderr_bytes = if let Some(mut h) = stderr_handle {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        let _ = h.read_to_end(&mut buf).await;
        buf
    } else {
        Vec::new()
    };

    let completed_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr_bytes);
        let stdout = String::from_utf8_lossy(&stdout_bytes);
        let details = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            format!("exit code: {:?}", status.code())
        };
        return write_error(
            output_dir, step_name, step_input, step_description,
            &created_at, cli,
            &format!("CLI agent '{}' failed: {}", cli, details.trim()),
        );
    }

    let agent_output = String::from_utf8_lossy(&stdout_bytes).to_string();

    // Write success output
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    let output_path = output_dir.join(format!("{}.md", step_name));
    let desc_escaped = step_description.unwrap_or("").replace('"', "\\\"");
    let file_content = format!(
        "---\nname: {}\ndescription: \"{}\"\nconnector: cli_agent\ninput: {}\nstatus: done\ncreated_at: {}\ncompleted_at: {}\nerror: null\ncli: {}\n---\n\n{}\n",
        step_name,
        desc_escaped,
        step_input,
        created_at,
        completed_at,
        cli,
        agent_output.trim(),
    );

    let temp_path = output_dir.join(format!(".{}.md.tmp", step_name));
    fs::write(&temp_path, &file_content)
        .map_err(|e| format!("Failed to write output: {}", e))?;
    fs::rename(&temp_path, &output_path)
        .map_err(|e| format!("Failed to finalize output: {}", e))?;

    Ok(output_path)
}

/// Expand `~` to the user's home directory.
fn expand_tilde(path: &str, home: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        PathBuf::from(home).join(rest)
    } else if path == "~" {
        PathBuf::from(home)
    } else {
        PathBuf::from(path)
    }
}

/// Write a failure output .md file and return Err.
fn write_error(
    output_dir: &Path,
    step_name: &str,
    step_input: &str,
    step_description: Option<&str>,
    created_at: &str,
    cli: &str,
    error: &str,
) -> Result<PathBuf, String> {
    let completed_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let _ = fs::create_dir_all(output_dir);
    let output_path = output_dir.join(format!("{}.md", step_name));
    let desc_escaped = step_description.unwrap_or("").replace('"', "\\\"");
    let err_escaped = error.replace('"', "\\\"").replace('\n', " ");
    let file_content = format!(
        "---\nname: {}\ndescription: \"{}\"\nconnector: cli_agent\ninput: {}\nstatus: failed\ncreated_at: {}\ncompleted_at: {}\nerror: \"{}\"\ncli: {}\n---\n\n## Error\n{}\n",
        step_name,
        desc_escaped,
        step_input,
        created_at,
        completed_at,
        err_escaped,
        cli,
        error,
    );
    let temp_path = output_dir.join(format!(".{}.md.tmp", step_name));
    if let Err(e) = fs::write(&temp_path, &file_content) {
        eprintln!("[cli_agent] Failed to write error state for step '{}': {}", step_name, e);
    } else if let Err(e) = fs::rename(&temp_path, &output_path) {
        eprintln!("[cli_agent] Failed to finalize error state for step '{}': {}", step_name, e);
    }
    Err(error.to_string())
}
