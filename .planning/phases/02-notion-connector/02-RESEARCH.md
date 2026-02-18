# Phase 2: Notion Connector - Research

**Researched:** 2026-02-18
**Domain:** notion-client page creation API, JSON structured output parsing, property type mapping, pipeline engine extension
**Confidence:** HIGH — notion-client PageProperty and CreateAPageRequest verified via GitHub source; existing codebase analyzed directly; connector pattern established by slack.rs

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| NOTN-03 | Setup wizard: user picks database from list fetched via Notion API | Already implemented in Phase 1 (`list_notion_databases`); Phase 2 adds the pipeline step config (`integration_id`) that references the chosen database — no new API calls needed |
| NOTN-04 | Setup wizard: app reads database schema (properties, select options, people) automatically | Already implemented in Phase 1 (`sync_notion_schema`); Phase 2 reads from the stored profile — no new API calls needed at pipeline execution time |
| NOTN-05 | Setup wizard: user maps conversation aliases to Notion workspace users (people mapping) | Already implemented in Phase 1 (`update_notion_people_mappings`); Phase 2 consumes those mappings via `resolve_people_aliases()` at execution time |
| NOTN-08 | Notion connector creates pages in the selected database with structured property values | Requires new `connectors/notion.rs` with `execute()` entry point; uses `notion_client::endpoints::pages::create::request::CreateAPageRequest` with `BTreeMap<String, PageProperty>`; page parent is `Parent::DatabaseId { database_id }` |
| EXEC-04 | Notion connector normalizes select values (case-insensitive match) and resolves people aliases to user IDs | `resolve_select_value()` does case-insensitive search in `profile.properties[prop_name].select_options`; `resolve_people_aliases()` maps alias strings to `User { id }` structs via `profile.people_mappings`; people property is `PageProperty::People { people: Vec<User> }` |
</phase_requirements>

---

## Summary

Phase 2 builds two Rust files and extends one: `connectors/notion.rs` (new), and extensions to `pipelines.rs` and `pipeline_engine.rs`. All Phase 1 infrastructure is already in place — credentials, profile I/O, schema sync, people mappings. Phase 2 consumes those at pipeline execution time.

The core challenge is property type mapping: the `notion-client` crate's `PageProperty` enum has ~20 variants, each with specific inner types. The connector must translate a `serde_json::Value` (from LLM output) into the correct `PageProperty` variant using the `property_type` strings stored in `NotionIntegrationProfile.properties`. This mapping table is the most critical piece to get right.

JSON extraction from LLM output requires tolerating markdown code fences (LLMs frequently wrap JSON in ` ```json ... ``` `). The extractor must try direct parse first, then strip the fence and retry. A parse failure must return a descriptive error showing what the raw content looked like, not a generic error. This is the "hard fail" decision made pre-roadmap: no silent fallthrough.

**Primary recommendation:** Implement `connectors/notion.rs` following the exact same `execute()` signature as `connectors/slack.rs`. Add `ConnectorType::Notion` to `pipelines.rs` with an `integration_id` field in the step config. Wire into `pipeline_engine.rs` with a new match arm. Do NOT add a `jsonschema` crate — serde_json's own `from_str::<Vec<serde_json::Value>>` is sufficient for JSON array validation; the schema enforcement happens implicitly when mapping properties.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `notion-client` | `1.0.11` (already in Cargo.toml) | Page creation via `create_a_page()` | Already added in Phase 1; page creation is in `client.pages.create_a_page(request)` |
| `serde_json` | `1.x` (already in Cargo.toml) | Parse LLM JSON output, build property map | Zero new dependencies; `serde_json::from_str` is the only JSON parser needed |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `chrono` | `0.4.43` (already in Cargo.toml) | ISO 8601 date strings in `DatePropertyValue` | When mapping a date string from LLM output to `DateOrDateTime::Date` |
| `reqwest` | `0.13` (already in Cargo.toml) | Already used by notion-client internally | No direct use by connector — notion-client wraps it |

### No New Dependencies Required

All Phase 2 work uses only libraries already in `Cargo.toml`. The roadmap plan mentioned adding `jsonschema` — **do not add it**. The `serde_json` array parse is the validation step; no JSON Schema validation is needed for v1.

