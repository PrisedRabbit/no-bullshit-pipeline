// Shell processing connector — runs a user-authored shell script.
//
// **Model (script-mode).** The Connection is just an *environment*:
//   { "cwd": "/abs/path",                 // REQUIRED — explicit, no default
//     "shell": "/bin/bash",               // optional, default /bin/bash
//     "env":  { "FOO": "bar" },           // optional, merged on top of NBP_* vars
//     "timeout_secs": 120 }               // optional, default 120
//
// The pipeline STEP carries the actual shell script (multi-line ok). The
// engine hands the unrendered script + the raw `{transcript}` / `{app}` /
// `{processing_result}` values to this connector. We invoke
// `<shell> -c <script>` in `cwd` with the raw values exported as env vars:
//
//   NBP_TRANSCRIPT         — full recording transcript
//   NBP_APP                — friendly app name (Zoom / FaceTime / NBP / …)
//   NBP_PROCESSING_RESULT  — previous Processing step's output (empty on step 1)
//   NBP_CALENDAR_TITLE     — matched calendar event title (empty if unmatched)
//   NBP_CALENDAR_ATTENDEES — matched event attendees, comma-separated
//   NBP_DATE               — recording date, local YYYY-MM-DD
//
// Env-var passing — not `{transcript}` placeholder substitution into the
// script string — keeps the user safe from shell injection (transcript text
// containing quotes / `$()` / backticks would otherwise execute).
//
// `stdout` becomes the step's body (chains into next step's
// `{processing_result}`). `stderr` is captured into the failure artifact
// only — a 0-exit script logging to stderr stays clean. Non-zero exit /
// timeout / spawn failure = step failure, halts downstream per the engine's
// Processing failure semantics.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command as AsyncCommand;

const DEFAULT_TIMEOUT_SECS: u64 = 120;
const DEFAULT_SHELL: &str = "/bin/bash";

