use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Execute an MCP tool call via Streamable HTTP transport (2024-11-05 spec).
///
/// Protocol sequence:
///   1. POST /  with JSON-RPC `initialize` → server responds with capabilities
///   2. POST /  with JSON-RPC `notifications/initialized` (notification, no id)
///   3. POST /  with JSON-RPC `tools/call` → returns tool result
///
/// Config fields:
///   url   - MCP server base URL (required), e.g. "https://mcp.example.com"
///   tool  - Tool name to invoke (required), e.g. "send-message"
///   args  - Optional JSON object passed as tool arguments (default: {})
pub async fn execute(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    step_input: &str,
    step_description: Option<&str>,
) -> Result<PathBuf, String> {
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let url = config
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("MCP connector config missing 'url'")?
        .trim_end_matches('/')
        .to_string();

    let tool = config
        .get("tool")
        .and_then(|v| v.as_str())
        .ok_or("MCP connector config missing 'tool'")?
        .to_string();

    // Extra args to merge into the tool call arguments
    let extra_args = config
        .get("args")
        .cloned()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    // Read and strip frontmatter from input
    let raw_content = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input file: {}", e))?;
    let content = super::strip_frontmatter(&raw_content);

    // Build arguments: merge extra_args with the content field
    let mut args_map = match extra_args {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };
    args_map.insert("content".to_string(), serde_json::Value::String(content.to_string()));
    let arguments = serde_json::Value::Object(args_map);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Step 1: initialize
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "nbp",
                "version": "0.4.0"
            }
        }
    });

    let init_resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&init_req)
        .send()
        .await
        .map_err(|e| format!("MCP initialize request failed: {}", e))?;

    if !init_resp.status().is_success() {
        let status = init_resp.status();
        let body = init_resp.text().await.unwrap_or_default();
        return write_error(
            output_dir,
            step_name,
            step_input,
            step_description,
            &created_at,
            &url,
            &tool,
            &format!("MCP initialize failed: HTTP {} — {}", status, body.trim()),
        );
    }

    // Extract session ID from initialize response if provided
    let session_id = init_resp
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Step 2: notifications/initialized (notification — no id, no response expected)
    let notif = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });
    // Send notification; ignore errors (some servers may not accept it)
    {
        let mut req = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");
        if let Some(ref sid) = session_id {
            req = req.header("Mcp-Session-Id", sid);
        }
        let _ = req.json(&notif).send().await;
    }

    // Step 3: tools/call
    let call_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": tool,
            "arguments": arguments
        }
    });

    let mut call_builder = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream");
    if let Some(ref sid) = session_id {
        call_builder = call_builder.header("Mcp-Session-Id", sid);
    }
    let call_resp = call_builder
        .json(&call_req)
        .send()
        .await
        .map_err(|e| format!("MCP tools/call request failed: {}", e))?;

    let call_status = call_resp.status();
    let content_type = call_resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body_text = call_resp
        .text()
        .await
        .unwrap_or_else(|_| "<failed to read response>".to_string());

    let completed_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    if !call_status.is_success() {
        return write_error(
            output_dir,
            step_name,
            step_input,
            step_description,
            &created_at,
            &url,
            &tool,
            &format!("MCP tools/call failed: HTTP {} — {}", call_status, body_text.trim()),
        );
    }

    // Parse JSON-RPC response — handle both JSON and SSE (text/event-stream)
    let rpc: serde_json::Value = if content_type.contains("text/event-stream") {
        // Extract JSON-RPC message from SSE: lines starting with "data: "
        let json_line = body_text
            .lines()
            .find(|l| l.starts_with("data: "))
            .map(|l| &l["data: ".len()..])
            .unwrap_or("");
        if json_line.is_empty() {
            return write_error(
                output_dir,
                step_name,
                step_input,
                step_description,
                &created_at,
                &url,
                &tool,
                "MCP tools/call returned SSE stream with no data event",
            );
        }
        serde_json::from_str(json_line).map_err(|e| {
            format!("MCP tools/call SSE data is not valid JSON: {}", e)
        })?
    } else {
        serde_json::from_str(&body_text).map_err(|e| {
            format!("MCP tools/call response is not valid JSON: {}", e)
        })?
    };

    // Check for JSON-RPC error
    if let Some(err_obj) = rpc.get("error") {
        let msg = err_obj
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return write_error(
            output_dir,
            step_name,
            step_input,
            step_description,
            &created_at,
            &url,
            &tool,
            &format!("MCP error: {}", msg),
        );
    }

    // Extract result content
    let result_text = extract_result_text(&rpc);

    // Write success output
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    let output_path = output_dir.join(format!("{}.md", step_name));
    let desc_escaped = step_description.unwrap_or("").replace('"', "\\\"");
    let tool_escaped = tool.replace('"', "\\\"");
    let file_content = format!(
        "---\nname: {}\ndescription: \"{}\"\nconnector: mcp\ninput: {}\nstatus: done\ncreated_at: {}\ncompleted_at: {}\nerror: null\nurl: {}\ntool: \"{}\"\n---\n\n{}\n",
        step_name,
        desc_escaped,
        step_input,
        created_at,
        completed_at,
        url,
        tool_escaped,
        result_text,
    );

    let temp_path = output_dir.join(format!(".{}.md.tmp", step_name));
    fs::write(&temp_path, &file_content)
        .map_err(|e| format!("Failed to write output: {}", e))?;
    fs::rename(&temp_path, &output_path)
        .map_err(|e| format!("Failed to finalize output: {}", e))?;

    Ok(output_path)
}

