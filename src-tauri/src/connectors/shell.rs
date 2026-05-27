// Shell processing connector — runs a configured local command, feeds the
// rendered template to its stdin, captures stdout as the step output.
//
// Connection.config shape (no secrets — Shell has no Keychain entry):
//   {
//     "command": "/usr/local/bin/jq",          // executable, absolute path encouraged
//     "args":    [".items[] | .name"],          // optional argv (after the command)
//     "cwd":     "~/work/notes",                // optional working dir (~ expanded)
//     "timeout_secs": 60,                       // optional, default 120
//     "env":     { "FOO": "bar" }               // optional env var overrides
//   }
//
// `stdin` is the engine's already-rendered template. `stdout` becomes the
// step's body — picked up by the runner for `{processing_result}` chaining.
// `stderr` is captured into the artifact's failure path only (not propagated
// as data), so a 0-exit script logging to stderr stays clean.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command as AsyncCommand;

const DEFAULT_TIMEOUT_SECS: u64 = 120;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShellConnectorConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

fn expand_tilde(path: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    if let Some(rest) = path.strip_prefix("~/") {
        PathBuf::from(home).join(rest)
    } else if path == "~" {
        PathBuf::from(home)
    } else {
        PathBuf::from(path)
    }
}

#[derive(Debug)]
struct ShellRunOutcome {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
}

async fn run_shell(cfg: &ShellConnectorConfig, stdin_payload: &str) -> Result<ShellRunOutcome, String> {
    if cfg.command.trim().is_empty() {
        return Err("Shell connector: 'command' is empty".to_string());
    }

    let mut cmd = AsyncCommand::new(&cfg.command);
    cmd.args(&cfg.args);
    if let Some(cwd) = cfg.cwd.as_deref().filter(|s| !s.trim().is_empty()) {
        cmd.current_dir(expand_tilde(cwd));
    }
    for (k, v) in &cfg.env {
        cmd.env(k, v);
    }
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn '{}': {} (is it on PATH or absolute?)", cfg.command, e))?;

    // Write stdin in a task so it doesn't deadlock with the child waiting on its
    // pipe — important for large rendered templates.
    if let Some(mut stdin) = child.stdin.take() {
        let payload = stdin_payload.as_bytes().to_vec();
        tokio::spawn(async move {
            // Closing the pipe signals EOF to the child; ignore write errors
            // (child may exit before consuming everything — that's a script bug,
            // not ours).
            let _ = stdin.write_all(&payload).await;
            let _ = stdin.shutdown().await;
        });
    }

    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();
    let timeout = Duration::from_secs(cfg.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));

    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(format!("Shell I/O error: {}", e)),
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(format!(
                "Shell command '{}' timed out after {}s",
                cfg.command,
                timeout.as_secs()
            ));
        }
    };

    let stdout = read_to_string(stdout_handle).await;
    let stderr = read_to_string(stderr_handle).await;

    Ok(ShellRunOutcome {
        stdout,
        stderr,
        exit_code: status.code(),
    })
}

async fn read_to_string<R>(handle: Option<R>) -> String
where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::AsyncReadExt;
    let mut buf = Vec::new();
    if let Some(mut h) = handle {
        let _ = h.read_to_end(&mut buf).await;
    }
    String::from_utf8_lossy(&buf).to_string()
}