### Installation

No changes to `Cargo.toml` needed.

---

## Architecture Patterns

### Recommended File Changes

```
src-tauri/src/
├── connectors/
│   ├── mod.rs         — ADD: pub mod notion;
│   ├── notion.rs      — CREATE: execute(), extract_json_array(),
│   │                            resolve_people_aliases(), resolve_select_value(),
│   │                            build_notion_properties()
│   └── slack.rs       — No changes (reference pattern)
│
├── pipelines.rs       — EXTEND: add ConnectorType::Notion variant,
│                                add validation for Notion config
│
└── pipeline_engine.rs — EXTEND: add ConnectorType::Notion arm in match
```

### Pattern 1: Connector Execute Signature (from slack.rs)

**What:** All connectors expose an identical `execute()` signature. The pipeline engine calls this function directly. The connector writes its output to `{output_dir}/{step_name}.md` with YAML frontmatter.

**When to use:** Always — this is the contract with `pipeline_engine.rs`.

```rust
// Source: /workspace/src-tauri/src/connectors/slack.rs — verified pattern
pub async fn execute(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    input_step: &str,
    description: Option<&str>,
) -> Result<PathBuf, String>
```

The function must:
1. Read `input_path` content (the previous step's `.md` output)
2. Strip frontmatter via `crate::connectors::strip_frontmatter()`
3. Parse the JSON body
4. Call the Notion API
5. Write `{output_dir}/{step_name}.md` with frontmatter (status: done/failed)
6. Return `Ok(output_path)` or `Err(descriptive_message)`

### Pattern 2: Notion Page Creation API

**What:** `client.pages.create_a_page(request)` creates a page in a database. The request uses a `BTreeMap<String, PageProperty>` for properties. The parent must be `Parent::DatabaseId { database_id }`.

**Verified source:** `github.com/takassh/notion-client` — `src/endpoints/pages/create/request.rs` and `src/objects/page.rs`

```rust
// Source: github.com/takassh/notion-client/blob/main/src/endpoints/pages/create/request.rs
use std::collections::BTreeMap;
use notion_client::{
    endpoints::pages::create::request::CreateAPageRequest,
    objects::{
        page::{PageProperty, SelectPropertyValue, DatePropertyValue, DateOrDateTime},
        parent::Parent,
        rich_text::{RichText, Text},
        user::User,
    },
};

let mut properties: BTreeMap<String, PageProperty> = BTreeMap::new();

// Title property (every page must have at least one title)
properties.insert("Name".to_string(), PageProperty::Title {
    id: None,
    title: vec![RichText::Text {
        text: Text { content: "My page title".to_string(), link: None },
        annotations: None,
        plain_text: None,
        href: None,
    }],
});

// Select property
properties.insert("Priority".to_string(), PageProperty::Select {
    id: None,
    select: Some(SelectPropertyValue {
        id: None,
        name: Some("High".to_string()),
        color: None,
    }),
});

// People property (array of User structs — only id field needed)
properties.insert("Assignee".to_string(), PageProperty::People {
    id: None,
    people: vec![User {
        id: "notion-user-uuid".to_string(),
        // other fields optional/default
        ..Default::default()
    }],
});

let request = CreateAPageRequest {
    parent: Parent::DatabaseId {
        database_id: profile.database_id.clone(),
    },
    icon: None,
    cover: None,
    properties,
    children: None,
};

let _page = client.pages.create_a_page(request).await
    .map_err(|e| format!("Failed to create Notion page: {:?}", e))?;
```

### Pattern 3: PageProperty Variant Mapping Table

**What:** The connector receives a `serde_json::Value` object from LLM output and must convert each field to the correct `PageProperty` variant based on the `property_type` stored in the integration profile.

**Verified source:** `github.com/takassh/notion-client/blob/main/src/objects/page.rs`

| property_type | PageProperty variant | Inner type | Notes |
|--------------|---------------------|-----------|-------|
| `title` | `PageProperty::Title { title: Vec<RichText> }` | `RichText::Text { text: Text { content } }` | Required field; page creation fails without it |
| `rich_text` | `PageProperty::RichText { rich_text: Vec<RichText> }` | Same as title | Use for long text bodies |
| `select` | `PageProperty::Select { select: Option<SelectPropertyValue> }` | `SelectPropertyValue { name: Some(..), .. }` | Must run `resolve_select_value()` first for case normalization |
| `multi_select` | `PageProperty::MultiSelect { multi_select: Vec<SelectPropertyValue> }` | Array of `SelectPropertyValue` | Each value must be case-normalized |
| `people` | `PageProperty::People { people: Vec<User> }` | `User { id, .. }` | Must run `resolve_people_aliases()` to get user IDs |
| `date` | `PageProperty::Date { date: Option<DatePropertyValue> }` | `DatePropertyValue { start: Some(DateOrDateTime::Date(..)) }` | LLM output must be ISO 8601 date string |
| `number` | `PageProperty::Number { number: Option<Number> }` | JSON number parsed via serde | `Number` is likely `f64` or `serde_json::Number` — verify |
| `checkbox` | `PageProperty::Checkbox { checkbox: bool }` | `bool` | |
| `url` | `PageProperty::Url { url: Option<String> }` | `String` | |
| `email` | `PageProperty::Email { email: Option<String> }` | `String` | |
| `phone_number` | `PageProperty::PhoneNumber { phone_number: Option<String> }` | `String` | |
| `status` | `PageProperty::Status { status: Option<SelectPropertyValue> }` | Same as select | Treat like select for write purposes |

**Skip these when writing (computed/read-only):**
- `formula`, `rollup`, `relation`, `created_time`, `last_edited_time`, `created_by`, `last_edited_by`, `unique_id`

### Pattern 4: JSON Extraction with Code Fence Tolerance

**What:** LLMs frequently output JSON wrapped in ` ```json ... ``` ` fences. The extractor must handle: bare JSON array, code-fenced JSON array, and extra whitespace. Must return a clear error on failure.

**When to use:** First operation inside `execute()` after stripping frontmatter.

```rust
// Source: ARCHITECTURE.md Pattern 3 (verified concept) + direct extension
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
        // Find the closing ```
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

    // Parse error: show first 500 chars of raw output
    let preview = &trimmed[..trimmed.len().min(500)];
    Err(format!(
        "Notion connector: could not parse LLM output as JSON array.\n\
         Expected a JSON array like: [{\"Title\": \"...\", ...}]\n\
         Got: {}",
        preview
    ))
}
```

### Pattern 5: Case-Insensitive Select Resolution

**What:** The LLM may output `"high"` when the Notion option is `"High"`. The connector must find the correct casing from the profile's known select options.

**When to use:** For every `select`, `multi_select`, and `status` property value.

```rust
// Source: EXEC-04 requirement; pattern designed from ARCHITECTURE.md
fn resolve_select_value(
    value: &str,
    property_name: &str,
    profile: &NotionIntegrationProfile,
) -> Result<String, String> {
    // Find the property definition in the profile
    let prop_def = profile.properties.iter()
        .find(|p| p.name == property_name);

    let Some(prop_def) = prop_def else {
        // Property not in profile — pass value through unchanged
        return Ok(value.to_string());
    };

    if prop_def.select_options.is_empty() {
        return Ok(value.to_string());
    }

    // Case-insensitive match against known options
    let normalized = prop_def.select_options.iter()
        .find(|opt| opt.eq_ignore_ascii_case(value));

    match normalized {
        Some(canonical) => Ok(canonical.clone()),
        None => {
            // Value not in known options — pass through; Notion API will reject if invalid
            // (better to let Notion give a clear "unknown option" error than silently drop)
            Ok(value.to_string())
        }
    }
}
```

### Pattern 6: People Alias Resolution

**What:** The LLM outputs a person alias like `"SK"` or `"Sergey"`. The connector must look up this alias in the profile's `people_mappings` to get the Notion user UUID.

**When to use:** For every `people` property value.

```rust
// Source: EXEC-04 requirement; people_mappings defined in Phase 1 NotionIntegrationProfile
fn resolve_people_aliases(
    aliases: &[serde_json::Value],
    profile: &NotionIntegrationProfile,
) -> Vec<User> {
    aliases.iter().filter_map(|alias_val| {
        let alias_str = alias_val.as_str()?;
        // Find mapping by alias (case-insensitive)
        let mapping = profile.people_mappings.iter()
            .find(|m| m.alias.eq_ignore_ascii_case(alias_str))?;
        Some(User {
            id: mapping.notion_user_id.clone(),
            name: Some(mapping.display_name.clone()),
            ..Default::default()
        })
    }).collect()
}
```

**OPEN QUESTION:** The `User` struct in `notion-client 1.0.11` was confirmed to have `id: String` and `name: Option<String>` fields from the Phase 1 research. However, `..Default::default()` requires `User` to implement `Default`. Verify this during implementation. If `User` does not implement `Default`, construct it with all known fields explicitly.

### Pattern 7: ConnectorType::Notion in pipelines.rs

**What:** Add `Notion` variant to the existing `ConnectorType` enum. Add validation that requires `integration_id` in the config.

```rust
// Source: /workspace/src-tauri/src/pipelines.rs — extend existing enum
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectorType {
    Llm,
    Save,
    Webhook,
    Slack,
    Mcp,
    Notion,  // ADD THIS
}

