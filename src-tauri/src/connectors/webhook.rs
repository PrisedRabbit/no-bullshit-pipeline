use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Webhook connector config parsed from step config JSON
#[derive(Debug)]
struct WebhookConfig {
    url: String,
    method: String,
    headers: HashMap<String, String>,
    body_format: String,
    timeout_sec: u64,
}

impl WebhookConfig {
    fn from_value(config: &serde_json::Value) -> Result<Self, String> {
        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("Webhook connector config missing 'url'")?
            .to_string();

        let method = config
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("POST")
            .to_uppercase();

        // Validate HTTP method
        if !["POST", "PUT", "PATCH"].contains(&method.as_str()) {
            return Err(format!(
                "Unsupported HTTP method '{}'. Must be POST, PUT, or PATCH",
                method
            ));
        }

        let mut headers = HashMap::new();
        if let Some(h) = config.get("headers").and_then(|v| v.as_object()) {
            for (key, value) in h {
                if let Some(v) = value.as_str() {
                    headers.insert(key.clone(), v.to_string());
                }
            }
        }

        let body_format = config
            .get("body_format")
            .and_then(|v| v.as_str())
            .unwrap_or("json")
            .to_string();

        let timeout_sec = config
            .get("timeout_sec")
            .and_then(|v| v.as_u64())
            .unwrap_or(30)
            .clamp(5, 300);

        Ok(WebhookConfig {
            url,
            method,
            headers,
            body_format,
            timeout_sec,
        })
    }
}

/// Sensitive header names that should be masked in metadata
const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "token",
    "secret",
    "key",
    "password",
    "x-api-key",
];