/// Execute Shell connector for a pipeline step.
///
/// Returns Err on non-zero exit / timeout / spawn failure — engine treats that
/// as a Processing-step failure and halts downstream (per spec failure
/// semantics). Successful stdout is written into the step's `<step>.md`
/// artifact, which the engine reads back for `{processing_result}` chaining.
pub async fn execute(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    step_input: &str,
    step_description: Option<&str>,
) -> Result<PathBuf, String> {
    let cfg: ShellConnectorConfig = serde_json::from_value(config.clone())
        .map_err(|e| format!("Invalid Shell connector config: {}", e))?;

    let stdin_payload = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read Shell step input: {}", e))?;

    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let outcome = run_shell(&cfg, &stdin_payload).await;
    let completed_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;
    let output_path = output_dir.join(format!("{}.md", step_name));

    match outcome {
        Ok(run) if run.exit_code == Some(0) => {
            let frontmatter = format!(
                r#"---
name: {}
description: "{}"
connector: shell
input: {}
status: done
created_at: {}
completed_at: {}
command: {}
exit_code: 0
error: null
---

{}
"#,
                step_name,
                step_description.unwrap_or("Run shell").replace('"', "\\\""),
                step_input,
                created_at,
                completed_at,
                cfg.command,
                run.stdout.trim_end(),
            );
            fs::write(&output_path, frontmatter)
                .map_err(|e| format!("Failed to write Shell artifact: {}", e))?;
            Ok(output_path)
        }
        Ok(run) => {
            // Non-zero exit. Surface stderr as the primary diagnostic; fall back
            // to "exit code N" when stderr is empty.
            let detail = if !run.stderr.trim().is_empty() {
                run.stderr.trim().to_string()
            } else {
                format!("exit code: {:?}", run.exit_code)
            };
            let err_msg = format!("Shell '{}' failed: {}", cfg.command, detail);
            write_failure_artifact(
                &output_path,
                step_name,
                step_input,
                step_description,
                &cfg.command,
                &created_at,
                &completed_at,
                &err_msg,
                Some(&run.stderr),
                run.exit_code,
            );
            Err(err_msg)
        }
        Err(spawn_err) => {
            write_failure_artifact(
                &output_path,
                step_name,
                step_input,
                step_description,
                &cfg.command,
                &created_at,
                &completed_at,
                &spawn_err,
                None,
                None,
            );
            Err(spawn_err)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn write_failure_artifact(
    output_path: &Path,
    step_name: &str,
    step_input: &str,
    step_description: Option<&str>,
    command: &str,
    created_at: &str,
    completed_at: &str,
    err: &str,
    stderr: Option<&str>,
    exit_code: Option<i32>,
) {
    let err_escaped = err.replace('"', "\\\"").replace('\n', " ");
    let stderr_block = match stderr.filter(|s| !s.trim().is_empty()) {
        Some(s) => format!("\n\n## stderr\n{}\n", s.trim()),
        None => String::new(),
    };
    let frontmatter = format!(
        r#"---
name: {}
description: "{}"
connector: shell
input: {}
status: failed
created_at: {}
completed_at: {}
command: {}
exit_code: {}
error: "{}"
---

## Error
{}{}
"#,
        step_name,
        step_description.unwrap_or("Run shell").replace('"', "\\\""),
        step_input,
        created_at,
        completed_at,
        command,
        exit_code.map(|c| c.to_string()).unwrap_or_else(|| "null".to_string()),
        err_escaped,
        err,
        stderr_block,
    );
    let _ = fs::write(output_path, frontmatter);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn echo_stdin_roundtrips_through_cat() {
        let cfg = ShellConnectorConfig {
            command: "cat".to_string(),
            args: vec![],
            cwd: None,
            timeout_secs: Some(5),
            env: HashMap::new(),
        };
        let out = run_shell(&cfg, "hello pipeline\n").await.unwrap();
        assert_eq!(out.exit_code, Some(0));
        assert_eq!(out.stdout, "hello pipeline\n");
        assert!(out.stderr.is_empty());
    }

    #[tokio::test]
    async fn nonzero_exit_propagates_as_error_via_execute() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("in.txt");
        fs::write(&input, "ignored").unwrap();
        let cfg = serde_json::json!({
            "command": "sh",
            "args": ["-c", "echo failing >&2; exit 7"],
            "timeout_secs": 5
        });
        let res = execute(&input, &cfg, tmp.path(), "shell-step", "transcript", None).await;
        assert!(res.is_err(), "expected non-zero exit to be an error");
        let md = fs::read_to_string(tmp.path().join("shell-step.md")).unwrap();
        assert!(md.contains("status: failed"));
        assert!(md.contains("exit_code: 7"));
        assert!(md.contains("failing"));
    }

    #[tokio::test]
    async fn timeout_kills_long_running_command() {
        let cfg = ShellConnectorConfig {
            command: "sh".to_string(),
            args: vec!["-c".into(), "sleep 5".into()],
            cwd: None,
            timeout_secs: Some(1),
            env: HashMap::new(),
        };
        let res = run_shell(&cfg, "").await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("timed out"));
    }
}
