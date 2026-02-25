use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliAvailability {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub install_hint: String,
}

#[tauri::command]
pub async fn check_cli_availability() -> Result<Vec<CliAvailability>, String> {
    let clis = vec![
        ("claude", "Claude Code", "npm install -g @anthropic-ai/claude-code"),
        ("codex", "Codex CLI", "npm install -g @openai/codex"),
        ("opencode", "OpenCode", "npm install -g opencode-ai"),
    ];

    let mut results = Vec::new();
    for (id, name, hint) in clis {
        let installed = is_cli_installed(id).await;
        results.push(CliAvailability {
            id: id.to_string(),
            name: name.to_string(),
            installed,
            install_hint: hint.to_string(),
        });
    }

    Ok(results)
}

async fn is_cli_installed(cli: &str) -> bool {
    Command::new("which")
        .arg(cli)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Execute a CLI agent (Claude Code or Codex CLI) as a pipeline step.
///
/// Spawns the selected CLI binary as a subprocess, pipes the transcript + prompt
/// via stdin, captures stdout as the step output. The agent runs with full user
/// permissions in the configured working directory.
///
/// Config fields:
///   cli          - CLI binary name: "claude" or "codex" (required)
///   prompt       - Instruction prompt for the agent (required)
///   working_dir  - Working directory for the subprocess (optional, defaults to home)
///   timeout      - Max execution time in seconds (optional, default 300)
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

    let working_dir = config
        .get("working_dir")
        .and_then(|v| v.as_str())
        .unwrap_or("~");

    let timeout_secs = config
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(300);

    // Read input content (transcript or previous step output)
    let raw_content = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input file: {}", e))?;
    let content = super::strip_frontmatter(&raw_content);

    // Resolve working directory (expand ~)
    let cwd = expand_tilde(working_dir);
    if !cwd.exists() {
        return write_error(
            output_dir,
            step_name,
            step_input,
            step_description,
            &created_at,
            cli,
            &format!("Working directory does not exist: {}", cwd.display()),
        );
    }

    // Build CLI arguments
    let args = build_cli_args(cli, prompt);

    // Compose stdin: prompt context + transcript
    let stdin_content = format!(
        "{}\n\n---\n\nTranscript:\n\n{}",
        prompt, content
    );

    // Spawn subprocess
    let mut child = Command::new(cli)
        .args(&args)
        .current_dir(&cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| {
            format!(
                "Failed to spawn '{}': {} (is it installed and in PATH?)",
                cli, e
            )
        })?;

    // Write stdin
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        let _ = stdin.write_all(stdin_content.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }

    // Take stdout/stderr handles so we can explicitly kill/wait on timeout.
    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();

    let status = match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        child.wait(),
    )
    .await
    {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => {
            return write_error(
                output_dir,
                step_name,
                step_input,
                step_description,
                &created_at,
                cli,
                &format!("CLI agent process error: {}", e),
            );
        }
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return write_error(
                output_dir,
                step_name,
                step_input,
                step_description,
                &created_at,
                cli,
                &format!("CLI agent timed out after {} seconds", timeout_secs),
            );
        }
    };

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

    let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
    let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();

    if !status.success() {
        let exit_code = status.code().map_or("unknown".to_string(), |c| c.to_string());
        let error_detail = if stderr.is_empty() {
            format!("CLI agent exited with code {}", exit_code)
        } else {
            format!(
                "CLI agent exited with code {}: {}",
                exit_code,
                stderr.trim()
            )
        };
        return write_error(
            output_dir,
            step_name,
            step_input,
            step_description,
            &created_at,
            cli,
            &error_detail,
        );
    }

    // Write success output
    let completed_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    let output_path = output_dir.join(format!("{}.md", step_name));
    let desc_escaped = step_description.unwrap_or("").replace('"', "\\\"");
    let cli_escaped = cli.replace('"', "\\\"");

    let result_text = if stdout.trim().is_empty() {
        "(no output)".to_string()
    } else {
        stdout.trim().to_string()
    };

    let file_content = format!(
        "---\nname: {}\ndescription: \"{}\"\nconnector: cli_agent\ninput: {}\nstatus: done\ncreated_at: {}\ncompleted_at: {}\nerror: null\ncli: \"{}\"\n---\n\n{}\n",
        step_name,
        desc_escaped,
        step_input,
        created_at,
        completed_at,
        cli_escaped,
        result_text,
    );

    let temp_path = output_dir.join(format!(".{}.md.tmp", step_name));
    fs::write(&temp_path, &file_content)
        .map_err(|e| format!("Failed to write output: {}", e))?;
    fs::rename(&temp_path, &output_path)
        .map_err(|e| format!("Failed to finalize output: {}", e))?;

    Ok(output_path)
}

/// Build CLI-specific arguments for non-interactive pipe mode.
fn build_cli_args(cli: &str, _prompt: &str) -> Vec<String> {
    match cli {
        "claude" => vec![
            "--print".to_string(),
        ],
        "codex" => vec![
            "exec".to_string(),
        ],
        _ => vec![],
    }
}

/// Expand ~ prefix to home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(after_tilde) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(after_tilde)
    } else if path == "~" {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
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
    let cli_escaped = cli.replace('"', "\\\"");
    let file_content = format!(
        "---\nname: {}\ndescription: \"{}\"\nconnector: cli_agent\ninput: {}\nstatus: failed\ncreated_at: {}\ncompleted_at: {}\nerror: \"{}\"\ncli: \"{}\"\n---\n\n## Error\n{}\n",
        step_name,
        desc_escaped,
        step_input,
        created_at,
        completed_at,
        err_escaped,
        cli_escaped,
        error,
    );
    let temp_path = output_dir.join(format!(".{}.md.tmp", step_name));
    if let Err(e) = fs::write(&temp_path, &file_content) {
        eprintln!(
            "[cli_agent] Failed to write error state for step '{}': {}",
            step_name, e
        );
    } else if let Err(e) = fs::rename(&temp_path, &output_path) {
        eprintln!(
            "[cli_agent] Failed to finalize error state for step '{}': {}",
            step_name, e
        );
    }
    Err(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde_with_path() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let result = expand_tilde("~/projects/my-app");
        assert_eq!(result, PathBuf::from(home).join("projects/my-app"));
    }

    #[test]
    fn test_expand_tilde_bare() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let result = expand_tilde("~");
        assert_eq!(result, PathBuf::from(home));
    }

    #[test]
    fn test_expand_tilde_absolute() {
        let result = expand_tilde("/usr/local/bin");
        assert_eq!(result, PathBuf::from("/usr/local/bin"));
    }

    #[test]
    fn test_build_cli_args_claude() {
        let args = build_cli_args("claude", "test prompt");
        assert_eq!(args, vec!["--print"]);
    }

    #[test]
    fn test_build_cli_args_codex() {
        let args = build_cli_args("codex", "test prompt");
        assert_eq!(args, vec!["exec"]);
    }
}