/// Extract human-readable text from a JSON-RPC tools/call result.
/// Handles both text content items and raw string results.
fn extract_result_text(rpc: &serde_json::Value) -> String {
    // result.content is an array of content items per MCP spec
    if let Some(content) = rpc.pointer("/result/content") {
        if let Some(arr) = content.as_array() {
            let texts: Vec<&str> = arr
                .iter()
                .filter_map(|item| {
                    if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                        item.get("text").and_then(|t| t.as_str())
                    } else {
                        None
                    }
                })
                .collect();
            if !texts.is_empty() {
                return texts.join("\n");
            }
        }
    }
    // Fallback: stringify the whole result
    if let Some(result) = rpc.get("result") {
        if let Some(s) = result.as_str() {
            return s.to_string();
        }
        return serde_json::to_string_pretty(result).unwrap_or_default();
    }
    String::new()
}

/// Write a failure output .md file and return Err.
fn write_error(
    output_dir: &Path,
    step_name: &str,
    step_input: &str,
    step_description: Option<&str>,
    created_at: &str,
    url: &str,
    tool: &str,
    error: &str,
) -> Result<PathBuf, String> {
    let completed_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let _ = fs::create_dir_all(output_dir);
    let output_path = output_dir.join(format!("{}.md", step_name));
    let desc_escaped = step_description.unwrap_or("").replace('"', "\\\"");
    let err_escaped = error.replace('"', "\\\"").replace('\n', " ");
    let tool_escaped = tool.replace('"', "\\\"");
    let file_content = format!(
        "---\nname: {}\ndescription: \"{}\"\nconnector: mcp\ninput: {}\nstatus: failed\ncreated_at: {}\ncompleted_at: {}\nerror: \"{}\"\nurl: {}\ntool: \"{}\"\n---\n\n## Error\n{}\n",
        step_name,
        desc_escaped,
        step_input,
        created_at,
        completed_at,
        err_escaped,
        url,
        tool_escaped,
        error,
    );
    let temp_path = output_dir.join(format!(".{}.md.tmp", step_name));
    if let Err(e) = fs::write(&temp_path, &file_content) {
        eprintln!("[mcp] Failed to write error state for step '{}': {}", step_name, e);
    } else if let Err(e) = fs::rename(&temp_path, &output_path) {
        eprintln!("[mcp] Failed to finalize error state for step '{}': {}", step_name, e);
    }
    Err(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_result_text_content_array() {
        let rpc = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "content": [
                    { "type": "text", "text": "Hello from MCP" }
                ]
            }
        });
        assert_eq!(extract_result_text(&rpc), "Hello from MCP");
    }

    #[test]
    fn test_extract_result_text_multiple_items() {
        let rpc = serde_json::json!({
            "result": {
                "content": [
                    { "type": "text", "text": "line one" },
                    { "type": "image", "url": "http://img" },
                    { "type": "text", "text": "line two" }
                ]
            }
        });
        assert_eq!(extract_result_text(&rpc), "line one\nline two");
    }

    #[test]
    fn test_extract_result_text_string_result() {
        let rpc = serde_json::json!({
            "result": "simple string"
        });
        assert_eq!(extract_result_text(&rpc), "simple string");
    }

    #[test]
    fn test_extract_result_text_empty() {
        let rpc = serde_json::json!({ "jsonrpc": "2.0", "id": 1 });
        assert_eq!(extract_result_text(&rpc), "");
    }
}