/// Mask sensitive header values for logging/metadata
fn sanitize_headers(headers: &HashMap<String, String>) -> HashMap<String, String> {
    headers
        .iter()
        .map(|(k, v)| {
            let is_sensitive = SENSITIVE_HEADERS
                .iter()
                .any(|s| k.to_lowercase().contains(s));
            if is_sensitive {
                (k.clone(), "***".to_string())
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect()
}

/// Format headers as YAML for frontmatter
fn format_headers_yaml(headers: &HashMap<String, String>, indent: &str) -> String {
    if headers.is_empty() {
        return format!("{}{{}}", indent);
    }
    let lines: Vec<String> = headers
        .iter()
        .map(|(k, v)| format!("{}  {}: {}", indent, k, v))
        .collect();
    format!("\n{}", lines.join("\n"))
}

/// Execute the Webhook connector for a pipeline step
pub async fn execute(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    step_input: &str,
    step_description: Option<&str>,
) -> Result<PathBuf, String> {
    let webhook_config = WebhookConfig::from_value(config)?;
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    // Read input content
    let raw_content = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input file: {}", e))?;

    // Strip frontmatter if present
    let content = super::strip_frontmatter(&raw_content);

    // Build HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(webhook_config.timeout_sec))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Build request
    let mut request_builder = match webhook_config.method.as_str() {
        "POST" => client.post(&webhook_config.url),
        "PUT" => client.put(&webhook_config.url),
        "PATCH" => client.patch(&webhook_config.url),
        _ => return Err(format!("Unsupported method: {}", webhook_config.method)),
    };

    // Add User-Agent
    request_builder = request_builder.header("User-Agent", "NBP/0.4.0");

    // Add custom headers
    for (key, value) in &webhook_config.headers {
        request_builder = request_builder.header(key.as_str(), value.as_str());
    }

    // Set body based on format
    request_builder = match webhook_config.body_format.as_str() {
        "json" => request_builder
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "content": content })),
        _ => request_builder
            .header("Content-Type", "text/plain")
            .body(content.to_string()),
    };

    // Send request
    let result = request_builder.send().await;

    // Sanitized headers for metadata
    let safe_headers = sanitize_headers(&webhook_config.headers);

    // Ensure output directory exists
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    let metadata_path = output_dir.join(format!("{}.md", step_name));

    match result {
        Ok(response) => {
            let status_code = response.status().as_u16();
            let status_text = response.status().to_string();
            let is_success = response.status().is_success();

            // Read response body (limit to 10KB)
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read response>".to_string());
            let body_truncated = if body.len() > 10240 {
                format!("{}...(truncated)", &body[..10240])
            } else {
                body
            };

            let completed_at =
                Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

            let (status_str, error_str, body_text) = if is_success {
                (
                    "done".to_string(),
                    "null".to_string(),
                    format!(
                        "Webhook request successful\nStatus: {}",
                        status_text
                    ),
                )
            } else {
                let err_msg = format!("HTTP {}", status_text);
                (
                    "failed".to_string(),
                    format!("\"{}\"", err_msg.replace('"', "\\\"")),
                    format!(
                        "Webhook request failed\nStatus: {}\nError: {}",
                        status_text, body_truncated
                    ),
                )
            };

            let headers_yaml = format_headers_yaml(&safe_headers, "    ");
            let metadata_content = format!(
                "---\nname: {}\ndescription: \"{}\"\nconnector: webhook\ninput: {}\nstatus: {}\ncreated_at: {}\ncompleted_at: {}\nerror: {}\nrequest:\n  url: {}\n  method: {}\n  headers: {}\nresponse:\n  status: {}\n  body: '{}'\n---\n\n{}\n",
                step_name,
                step_description.unwrap_or("").replace('"', "\\\""),
                step_input,
                status_str,
                created_at,
                completed_at,
                error_str,
                webhook_config.url,
                webhook_config.method,
                headers_yaml,
                status_code,
                body_truncated.replace('\'', "\\'"),
                body_text,
            );

            let temp_path = output_dir.join(format!(".{}.md.tmp", step_name));
            fs::write(&temp_path, &metadata_content)
                .map_err(|e| format!("Failed to write metadata: {}", e))?;
            fs::rename(&temp_path, &metadata_path)
                .map_err(|e| format!("Failed to finalize metadata: {}", e))?;

            if is_success {
                Ok(metadata_path)
            } else {
                Err(format!("HTTP {}", status_text))
            }
        }
        Err(error) => {
            let completed_at =
                Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

            let err_msg = if error.is_timeout() {
                format!("Request timed out after {}s", webhook_config.timeout_sec)
            } else if error.is_connect() {
                format!("Connection failed: {}", error)
            } else {
                format!("Request failed: {}", error)
            };

            let headers_yaml = format_headers_yaml(&safe_headers, "    ");
            let metadata_content = format!(
                "---\nname: {}\ndescription: \"{}\"\nconnector: webhook\ninput: {}\nstatus: failed\ncreated_at: {}\ncompleted_at: {}\nerror: \"{}\"\nrequest:\n  url: {}\n  method: {}\n  headers: {}\nresponse:\n  status: 0\n  body: ''\n---\n\nWebhook request failed\n{}\n",
                step_name,
                step_description.unwrap_or("").replace('"', "\\\""),
                step_input,
                created_at,
                completed_at,
                err_msg.replace('"', "\\\""),
                webhook_config.url,
                webhook_config.method,
                headers_yaml,
                err_msg,
            );

            let temp_path = output_dir.join(format!(".{}.md.tmp", step_name));
            if let Err(write_err) = fs::write(&temp_path, &metadata_content) {
                eprintln!(
                    "Warning: Failed to write webhook error state for step '{}': {}",
                    step_name, write_err
                );
            } else if let Err(rename_err) = fs::rename(&temp_path, &metadata_path) {
                eprintln!(
                    "Warning: Failed to finalize webhook error state for step '{}': {}",
                    step_name, rename_err
                );
            }

            Err(err_msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_config_from_value() {
        let config = serde_json::json!({
            "url": "https://hooks.example.com/test",
            "method": "POST",
            "headers": {
                "Authorization": "Bearer secret",
                "X-Custom": "value"
            },
            "body_format": "json",
            "timeout_sec": 60
        });
        let wc = WebhookConfig::from_value(&config).unwrap();
        assert_eq!(wc.url, "https://hooks.example.com/test");
        assert_eq!(wc.method, "POST");
        assert_eq!(wc.headers.len(), 2);
        assert_eq!(wc.body_format, "json");
        assert_eq!(wc.timeout_sec, 60);
    }

    #[test]
    fn test_webhook_config_defaults() {
        let config = serde_json::json!({
            "url": "https://example.com/webhook"
        });
        let wc = WebhookConfig::from_value(&config).unwrap();
        assert_eq!(wc.method, "POST");
        assert_eq!(wc.body_format, "json");
        assert_eq!(wc.timeout_sec, 30);
    }

    #[test]
    fn test_webhook_config_missing_url() {
        let config = serde_json::json!({
            "method": "POST"
        });
        let result = WebhookConfig::from_value(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("url"));
    }

    #[test]
    fn test_webhook_config_invalid_method() {
        let config = serde_json::json!({
            "url": "https://example.com",
            "method": "DELETE"
        });
        let result = WebhookConfig::from_value(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported HTTP method"));
    }

    #[test]
    fn test_webhook_config_timeout_clamped() {
        let config = serde_json::json!({
            "url": "https://example.com",
            "timeout_sec": 1
        });
        let wc = WebhookConfig::from_value(&config).unwrap();
        assert_eq!(wc.timeout_sec, 5); // Clamped to minimum

        let config2 = serde_json::json!({
            "url": "https://example.com",
            "timeout_sec": 1000
        });
        let wc2 = WebhookConfig::from_value(&config2).unwrap();
        assert_eq!(wc2.timeout_sec, 300); // Clamped to maximum
    }

    #[test]
    fn test_sanitize_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer secret123".to_string());
        headers.insert("X-Custom".to_string(), "visible-value".to_string());
        headers.insert("X-API-Key".to_string(), "key123".to_string());

        let sanitized = sanitize_headers(&headers);
        assert_eq!(sanitized.get("Authorization").unwrap(), "***");
        assert_eq!(sanitized.get("X-Custom").unwrap(), "visible-value");
        assert_eq!(sanitized.get("X-API-Key").unwrap(), "***");
    }

    #[test]
    fn test_sanitize_headers_case_insensitive() {
        let mut headers = HashMap::new();
        headers.insert("x-secret-token".to_string(), "hidden".to_string());
        headers.insert("X-PASSWORD-Header".to_string(), "hidden".to_string());

        let sanitized = sanitize_headers(&headers);
        assert_eq!(sanitized.get("x-secret-token").unwrap(), "***");
        assert_eq!(sanitized.get("X-PASSWORD-Header").unwrap(), "***");
    }

    #[test]
    fn test_strip_frontmatter() {
        let content = "---\nstatus: done\n---\n\nBody content";
        assert_eq!(super::super::strip_frontmatter(content), "Body content");
    }

    #[test]
    fn test_strip_frontmatter_no_frontmatter() {
        let content = "Plain text content";
        assert_eq!(super::super::strip_frontmatter(content), "Plain text content");
    }

    #[test]
    fn test_format_headers_yaml_empty() {
        let headers = HashMap::new();
        let yaml = format_headers_yaml(&headers, "    ");
        assert_eq!(yaml, "    {}");
    }

    #[test]
    fn test_format_headers_yaml_with_entries() {
        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "value".to_string());
        let yaml = format_headers_yaml(&headers, "    ");
        assert!(yaml.contains("X-Custom: value"));
    }
}