// In validate_step_config(), add:
ConnectorType::Notion => {
    if step.config.get("integration_id").and_then(|v| v.as_str()).is_none() {
        return Err(format!(
            "Step '{}': Notion connector requires 'integration_id' in config",
            step.name
        ));
    }
}
```

### Pattern 8: pipeline_engine.rs Match Arm

**What:** Add `ConnectorType::Notion` arm to the `match step.connector` block in `execute_pipeline_internal()`.

```rust
// Source: /workspace/src-tauri/src/pipeline_engine.rs — extend existing match
ConnectorType::Notion => {
    connectors::notion::execute(
        &input_path,
        &step.config,
        &output_dir,
        &step.name,
        &step.input,
        step.description.as_deref(),
    )
    .await
}
```

### Anti-Patterns to Avoid

- **Fetching schema during execution:** Do NOT call `retrieve_a_database()` in `connectors/notion.rs`. Read from the stored profile JSON via `load_notion_profile()`.
- **Adding `jsonschema` crate:** Not needed. `serde_json::from_str::<Vec<Value>>` is the validation. The property mapping loop silently skips unknown fields — the LLM output doesn't need to match the schema exactly.
- **Generic Notion API errors:** When JSON parse fails, always show the raw LLM output (first N chars) in the error. Never say "Notion API error" when the problem is parse failure.
- **Silently skipping null people values:** If the LLM outputs `null` for a people field, skip that property entirely (don't send an empty array). An empty `people` array in a Notion page creation request may clear existing assignees.
- **Using `HashMap` for properties:** Use `BTreeMap<String, PageProperty>` — this is what `CreateAPageRequest` requires (verified from source).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Notion page creation HTTP request | Manual reqwest call to POST /v1/pages | `client.pages.create_a_page(request)` | Type-safe; handles auth header, versioning, error parsing |
| PageProperty type construction | Custom JSON serialization for property values | `PageProperty` enum variants from `notion-client` | Complex serde tag structure; using the enum is correct |
| JSON code fence detection | Regex | Plain string `find("```json")` with index arithmetic | Regex adds a dependency; string search is sufficient for this specific pattern |
| Select option case matching | Levenshtein distance / fuzzy match | `eq_ignore_ascii_case()` | Case-insensitive match is sufficient; fuzzy matching risks silently picking wrong option |

---

## Common Pitfalls

### Pitfall 1: User struct construction — Default trait availability

**What goes wrong:** `User { id: .., ..Default::default() }` fails with "the trait `Default` is not implemented for `User`" if the `notion-client` crate doesn't derive `Default` on `User`.

**Why it happens:** The `User` struct has many optional fields (name, avatar_url, user_type, etc.). The research from Phase 1 confirmed `id: String`, `name: Option<String>`, `avator_url: Option<String>`, `user_type: Option<UserType>`. If `Default` is not derived, the struct literal must be explicit.

**How to avoid:** Verify via `cargo doc` after Phase 1 ran. If `Default` is not available, construct `User` explicitly with all `Option` fields set to `None`:
```rust
User {
    id: mapping.notion_user_id.clone(),
    name: Some(mapping.display_name.clone()),
    avator_url: None,    // note: "avator" is the crate's typo
    user_type: None,
}
```

**Warning signs:** Compile error on `..Default::default()` in User construction.

### Pitfall 2: `Number` type in PageProperty::Number

**What goes wrong:** `PageProperty::Number { number: Option<Number> }` — the `Number` type may be a newtype wrapper, `f64`, or `serde_json::Number`. Using the wrong type causes a compile error.

**Why it happens:** The `notion-client` source shows `Number` in the objects module but the concrete type is not fully documented.

**How to avoid:** Check `notion_client::objects::Number` via `cargo doc`. It is likely `type Number = serde_json::Number` or a newtype. For the connector, convert the `serde_json::Value` to `f64` first via `.as_f64()`, then construct the Notion Number.

**Warning signs:** Compile error on `PageProperty::Number { number: Some(value) }` where `value` is the wrong type.

### Pitfall 3: Title property is required — page creation fails without it

**What goes wrong:** Creating a Notion page without a `Title` property type returns a 400 error from the API.

**Why it happens:** Every Notion database has one `title` type property (even if renamed). If the LLM output doesn't include a value for it, or if the connector skips it because the value is null/empty, the page creation will fail.

**How to avoid:** In `build_notion_properties()`, if the `title` property is present in the profile but missing from the LLM output, use an empty string (`""`) for the title rather than skipping the property. Alternatively, fail with a clear error: "LLM output missing required title field '{prop_name}'".

**Warning signs:** Notion API returns 400 with "body failed validation" message.

### Pitfall 4: `ConnectorType::Notion` serde serialization

**What goes wrong:** Adding `Notion` to `ConnectorType` with `#[serde(rename_all = "lowercase")]` will serialize it as `"notion"`. Existing pipeline JSON files saved before this change won't have `"notion"` values, but new pipelines will. No migration needed — just verify the variant serializes correctly.

