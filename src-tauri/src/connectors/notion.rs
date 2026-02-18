use std::path::{Path, PathBuf};
use std::fs;
use std::collections::BTreeMap;
use chrono::Utc;
use notion_client::endpoints::Client;
use notion_client::endpoints::pages::create::request::CreateAPageRequest;
use notion_client::objects::page::{PageProperty, SelectPropertyValue, DatePropertyValue, DateOrDateTime};
use notion_client::objects::parent::Parent;
use notion_client::objects::rich_text::{RichText, Text};
use notion_client::objects::user::User;
use crate::integrations::notion::{load_notion_profile, get_notion_token, NotionIntegrationProfile};

// ──────────────────────────────────────────────────────────────────────────────
// Config
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct NotionConnectorConfig {
    integration_id: String,
}

impl NotionConnectorConfig {
    fn from_value(config: &serde_json::Value) -> Result<Self, String> {
        let integration_id = config
            .get("integration_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                "Notion connector config missing 'integration_id'. \
                 Add integration_id to the step config in the pipeline definition."
                    .to_string()
            })?
            .to_string();
        Ok(NotionConnectorConfig { integration_id })
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// JSON extraction from LLM output
// ──────────────────────────────────────────────────────────────────────────────

/// Extract a JSON array from LLM output content.
/// Handles:
///   1. Bare JSON array (after stripping YAML frontmatter)
///   2. JSON array wrapped in ```json ... ``` code fence
///   3. JSON array wrapped in ``` ... ``` bare code fence
///
/// Returns a descriptive error showing the raw content on parse failure.
fn extract_json_array(content: &str) -> Result<Vec<serde_json::Value>, String> {
    let body = crate::connectors::strip_frontmatter(content);
    let trimmed = body.trim();

    // Try direct parse (bare JSON array)
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(trimmed) {
        return Ok(arr);
    }

    // Try extracting from ```json ... ``` fence
    if let Some(fence_start) = trimmed.find("```json") {
        let after_fence = &trimmed[fence_start + 7..];
        if let Some(fence_end) = after_fence.find("```") {
            let json_str = after_fence[..fence_end].trim();
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
                return Ok(arr);
            }
        }
    }

    // Try extracting from ``` (no language tag) fence
    if let Some(fence_start) = trimmed.find("```\n") {
        let after_fence = &trimmed[fence_start + 4..];
        if let Some(fence_end) = after_fence.find("```") {
            let json_str = after_fence[..fence_end].trim();
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
                return Ok(arr);
            }
        }
    }

    // Parse failed — return descriptive error with raw LLM output (AUGM-05)
    let preview = &trimmed[..trimmed.len().min(500)];
    Err(format!(
        "Notion connector: could not parse LLM output as JSON array.\n\
         Expected a JSON array like: [{{\"Title\": \"...\", ...}}]\n\
         Raw LLM output (first 500 chars): {}",
        preview
    ))
}

// ──────────────────────────────────────────────────────────────────────────────
// LLM output validation
// ──────────────────────────────────────────────────────────────────────────────

/// Writable property types — used for validation to check that LLM output
/// contains at least one key matching a property the connector can map.
const WRITABLE_TYPES: &[&str] = &[
    "title", "rich_text", "select", "multi_select", "people",
    "date", "number", "checkbox", "url", "email", "phone_number", "status",
];