/// Names of env vars the engine sets on every Shell step. Kept here so the
/// JS layer's hint text + this connector's runtime stay in sync via a
/// single source.
pub const ENV_VAR_TRANSCRIPT: &str = "NBP_TRANSCRIPT";
pub const ENV_VAR_APP: &str = "NBP_APP";
pub const ENV_VAR_PROCESSING_RESULT: &str = "NBP_PROCESSING_RESULT";
pub const ENV_VAR_CALENDAR_TITLE: &str = "NBP_CALENDAR_TITLE";
pub const ENV_VAR_CALENDAR_ATTENDEES: &str = "NBP_CALENDAR_ATTENDEES";
pub const ENV_VAR_DATE: &str = "NBP_DATE";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShellConnectorConfig {
    /// Required. Working directory for the script. Expanded for `~`.
    /// We refuse to default it — silent `cd $HOME` was surprising the user.
    pub cwd: String,
    /// Optional shell binary, default `/bin/bash`. Must accept `-c <script>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    /// Optional timeout. Kills the subprocess if exceeded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    /// Optional extra env vars. Merged on top of the NBP_* set so the user
    /// CAN override them (rarely needed; mostly for adding their own keys).
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

async fn run_shell(
    cfg: &ShellConnectorConfig,
    script: &str,
    env_extras: &HashMap<String, String>,
) -> Result<ShellRunOutcome, String> {
    if script.trim().is_empty() {
        return Err("Shell step has empty script".to_string());
    }
    if cfg.cwd.trim().is_empty() {
        return Err(
            "Shell step has no 'cwd' set — Working dir is required (no implicit default)."
                .to_string(),
        );
    }
    let cwd = expand_tilde(&cfg.cwd);
    if !cwd.is_dir() {
        return Err(format!(
            "Shell step cwd does not exist or is not a directory: {}",
            cwd.display()
        ));
    }

    let shell = cfg
        .shell
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(DEFAULT_SHELL);

    let mut cmd = AsyncCommand::new(shell);
    cmd.arg("-c").arg(script);
    cmd.current_dir(&cwd);
    // Order matters: engine-provided NBP_* first, user overrides last — same
    // shape as cargo's env-precedence model. Lets the user shadow if they
    // really want to (e.g. inject a different transcript for testing).
    for (k, v) in env_extras {
        cmd.env(k, v);
    }
    for (k, v) in &cfg.env {
        cmd.env(k, v);
    }
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn '{}': {}", shell, e))?;

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
                "Shell script timed out after {}s",
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
/// Diverges from the generic connector signature: takes the RAW script +
/// the RAW per-run values for the engine's `{transcript}` / `{app}` /
/// `{processing_result}` placeholders. The engine special-cases this in
/// `pipeline_engine::dispatch_step` so other connectors keep their
/// rendered-template contract.
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    script: &str,
    transcript: &str,
    app: &str,
    processing_result: &str,
    calendar_title: &str,
    calendar_attendees: &str,
    date: &str,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    step_input: &str,
    step_description: Option<&str>,
) -> Result<PathBuf, String> {
    let cfg: ShellConnectorConfig = serde_json::from_value(config.clone())
        .map_err(|e| format!("Invalid Shell connector config: {}", e))?;

    let mut env_extras: HashMap<String, String> = HashMap::new();
    env_extras.insert(ENV_VAR_TRANSCRIPT.to_string(), transcript.to_string());
    env_extras.insert(ENV_VAR_APP.to_string(), app.to_string());
    env_extras.insert(
        ENV_VAR_PROCESSING_RESULT.to_string(),
        processing_result.to_string(),
    );
    env_extras.insert(
        ENV_VAR_CALENDAR_TITLE.to_string(),
        calendar_title.to_string(),
    );
    env_extras.insert(
        ENV_VAR_CALENDAR_ATTENDEES.to_string(),
        calendar_attendees.to_string(),
    );
    env_extras.insert(ENV_VAR_DATE.to_string(), date.to_string());

    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let outcome = run_shell(&cfg, script, &env_extras).await;
    let completed_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    fs::create_dir_all(output_dir).map_err(|e| format!("Failed to create output dir: {}", e))?;
    let output_path = output_dir.join(format!("{}.md", step_name));

    let shell_label = cfg
        .shell
        .clone()
        .unwrap_or_else(|| DEFAULT_SHELL.to_string());

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
shell: {}
cwd: {}
exit_code: 0
error: null
---

{}
"#,
                step_name,
                step_description
                    .unwrap_or("Run shell script")
                    .replace('"', "\\\""),
                step_input,
                created_at,
                completed_at,
                shell_label,
                cfg.cwd,
                run.stdout.trim_end(),
            );
            fs::write(&output_path, frontmatter)
                .map_err(|e| format!("Failed to write Shell artifact: {}", e))?;
            Ok(output_path)
        }
        Ok(run) => {
            let detail = if !run.stderr.trim().is_empty() {
                run.stderr.trim().to_string()
            } else {
                format!("exit code: {:?}", run.exit_code)
            };
            let err_msg = format!("Shell script failed: {}", detail);
            write_failure_artifact(
                &output_path,
                step_name,
                step_input,
                step_description,
                &shell_label,
                &cfg.cwd,
                &created_at,
                &completed_at,
                &err_msg,
                Some(&run.stderr),
                run.exit_code,
            );
            Err(err_msg)
        }
        Err(err) => {
            write_failure_artifact(
                &output_path,
                step_name,
                step_input,
                step_description,
                &shell_label,
                &cfg.cwd,
                &created_at,
                &completed_at,
                &err,
                None,
                None,
            );
            Err(err)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn write_failure_artifact(
    output_path: &Path,
    step_name: &str,
    step_input: &str,
    step_description: Option<&str>,
    shell: &str,
    cwd: &str,
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
shell: {}
cwd: {}
exit_code: {}
error: "{}"
---

## Error
{}{}
"#,
        step_name,
        step_description
            .unwrap_or("Run shell script")
            .replace('"', "\\\""),
        step_input,
        created_at,
        completed_at,
        shell,
        cwd,
        exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "null".to_string()),
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

    fn env_with_defaults() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert(ENV_VAR_TRANSCRIPT.to_string(), "hello world".to_string());
        m.insert(ENV_VAR_APP.to_string(), "TestApp".to_string());
        m.insert(ENV_VAR_PROCESSING_RESULT.to_string(), "".to_string());
        m
    }

    #[tokio::test]
    async fn env_vars_visible_to_script() {
        let cfg = ShellConnectorConfig {
            cwd: std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()),
            shell: None,
            timeout_secs: Some(5),
            env: HashMap::new(),
        };
        let env = env_with_defaults();
        let out = run_shell(&cfg, r#"echo "got: $NBP_TRANSCRIPT from $NBP_APP""#, &env)
            .await
            .unwrap();
        assert_eq!(out.exit_code, Some(0));
        assert_eq!(out.stdout.trim(), "got: hello world from TestApp");
    }

    #[tokio::test]
    async fn empty_cwd_fails_loudly() {
        let cfg = ShellConnectorConfig {
            cwd: String::new(),
            shell: None,
            timeout_secs: Some(5),
            env: HashMap::new(),
        };
        let res = run_shell(&cfg, "echo ok", &env_with_defaults()).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("cwd"));
    }

    #[tokio::test]
    async fn nonexistent_cwd_fails_loudly() {
        let cfg = ShellConnectorConfig {
            cwd: "/no/such/dir/at/all".to_string(),
            shell: None,
            timeout_secs: Some(5),
            env: HashMap::new(),
        };
        let res = run_shell(&cfg, "echo ok", &env_with_defaults()).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn nonzero_exit_propagates_with_stderr() {
        let tmp = TempDir::new().unwrap();
        let cfg = serde_json::json!({
            "cwd": tmp.path(),
            "timeout_secs": 5
        });
        let res = execute(
            r#"echo "boom" >&2; exit 7"#,
            "transcript",
            "App",
            "prev",
            "",
            "",
            "",
            &cfg,
            tmp.path(),
            "shell-step",
            "rendered-template",
            None,
        )
        .await;
        assert!(res.is_err());
        let md = fs::read_to_string(tmp.path().join("shell-step.md")).unwrap();
        assert!(md.contains("status: failed"));
        assert!(md.contains("exit_code: 7"));
        assert!(md.contains("boom"));
    }

    #[tokio::test]
    async fn timeout_kills_long_running_script() {
        let cfg = ShellConnectorConfig {
            cwd: std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()),
            shell: None,
            timeout_secs: Some(1),
            env: HashMap::new(),
        };
        let res = run_shell(&cfg, "sleep 5", &env_with_defaults()).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("timed out"));
    }

    #[tokio::test]
    async fn user_env_overrides_nbp_default() {
        let mut user_env = HashMap::new();
        user_env.insert(ENV_VAR_APP.to_string(), "Overridden".to_string());
        let cfg = ShellConnectorConfig {
            cwd: std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()),
            shell: None,
            timeout_secs: Some(5),
            env: user_env,
        };
        let out = run_shell(&cfg, "echo $NBP_APP", &env_with_defaults())
            .await
            .unwrap();
        assert_eq!(out.stdout.trim(), "Overridden");
    }
}