**How to avoid:** Add a unit test to `pipelines.rs::tests` that asserts `serde_json::to_string(&ConnectorType::Notion).unwrap() == "\"notion\""`.

**Warning signs:** Serde deserialization fails with "unknown variant `notion`" — this would only happen if the `#[serde(rename_all)]` is not applied consistently.

### Pitfall 5: People values as string vs array

**What goes wrong:** The LLM may output the people field as a single string (`"SK"`) or as an array (`["SK", "CS"]`). The connector must handle both.

**Why it happens:** Without strict schema enforcement in the prompt (Phase 3's job), the LLM may use either form.

**How to avoid:** In `build_notion_properties()`, when handling a `people` property, normalize the value:
- If it's a `Value::String(s)`, treat as `vec![Value::String(s)]`
- If it's a `Value::Array(arr)`, use directly
- If it's `Value::Null`, skip the property

### Pitfall 6: DatabaseProperty::Status select_options is empty

**What goes wrong:** In Phase 1, `convert_database_property()` for `Status` returns empty `select_options`. This means case-insensitive resolution won't work for status properties.

**Why it happens:** The `Status` DatabaseProperty inner struct's field names were not verified against the actual crate in Phase 1 (noted in VERIFICATION.md as an open item).

**How to avoid:** In Phase 2, when `resolve_select_value()` is called for a `status` property and `select_options` is empty, pass the value through unchanged (no case normalization). This is safe — Notion's Status property has defined options just like Select. The fix for extracting Status options belongs in Phase 2's `sync_notion_schema` extension (fix `convert_database_property` for Status) OR accept that status values will be passed through without normalization.

**Warning signs:** Notion API returns 400 for status values that don't match the exact casing.

---

## Code Examples

Verified patterns from official sources and codebase analysis:

### Full execute() skeleton for connectors/notion.rs

```rust
// Source: pattern mirrors /workspace/src-tauri/src/connectors/slack.rs
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::BTreeMap;
use chrono::Utc;
use notion_client::{
    endpoints::{Client, pages::create::request::CreateAPageRequest},
    objects::{
        page::{PageProperty, SelectPropertyValue, DatePropertyValue, DateOrDateTime},
        parent::Parent,
        rich_text::{RichText, Text},
        user::User,
    },
};
use crate::integrations::notion::{load_notion_profile, get_notion_token, NotionIntegrationProfile};

#[derive(Debug)]
struct NotionConnectorConfig {
    integration_id: String,
}

impl NotionConnectorConfig {
    fn from_value(config: &serde_json::Value) -> Result<Self, String> {
        let integration_id = config
            .get("integration_id")
            .and_then(|v| v.as_str())
            .ok_or("Notion connector config missing 'integration_id'")?
            .to_string();
        Ok(NotionConnectorConfig { integration_id })
    }
}

pub async fn execute(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    input_step: &str,
    description: Option<&str>,
) -> Result<PathBuf, String> {
    let connector_config = NotionConnectorConfig::from_value(config)?;
    let profile = load_notion_profile(&connector_config.integration_id)?;
    let token = get_notion_token(&connector_config.integration_id)?;
    let client = Client::new(token, None)
        .map_err(|e| format!("Failed to create Notion client: {:?}", e))?;

    let raw = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input: {}", e))?;

    let items = extract_json_array(&raw)?;

    let mut page_urls: Vec<String> = Vec::new();
    for item in &items {
        let properties = build_notion_properties(item, &profile)?;
        let request = CreateAPageRequest {
            parent: Parent::DatabaseId { database_id: profile.database_id.clone() },
            icon: None,
            cover: None,
            properties,
            children: None,
        };
        let page = client.pages.create_a_page(request).await
            .map_err(|e| format!("Failed to create Notion page: {:?}", e))?;
        // page.url or page.id for output
        page_urls.push(page.id.clone());
    }

    // Write output .md with frontmatter
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;
    let output_path = output_dir.join(format!("{}.md", step_name));
    let now = Utc::now().to_rfc3339();
    let content = format!(
        "---\nname: {}\ndescription: \"{}\"\nconnector: notion\ninput: {}\n\
         status: done\ncreated_at: {}\ncompleted_at: {}\nintegration_id: {}\nerror: null\n---\n\n\
         Created {} Notion page(s): {}",
        step_name,
        description.unwrap_or("Create Notion pages"),
        input_step,
        now, now,
        connector_config.integration_id,
        page_urls.len(),
        page_urls.join(", ")
    );
    fs::write(&output_path, content)
        .map_err(|e| format!("Failed to write output: {}", e))?;

    Ok(output_path)
}
```

### build_notion_properties() — property type dispatch

```rust
// Source: PageProperty variants from github.com/takassh/notion-client/blob/main/src/objects/page.rs
fn build_notion_properties(
    item: &serde_json::Value,
    profile: &NotionIntegrationProfile,
) -> Result<BTreeMap<String, PageProperty>, String> {
    let obj = item.as_object()
        .ok_or("JSON item is not an object")?;

    let mut properties: BTreeMap<String, PageProperty> = BTreeMap::new();

    for prop_def in &profile.properties {
        let value = match obj.get(&prop_def.name) {
            Some(v) => v,
            None => continue,  // LLM didn't provide this property — skip
        };

        if value.is_null() {
            continue;  // null means omit the property
        }

        let page_prop = match prop_def.property_type.as_str() {
            "title" => {
                let text = value.as_str().unwrap_or("").to_string();
                PageProperty::Title {
                    id: None,
                    title: vec![RichText::Text {
                        text: Text { content: text, link: None },
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
                        text: Text { content: text, link: None },
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
                let values = match value {
                    serde_json::Value::Array(arr) => arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| resolve_select_value(s, &prop_def.name, profile))
                        .map(|name| SelectPropertyValue { id: None, name: Some(name), color: None })
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
                PageProperty::MultiSelect { id: None, multi_select: values }
            }
            "people" => {
                let aliases = match value {
                    serde_json::Value::Array(arr) => arr.clone(),
                    serde_json::Value::String(s) => vec![serde_json::Value::String(s.clone())],
                    _ => vec![],
                };
                let users = resolve_people_aliases(&aliases, profile);
                if users.is_empty() { continue; }  // don't send empty people array
                PageProperty::People { id: None, people: users }
            }
            "date" => {
                let date_str = value.as_str().unwrap_or("");
                PageProperty::Date {
                    id: None,
                    date: Some(DatePropertyValue {
                        start: Some(DateOrDateTime::Date(date_str.to_string())),
                        end: None,
                        time_zone: None,
                    }),
                }
            }
            "checkbox" => {
                PageProperty::Checkbox {
                    id: None,
                    checkbox: value.as_bool().unwrap_or(false),
                }
            }
            "url" => PageProperty::Url { id: None, url: value.as_str().map(|s| s.to_string()) },
            "email" => PageProperty::Email { id: None, email: value.as_str().map(|s| s.to_string()) },
            "phone_number" => PageProperty::PhoneNumber {
                id: None,
                phone_number: value.as_str().map(|s| s.to_string()),
            },
            "status" => {
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
            // Skip computed/read-only property types
            "formula" | "rollup" | "relation" | "created_time" | "last_edited_time"
            | "created_by" | "last_edited_by" | "unique_id" | "unknown" => continue,
            _ => continue,
        };

        properties.insert(prop_def.name.clone(), page_prop);
    }

    // Verify at least one property was mapped
    if properties.is_empty() {
        return Err("No properties could be mapped from LLM output to Notion schema. \
                    Verify the integration profile is synced.".to_string());
    }

    Ok(properties)
}
```

### Checking Page struct fields (for output URL)

The `Page` struct returned by `create_a_page()` has an `id` field and a `url` field. Use `page.url` if available for the output, falling back to `page.id`. Verify the exact field names via `cargo doc -- --no-deps` on the notion-client crate.

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| Manual reqwest calls to Notion API | `notion-client::Client::pages.create_a_page()` | Type-safe; no hand-rolled auth or versioning |
| Connector receives arbitrary text | Connector receives structured JSON from LLM step | Phase 3 (prompt augmentation) will guarantee JSON format; Phase 2 must handle imperfect output gracefully |
| `ConnectorType` enum has 5 variants | Add `Notion` as 6th variant | Backward compatible; existing pipeline JSON files unaffected |

**Important:** Phase 2 does NOT include prompt augmentation (that is Phase 3). In Phase 2, the LLM step will produce whatever output it produces. The Notion connector must parse it or fail with a clear error. Phase 3 will inject format instructions into the LLM prompt to make the output reliably structured.

---

## Open Questions

1. **Does `User` in notion-client 1.0.11 implement `Default`?**
   - What we know: `User` has fields `id: String`, `name: Option<String>`, `avator_url: Option<String>`, `user_type: Option<UserType>` (confirmed Phase 1 research). `Option` fields default to `None` but `String` requires `Default = ""`
   - What's unclear: Whether `#[derive(Default)]` is on the User struct
   - Recommendation: Check via `cargo doc`. If not, construct User with all fields explicitly. The `id` is the only required field for the People property.

2. **Exact `Number` type in `PageProperty::Number`**
   - What we know: The field is `number: Option<Number>` and `Number` comes from `notion_client::objects`
   - What's unclear: Whether `Number` is `serde_json::Number`, `f64`, or a newtype wrapper
   - Recommendation: Check `notion_client::objects::Number` via `cargo doc` or source. Most likely it is `serde_json::Number` re-exported. Convert `Value::Number` from LLM output directly.

3. **`Page` struct fields after `create_a_page()` returns**
   - What we know: Returns `Result<Page, NotionClientError>`
   - What's unclear: Whether `Page.url` exists or whether only `Page.id` is available
   - Recommendation: Use `page.id` for output record; if `page.url` exists use it instead. Check via `cargo doc` on `notion_client::objects::page::Page`.

4. **`DateOrDateTime` exact variant names for date strings**
   - What we know: `DatePropertyValue { start: Option<DateOrDateTime>, end, time_zone }` and `DateOrDateTime` is an enum
   - What's unclear: Whether `DateOrDateTime::Date(String)` is the correct variant name, or `DateOrDateTime::DateOnly` or similar
   - Recommendation: Check via `cargo doc`. LLM date output may be `"2026-02-18"` (date only) or `"2026-02-18T10:00:00Z"` (datetime). Handle both: if string contains `T`, use the datetime variant; otherwise use the date variant.

5. **Status property `select_options` is empty in Phase 1 profiles**
   - What we know: `convert_database_property()` for `DatabaseProperty::Status` returns empty `select_options` (Phase 1 VERIFICATION.md, line 109 notes)
   - What's unclear: Whether Phase 2 should fix the Status options extraction in `sync_notion_schema` or just accept passthrough without normalization
   - Recommendation: Fix Status option extraction in Phase 2's Plan 02-01. Check if `DatabaseProperty::Status` has an inner struct with `options` field (similar to Select). If so, extract them. If the inner struct fields are inaccessible, accept passthrough for Status values.

---

## Sources

### Primary (HIGH confidence)

- `github.com/takassh/notion-client/blob/main/src/endpoints/pages/create/request.rs` — `CreateAPageRequest` struct verified: `{ parent: Parent, icon: Option<Icon>, cover: Option<File>, properties: BTreeMap<String, PageProperty>, children: Option<Vec<Block>> }`
- `github.com/takassh/notion-client/blob/main/src/objects/page.rs` — `PageProperty` enum verified: all variants with inner types confirmed (Checkbox, Date, Email, MultiSelect, Number, People, PhoneNumber, RichText, Select, Status, Title, Url, etc.)
- `github.com/takassh/notion-client/blob/main/src/objects/rich_text.rs` — `RichText::Text { text: Text { content: String, link: Option<Link> } }` verified
- `github.com/takassh/notion-client/blob/main/src/objects/parent.rs` — `Parent::DatabaseId { database_id: String }` verified
- `/workspace/src-tauri/src/connectors/slack.rs` — `execute()` signature and output frontmatter pattern (direct codebase analysis)
- `/workspace/src-tauri/src/connectors/llm.rs` — atomic write pattern, error state pattern (direct codebase analysis)
- `/workspace/src-tauri/src/pipelines.rs` — `ConnectorType` enum, `validate_step_config()` pattern (direct codebase analysis)
- `/workspace/src-tauri/src/pipeline_engine.rs` — `execute_pipeline_internal()` match arm pattern (direct codebase analysis)
- `/workspace/src-tauri/src/integrations/notion.rs` — `load_notion_profile()`, `get_notion_token()`, `NotionIntegrationProfile`, `PeopleMapping` types (direct codebase analysis — Phase 1 output)

### Secondary (MEDIUM confidence)

- `docs.rs/notion-client/1.0.11/notion_client/objects/page/index.html` — Module index listing confirmed presence of `SelectPropertyValue`, `DatePropertyValue`, `DateOrDateTime` types (specific field names not confirmed via docs.rs due to low documentation coverage)
- `/workspace/.planning/phases/01-notion-integration-infrastructure/01-RESEARCH.md` — Phase 1 research confirming `User` field names (`id`, `name`, `avator_url`, `user_type`)
- `/workspace/.planning/research/ARCHITECTURE.md` — `extract_json_array()` pattern and `build_notion_format_spec()` pattern (original design source, validated against current codebase)

### Tertiary (LOW confidence)

- `Number` type alias in `notion_client::objects` — mentioned in page.rs enum but concrete type not confirmed; likely `serde_json::Number`
- `DateOrDateTime` variant names — enum confirmed to exist in module index but exact variant names (`Date`, `DateTime`, `DateOnly`) not verified
- `User::Default` trait implementation — not confirmed; if absent, explicit struct construction required

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; notion-client already in Cargo.toml; all extension points identified in existing source
- Architecture: HIGH — connector pattern from slack.rs is clear; PageProperty variants confirmed from GitHub source; CreateAPageRequest structure verified
- Pitfalls: HIGH — Title required, Status options empty, and people array/string normalization all identified from direct analysis; compile-time issues documented with workarounds
- Open questions: MEDIUM — four specific type questions that require `cargo doc` during implementation; all have clear resolution strategies

**Research date:** 2026-02-18
**Valid until:** 2026-03-18 (30 days — notion-client could update; PageProperty API shape verified at 1.0.11)