/// Validate that each item in the parsed JSON array has at least one key
/// matching a writable property from the integration profile.
///
/// Called between `extract_json_array()` and `build_notion_properties()` in
/// the execute flow. On failure, returns a descriptive error including the
/// raw LLM output so the user can see what went wrong.
fn validate_llm_output_for_notion(
    items: &[serde_json::Value],
    profile: &NotionIntegrationProfile,
    raw_output: &str,
) -> Result<(), String> {
    if items.is_empty() {
        return Err(format!(
            "Notion connector: LLM output parsed as empty JSON array — no pages to create.\n\
             Raw LLM output (first 500 chars): {}",
            &raw_output[..raw_output.len().min(500)]
        ));
    }

    // Collect writable property names from the profile
    let writable_names: std::collections::HashSet<&str> = profile.properties.iter()
        .filter(|p| WRITABLE_TYPES.contains(&p.property_type.as_str()))
        .map(|p| p.name.as_str())
        .collect();

    for (idx, item) in items.iter().enumerate() {
        let obj = match item.as_object() {
            Some(o) => o,
            None => return Err(format!(
                "Notion connector: JSON array element {} is not an object.\n\
                 Expected objects like {{\"Title\": \"...\", ...}}\n\
                 Raw LLM output (first 500 chars): {}",
                idx,
                &raw_output[..raw_output.len().min(500)]
            )),
        };

        // Check that at least one key matches a writable profile property
        let has_valid_key = obj.keys().any(|k| writable_names.contains(k.as_str()));
        if !has_valid_key {
            return Err(format!(
                "Notion connector: JSON array element {} has no keys matching the database schema.\n\
                 Expected property names: {}\n\
                 Got keys: {}\n\
                 Raw LLM output (first 500 chars): {}",
                idx,
                writable_names.iter().copied().collect::<Vec<_>>().join(", "),
                obj.keys().cloned().collect::<Vec<_>>().join(", "),
                &raw_output[..raw_output.len().min(500)]
            ));
        }
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Select value resolution
// ──────────────────────────────────────────────────────────────────────────────

/// Resolve a select/status value against the profile's known options using
/// case-insensitive matching. Returns the canonical (profile) casing on match,
/// or the original value unchanged if no match (passes through to Notion API).
fn resolve_select_value(
    value: &str,
    property_name: &str,
    profile: &NotionIntegrationProfile,
) -> String {
    // Find the property definition in the profile
    let prop_def = profile
        .properties
        .iter()
        .find(|p| p.name == property_name);

    let Some(prop_def) = prop_def else {
        // Property not in profile — pass value through unchanged
        return value.to_string();
    };

    if prop_def.select_options.is_empty() {
        // Empty options (e.g. Status property with unextracted options) — pass through
        return value.to_string();
    }

    // Case-insensitive match against known options
    match prop_def
        .select_options
        .iter()
        .find(|opt| opt.eq_ignore_ascii_case(value))
    {
        Some(canonical) => canonical.clone(),
        // Value not in known options — pass through; Notion API will reject if truly invalid
        None => value.to_string(),
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// People alias resolution
// ──────────────────────────────────────────────────────────────────────────────

/// Resolve an array of alias values to Notion User structs via the profile's
/// people_mappings. Aliases that don't match any mapping are silently skipped.
fn resolve_people_aliases(
    aliases: &[serde_json::Value],
    profile: &NotionIntegrationProfile,
) -> Vec<User> {
    aliases
        .iter()
        .filter_map(|alias_val| {
            let alias_str = alias_val.as_str()?;
            // Find mapping by alias (case-insensitive)
            let mapping = profile
                .people_mappings
                .iter()
                .find(|m| m.alias.eq_ignore_ascii_case(alias_str))?;
            // Construct User — notion-client User may not implement Default,
            // so we explicitly set all known fields.
            // Note: "avator_url" is the crate's typo (not "avatar_url").
            Some(User {
                id: mapping.notion_user_id.clone(),
                name: Some(mapping.display_name.clone()),
                avator_url: None,
                user_type: None,
            })
        })
        .collect()
}

// ──────────────────────────────────────────────────────────────────────────────
// Property building
// ──────────────────────────────────────────────────────────────────────────────

/// Build the `BTreeMap<String, PageProperty>` needed for `CreateAPageRequest`.
/// Iterates the profile's property definitions (not the JSON keys) to ensure
/// only known schema properties are sent to Notion.
///
/// Returns an error if the resulting property map is empty (nothing could be mapped).
fn build_notion_properties(
    item: &serde_json::Value,
    profile: &NotionIntegrationProfile,
) -> Result<BTreeMap<String, PageProperty>, String> {
    let obj = item
        .as_object()
        .ok_or_else(|| "JSON item is not an object — expected a JSON object with property names as keys".to_string())?;

    let mut properties: BTreeMap<String, PageProperty> = BTreeMap::new();

    for prop_def in &profile.properties {
        let value = match obj.get(&prop_def.name) {
            Some(v) => v,
            None => continue, // LLM didn't provide this property — skip
        };

        // Skip null values entirely
        if value.is_null() {
            continue;
        }

        let page_prop = match prop_def.property_type.as_str() {
            "title" => {
                let text = value.as_str().unwrap_or("").to_string();
                PageProperty::Title {
                    id: None,
                    title: vec![RichText::Text {
                        text: Text {
                            content: text,
                            link: None,
                        },
                        annotations: None,
                        plain_text: None,
                        href: None,
                    }],
                }
            }

            "rich_text" => {
                let text = value.as_str().unwrap_or("").to_string();
                PageProperty::RichText {
                    id: None,
                    rich_text: vec![RichText::Text {
                        text: Text {
                            content: text,
                            link: None,
                        },
                        annotations: None,
                        plain_text: None,
                        href: None,
                    }],
                }
            }

            "select" => {
                let raw = value.as_str().unwrap_or("");
                let canonical = resolve_select_value(raw, &prop_def.name, profile);
                PageProperty::Select {
                    id: None,
                    select: Some(SelectPropertyValue {
                        id: None,
                        name: Some(canonical),
                        color: None,
                    }),
                }
            }

            "multi_select" => {
                let values: Vec<SelectPropertyValue> = match value {
                    serde_json::Value::Array(arr) => arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| resolve_select_value(s, &prop_def.name, profile))
                        .map(|name| SelectPropertyValue {
                            id: None,
                            name: Some(name),
                            color: None,
                        })
                        .collect(),
                    serde_json::Value::String(s) => {
                        vec![SelectPropertyValue {
                            id: None,
                            name: Some(resolve_select_value(s, &prop_def.name, profile)),
                            color: None,
                        }]
                    }
                    _ => vec![],
                };
                PageProperty::MultiSelect {
                    id: None,
                    multi_select: values,
                }
            }

            "people" => {
                let aliases: Vec<serde_json::Value> = match value {
                    serde_json::Value::Array(arr) => arr.clone(),
                    serde_json::Value::String(s) => {
                        vec![serde_json::Value::String(s.clone())]
                    }
                    // null was already skipped above; other types: skip property
                    _ => continue,
                };
                let users = resolve_people_aliases(&aliases, profile);
                // Do NOT send empty people array — could clear existing assignees
                if users.is_empty() {
                    continue;
                }
                PageProperty::People {
                    id: None,
                    people: users,
                }
            }

            "date" => {
                let date_str = value.as_str().unwrap_or("").to_string();
                // Use DateOrDateTime::DateTime for strings containing 'T' (ISO 8601 datetime),
                // otherwise DateOrDateTime::Date for date-only strings.
                let date_value = if date_str.contains('T') {
                    DateOrDateTime::DateTime(date_str)
                } else {
                    DateOrDateTime::Date(date_str)
                };
                PageProperty::Date {
                    id: None,
                    date: Some(DatePropertyValue {
                        start: Some(date_value),
                        end: None,
                        time_zone: None,
                    }),
                }
            }

            "number" => {
                // Convert to f64 then to serde_json::Number for the PageProperty::Number variant.
                // The notion-client Number type is serde_json::Number (re-exported or aliased).
                if let Some(num_f64) = value.as_f64() {
                    if let Some(num) = serde_json::Number::from_f64(num_f64) {
                        PageProperty::Number {
                            id: None,
                            number: Some(num),
                        }
                    } else {
                        // NaN/Infinity — skip
                        continue;
                    }
                } else {
                    // Value is not a number — skip
                    continue;
                }
            }

            "checkbox" => PageProperty::Checkbox {
                id: None,
                checkbox: value.as_bool().unwrap_or(false),
            },

            "url" => PageProperty::Url {
                id: None,
                url: value.as_str().map(|s| s.to_string()),
            },

            "email" => PageProperty::Email {
                id: None,
                email: value.as_str().map(|s| s.to_string()),
            },

            "phone_number" => PageProperty::PhoneNumber {
                id: None,
                phone_number: value.as_str().map(|s| s.to_string()),
            },

            "status" => {
                // Treat status like select — resolve_select_value passes through when options empty
                let raw = value.as_str().unwrap_or("");
                let canonical = resolve_select_value(raw, &prop_def.name, profile);
                PageProperty::Status {
                    id: None,
                    status: Some(SelectPropertyValue {
                        id: None,
                        name: Some(canonical),
                        color: None,
                    }),
                }
            }

            // Computed/read-only property types — skip silently
            "formula"
            | "rollup"
            | "relation"
            | "created_time"
            | "last_edited_time"
            | "created_by"
            | "last_edited_by"
            | "unique_id"
            | "unknown" => continue,

            _ => continue,
        };

        properties.insert(prop_def.name.clone(), page_prop);
    }

    if properties.is_empty() {
        return Err(
            "No properties could be mapped from LLM output to Notion schema. \
             Verify the integration profile is synced (Settings > Integrations > Sync Schema), \
             and that the LLM output contains at least one key matching a writable property name."
                .to_string(),
        );
    }

    Ok(properties)
}

// ──────────────────────────────────────────────────────────────────────────────
// Execute entry point
// ──────────────────────────────────────────────────────────────────────────────

/// Execute Notion connector: parse LLM JSON output, build Notion PageProperty maps,
/// and create one Notion database page per JSON array element.
///
/// Matches the standard connector signature used by pipeline_engine.rs.
pub async fn execute(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    input_step: &str,
    description: Option<&str>,
) -> Result<PathBuf, String> {
    // Parse connector config
    let connector_config = NotionConnectorConfig::from_value(config)?;

    // Load integration profile from disk
    let profile = load_notion_profile(&connector_config.integration_id)?;

    // Get Notion API token (never logged or included in errors)
    let token = get_notion_token(&connector_config.integration_id)?;

    // Create Notion client
    let client = Client::new(token, None)
        .map_err(|e| format!("Failed to create Notion client: {:?}", e))?;

    // Read the input file (previous step's output)
    let raw = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input file '{}': {}", input_path.display(), e))?;

    // Extract JSON array from LLM output (handles bare JSON and code fences)
    let items = extract_json_array(&raw)?;

    // Validate LLM output structure against the integration profile schema (AUGM-04, AUGM-05).
    // Fails with a clear error + raw output if structure doesn't match.
    validate_llm_output_for_notion(&items, &profile, &raw)?;

    // Create one Notion page per JSON array element
    let mut page_ids: Vec<String> = Vec::new();
    for item in &items {
        let properties = build_notion_properties(item, &profile)?;

        let request = CreateAPageRequest {
            parent: Parent::DatabaseId {
                database_id: profile.database_id.clone(),
            },
            icon: None,
            cover: None,
            properties,
            children: None,
        };

        let page = client
            .pages
            .create_a_page(request)
            .await
            .map_err(|e| format!("Failed to create Notion page: {:?}", e))?;

        page_ids.push(page.id.clone());
    }

    // Write output markdown with YAML frontmatter
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let output_path = output_dir.join(format!("{}.md", step_name));
    let now = Utc::now().to_rfc3339();
    let pages_summary = if page_ids.is_empty() {
        "No pages created (empty input array)".to_string()
    } else {
        format!("Created {} Notion page(s): {}", page_ids.len(), page_ids.join(", "))
    };

    let frontmatter = format!(
        r#"---
name: {}
description: "{}"
connector: notion
input: {}
status: done
created_at: {}
completed_at: {}
integration_id: {}
pages_created: {}
error: null
---

{}
"#,
        step_name,
        description.unwrap_or("Create Notion pages"),
        input_step,
        now,
        now,
        connector_config.integration_id,
        page_ids.len(),
        pages_summary
    );

    fs::write(&output_path, frontmatter)
        .map_err(|e| format!("Failed to write output file: {}", e))?;

    Ok(output_path)
}
