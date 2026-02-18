# Phase 3: Prompt Augmentation - Research

**Researched:** 2026-02-18
**Domain:** Pipeline engine look-ahead, prompt injection, schema-to-format-spec generation, JSON validation
**Confidence:** HIGH — all findings based on direct codebase analysis of Phase 1 and Phase 2 output; no external dependencies required; patterns drawn from existing `connectors/llm.rs` and `pipeline_engine.rs`

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| AUGM-01 | Pipeline engine auto-detects when an LLM step is followed by a structured delivery step (Notion) | `execute_pipeline_internal()` iterates `pipeline.steps` as a `Vec<PipelineStep>` — N+1 look-ahead is `pipeline.steps.get(i + 1)` at the iteration index. No new data structures needed. |
| AUGM-02 | Format instructions derived from the destination schema are auto-injected into the LLM prompt | `build_notion_format_spec()` reads `NotionIntegrationProfile.properties` and builds a compact instruction string. The engine calls `build_augmented_prompt()` which appends this spec to the template-rendered prompt before the LLM call. Requires threading the augmented prompt through `connectors::llm::execute()`. |
| AUGM-03 | User never writes format specs manually — schema-to-prompt is automatic | `build_notion_format_spec()` is called only when look-ahead detects a Notion step. The user's prompt template is unmodified; the format spec is appended automatically before the API call. |
| AUGM-04 | AI structured JSON output is validated against the integration profile schema before delivery | `validate_llm_output_for_notion()` is called inside `connectors/notion.rs::execute()` before `build_notion_properties()`. It checks that the parsed JSON array items contain at least one key matching a writable profile property. Returns `Err` with raw output on failure. |
| AUGM-05 | If AI output is not valid JSON, step fails with clear error message and raw output shown | `extract_json_array()` in `connectors/notion.rs` already returns a descriptive error with a preview of raw content. AUGM-05 is already partially met by Phase 2 work — Phase 3 strengthens the error message to explicitly reference the raw LLM output. |
</phase_requirements>

---

## Summary

Phase 3 implements automatic prompt augmentation: when the pipeline engine detects an LLM step immediately followed by a Notion step, it reads the Notion integration profile and appends a compact JSON format specification to the LLM prompt. The user's prompt template is never modified — the format spec is injected at execution time. AI output is then validated against the profile schema before the Notion connector attempts page creation.

The implementation touches three files: `pipeline_engine.rs` (look-ahead detection and augmented prompt construction), `connectors/llm.rs` (accept an optional augmented prompt override), and `connectors/notion.rs` (pre-delivery validation that strengthens AUGM-04 and AUGM-05). No new dependencies are needed — everything uses types already established by Phases 1 and 2.

The most critical design decision: `build_augmented_prompt()` must be a hard `Result<String, String>` — if the integration profile cannot be loaded (missing or corrupt file), the pipeline fails before the expensive LLM API call with a clear "sync schema in Settings" error. This is a pre-roadmap decision that must be honored.

**Primary recommendation:** Implement look-ahead in `execute_pipeline_internal()` at the LLM step dispatch point. Extract augmentation logic into `build_augmented_prompt()` in `pipeline_engine.rs`. Pass the augmented prompt as an optional parameter to `connectors::llm::execute()`. Implement `build_notion_format_spec()` as a standalone function in `pipeline_engine.rs` (not in `connectors/notion.rs`) since it is engine-level concern. Add `validate_llm_output_for_notion()` call in `connectors/notion.rs` between `extract_json_array()` and `build_notion_properties()`.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde_json` | `1.x` (already in Cargo.toml) | Parse and validate LLM JSON output | Already used throughout connectors; `from_str::<Vec<Value>>` is the validation check |
| `crate::integrations::notion` | Phase 1 output | Load `NotionIntegrationProfile` for format spec | Already used by `connectors/notion.rs`; `load_notion_profile()` is the accessor |

### No New Dependencies Required

Phase 3 requires zero new Cargo dependencies. All required types and functions exist in the codebase from Phases 1 and 2.

---

## Architecture Patterns

### Recommended File Changes

```
src-tauri/src/
├── pipeline_engine.rs   — ADD: look-ahead detection in execute_pipeline_internal(),
│                                build_augmented_prompt() function,
│                                build_notion_format_spec() function
│
├── connectors/
│   ├── llm.rs           — EXTEND: accept Option<String> augmented_prompt override;
│   │                              pass it to the API call instead of full_prompt
│   │
│   └── notion.rs        — EXTEND: call validate_llm_output_for_notion() between
│                                   extract_json_array() and build_notion_properties()
```

### Pattern 1: N+1 Look-Ahead in pipeline_engine.rs

**What:** At the LLM step dispatch point in `execute_pipeline_internal()`, check if the next step is `ConnectorType::Notion`. If so, load the Notion profile and build the augmented prompt.

**When to use:** Only for `ConnectorType::Llm` steps with a next step that is `ConnectorType::Notion`.

**Key insight from codebase:** `execute_pipeline_internal()` iterates `pipeline.steps.iter().enumerate()`. The pipeline steps are available as a `Vec<PipelineStep>` cloned from the loaded pipelines. Look-ahead is `pipeline.steps.get(i + 1)`.

```rust
// Source: src-tauri/src/pipeline_engine.rs — existing iteration pattern
for (i, step) in pipeline.steps.iter().enumerate() {
    // ...
    let step_result = match step.connector {
        ConnectorType::Llm => {
            // N+1 look-ahead: check if next step is Notion
            let augmented_prompt = if let Some(next_step) = pipeline.steps.get(i + 1) {
                if next_step.connector == ConnectorType::Notion {
                    // Build augmented prompt — hard fail if profile unavailable
                    let integration_id = next_step.config
                        .get("integration_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Notion step missing integration_id".to_string())?;

                    Some(build_augmented_prompt(
                        &step.config,
                        &input_path,
                        integration_id,
                    )?)
                } else {
                    None
                }
            } else {
                None
            };

            connectors::llm::execute(
                &input_path,
                &step.config,
                &output_dir,
                &step.name,
                &step.input,
                step.description.as_deref(),
                augmented_prompt.as_deref(),  // NEW PARAMETER
            )
            .await
        }
        // ... other arms unchanged
    };
}
```

**CRITICAL NOTE:** The look-ahead must only trigger for the *immediately* following step. Step i+1, not any future step. A pipeline like `[LLM, LLM, Notion]` — the first LLM should NOT be augmented, only the second LLM (at position i=1, next=Notion at i+1=2).

### Pattern 2: build_augmented_prompt() — Hard Fail Design

**What:** Constructs the full augmented prompt string: loads the template, substitutes transcript, then appends the format spec. Returns `Result<String, String>` — returning `Err` causes the pipeline to fail before the LLM call.

**Why it lives in pipeline_engine.rs:** It orchestrates multiple sub-concerns (template loading, transcript reading, profile loading, spec generation) that are all engine-level. Connectors don't orchestrate other connectors.

```rust
// Source: pattern follows connectors/llm.rs prompt construction logic
fn build_augmented_prompt(
    step_config: &serde_json::Value,
    input_path: &Path,
    notion_integration_id: &str,
) -> Result<String, String> {
    // Load the prompt template (same as llm.rs does internally)
    let prompt_template_name = step_config
        .get("prompt_template")
        .and_then(|v| v.as_str())
        .ok_or("LLM step missing prompt_template in config")?;

    let template = crate::prompt_templates::get_prompt_template_internal(prompt_template_name)?;

    // Read input content and strip frontmatter
    let raw_input = std::fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input file: {}", e))?;
    let input_content = crate::connectors::strip_frontmatter(&raw_input);

    // Substitute {transcript} variable
    let base_prompt = crate::prompt_templates::substitute_variables(&template.prompt, input_content);

    // Load Notion integration profile — hard fail with clear error if missing
    let profile = crate::integrations::notion::load_notion_profile(notion_integration_id)
        .map_err(|_| format!(
            "Cannot augment prompt: Notion integration '{}' profile not found. \
             Sync schema in Settings > Integrations before running this pipeline.",
            notion_integration_id
        ))?;

    // Build format spec from profile
    let format_spec = build_notion_format_spec(&profile);

    // Append format spec to the base prompt
    Ok(format!("{}\n\n{}", base_prompt, format_spec))
}
```

**IMPORTANT:** `build_augmented_prompt()` must NOT be called by `connectors::llm::execute()`. The engine calls it, then passes the result as an override to the connector. This preserves connector independence — connectors don't know about other connectors.

### Pattern 3: build_notion_format_spec() — Token Budget Design

**What:** Builds a compact format instruction string from the integration profile's properties. Must stay within a reasonable token budget for databases with many properties.

**Key design decisions from requirements:**
1. Only include *writable* properties (exclude: formula, rollup, relation, created_time, last_edited_time, created_by, last_edited_by, unique_id, unknown)
2. Include property type and available options for select/multi_select/status
3. Use concise format — one line per property is sufficient
4. Token budget: aim for under 500 tokens for the format spec itself (~200 words)

**Field relevance filtering:** For properties with many select options, truncate to the first 10-15 options with a "(+ N more)" note. This prevents a database with 50 select options from bloating the prompt.

```rust
// Source: pattern designed from NotionIntegrationProfile structure
// (src-tauri/src/integrations/notion.rs — NotionPropertyDef, NotionIntegrationProfile)

/// Writable property types that the LLM should populate.
/// Read-only/computed types are excluded from the format spec.
const WRITABLE_TYPES: &[&str] = &[
    "title", "rich_text", "select", "multi_select", "people",
    "date", "number", "checkbox", "url", "email", "phone_number", "status",
];

/// Maximum number of select options to include in the format spec.
/// Prevents token budget overflow for databases with many options.
const MAX_OPTIONS_IN_SPEC: usize = 12;

fn build_notion_format_spec(profile: &NotionIntegrationProfile) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push("Output a JSON array where each element is an object with these fields:".to_string());
    lines.push(format!("Database: {}", profile.database_name));
    lines.push(String::new());

    for prop in &profile.properties {
        // Skip read-only/computed properties
        if !WRITABLE_TYPES.contains(&prop.property_type.as_str()) {
            continue;
        }

        let field_spec = match prop.property_type.as_str() {
            "title" => format!("- \"{}\" (string, REQUIRED): page title", prop.name),
            "rich_text" => format!("- \"{}\" (string): long text", prop.name),
            "number" => format!("- \"{}\" (number): numeric value", prop.name),
            "checkbox" => format!("- \"{}\" (boolean): true or false", prop.name),
            "url" => format!("- \"{}\" (string): URL", prop.name),
            "email" => format!("- \"{}\" (string): email address", prop.name),
            "phone_number" => format!("- \"{}\" (string): phone number", prop.name),
            "date" => format!("- \"{}\" (string): ISO 8601 date, e.g. \"2026-02-18\"", prop.name),
            "people" => format!("- \"{}\" (array of strings): person aliases", prop.name),
            "select" | "status" => {
                if prop.select_options.is_empty() {
                    format!("- \"{}\" (string): one of the defined options", prop.name)
                } else {
                    let (shown, overflow) = if prop.select_options.len() > MAX_OPTIONS_IN_SPEC {
                        (&prop.select_options[..MAX_OPTIONS_IN_SPEC],
                         prop.select_options.len() - MAX_OPTIONS_IN_SPEC)
                    } else {
                        (&prop.select_options[..], 0)
                    };
                    let options_str: Vec<&str> = shown.iter().map(|s| s.as_str()).collect();
                    if overflow > 0 {
                        format!(
                            "- \"{}\" (string): one of: {} (+ {} more)",
                            prop.name,
                            options_str.join(", "),
                            overflow
                        )
                    } else {
                        format!(
                            "- \"{}\" (string): one of: {}",
                            prop.name,
                            options_str.join(", ")
                        )
                    }
                }
            }
            "multi_select" => {
                if prop.select_options.is_empty() {
                    format!("- \"{}\" (array of strings): multiple values from defined options", prop.name)
                } else {
                    let (shown, overflow) = if prop.select_options.len() > MAX_OPTIONS_IN_SPEC {
                        (&prop.select_options[..MAX_OPTIONS_IN_SPEC],
                         prop.select_options.len() - MAX_OPTIONS_IN_SPEC)
                    } else {
                        (&prop.select_options[..], 0)
                    };
                    let options_str: Vec<&str> = shown.iter().map(|s| s.as_str()).collect();
                    if overflow > 0 {
                        format!(
                            "- \"{}\" (array of strings): values from: {} (+ {} more)",
                            prop.name,
                            options_str.join(", "),
                            overflow
                        )
                    } else {
                        format!(
                            "- \"{}\" (array of strings): values from: {}",
                            prop.name,
                            options_str.join(", ")
                        )
                    }
                }
            }
            _ => continue,  // Unknown writable types — skip
        };
        lines.push(field_spec);
    }

    lines.push(String::new());
    lines.push("Omit any field you cannot determine from the transcript. Use null for unknown optional fields.".to_string());
    lines.push("People fields: use the person's name or alias as a string (e.g. \"Alice\", \"SK\").".to_string());
    lines.push("Return ONLY the JSON array. No prose, no markdown, no code fences.".to_string());

    lines.join("\n")
}
```

### Pattern 4: connectors/llm.rs — Accept Augmented Prompt

**What:** The `execute()` function signature gains one new parameter: `augmented_prompt: Option<&str>`. When `Some`, it is used instead of the internally-constructed `full_prompt`. When `None`, behavior is unchanged (backward compatible).

**Why Option<&str> not bool + String:** The engine only builds the augmented prompt when needed; passing `None` avoids any overhead for non-Notion pipelines.

```rust
// Source: src-tauri/src/connectors/llm.rs — extend existing execute() signature

pub async fn execute(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    step_input: &str,
    step_description: Option<&str>,
    augmented_prompt: Option<&str>,  // NEW: None = normal behavior, Some = use this instead
) -> Result<PathBuf, String> {
    let llm_config = LlmConfig::from_value(config)?;
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    // When augmented_prompt is provided, skip internal template loading entirely.
    // The engine has already built the full prompt including format spec.
    let prompt_to_send = if let Some(augmented) = augmented_prompt {
        // Token check still needed — augmented prompt may be large
        let approx_tokens = estimate_tokens(augmented);
        let context_limit = context_limit_for_provider(&llm_config.provider);
        if approx_tokens > context_limit {
            let max_chars = context_limit * 4;
            augmented[..augmented.len().min(max_chars)].to_string()
        } else {
            augmented.to_string()
        }
    } else {
        // Existing path: read input, load template, substitute variables
        let raw_input = fs::read_to_string(input_path)
            .map_err(|e| format!("Failed to read input file: {}", e))?;
        let input_content = super::strip_frontmatter(&raw_input);
        let template = get_prompt_template_internal(&llm_config.prompt_template)?;
        let full_prompt = substitute_variables(&template.prompt, input_content);
        let approx_tokens = estimate_tokens(&full_prompt);
        let context_limit = context_limit_for_provider(&llm_config.provider);
        if approx_tokens > context_limit {
            let max_chars = context_limit * 4;
            full_prompt[..full_prompt.len().min(max_chars)].to_string()
        } else {
            full_prompt
        }
    };

    // ... rest of execute() unchanged (API call, output write)
}
```

**CRITICAL:** All existing callers of `connectors::llm::execute()` must be updated to pass `None` as the last argument. The only caller is `pipeline_engine.rs`. Verify with grep after implementation.

### Pattern 5: validate_llm_output_for_notion() in connectors/notion.rs

**What:** Called between `extract_json_array()` and `build_notion_properties()`. Validates that each JSON array item has at least one key matching a writable profile property. Returns a descriptive error with the raw output on failure.

**Why here and not in the engine:** Validation is the Notion connector's responsibility — it knows the schema. The engine only knows that LLM output was produced.

```rust
// Source: designed from NotionIntegrationProfile structure and AUGM-04 requirement

fn validate_llm_output_for_notion(
    items: &[serde_json::Value],
    profile: &NotionIntegrationProfile,
    raw_output: &str,
) -> Result<(), String> {
    if items.is_empty() {
        return Err(format!(
            "Notion connector: LLM output parsed as empty JSON array — no pages to create.\n\
             Raw output: {}",
            &raw_output[..raw_output.len().min(500)]
        ));
    }

    // Get writable property names from profile
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
                 Raw output: {}",
                idx,
                &raw_output[..raw_output.len().min(500)]
            )),
        };

        // Check that at least one key in the object matches a writable property
        let has_valid_key = obj.keys().any(|k| writable_names.contains(k.as_str()));
        if !has_valid_key {
            return Err(format!(
                "Notion connector: JSON array element {} has no keys matching profile schema.\n\
                 Profile writable properties: {}\n\
                 Got keys: {}\n\
                 Raw output: {}",
                idx,
                profile.properties.iter()
                    .filter(|p| writable_names.contains(p.name.as_str()))
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                obj.keys().cloned().collect::<Vec<_>>().join(", "),
                &raw_output[..raw_output.len().min(500)]
            ));
        }
    }

    Ok(())
}
```

**Note:** `WRITABLE_TYPES` should be defined once in `connectors/notion.rs` so both `build_notion_properties()` and `validate_llm_output_for_notion()` share it.

### Anti-Patterns to Avoid

- **Augmenting prompts inside `connectors/llm.rs`:** The LLM connector must not import from `integrations::notion` or `connectors::notion`. Connectors don't have cross-knowledge. Augmentation is the engine's job.
- **Modifying user's prompt template:** The `PromptTemplate.prompt` stored on disk is never modified. The format spec is only appended at runtime in memory.
- **Silent fallthrough when profile is missing:** If `load_notion_profile()` returns Err, the pipeline MUST fail before the LLM API call with a clear error message. No fallback to non-augmented mode (pre-roadmap decision).
- **Rebuilding the prompt inside both the engine and the connector:** The engine builds the augmented prompt once. The connector receives it via the new parameter. The connector does NOT additionally call template loading.
- **Validating against JSON Schema:** No `jsonschema` crate needed. Validation is "has at least one key matching a writable profile property" — this is sufficient for AUGM-04 at v1 scope.
- **Augmenting every LLM step in multi-step pipelines:** Only augment the LLM step that is *directly* followed by a Notion step. Pipeline `[LLM, LLM, Notion]` — only step at index 1 (LLM) gets augmented, not step at index 0.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Token estimation for augmented prompt | New token counter | Existing `estimate_tokens()` in `llm.rs` | Already tested and calibrated for all providers |
| Profile loading with error mapping | Custom file I/O | `load_notion_profile()` from `integrations::notion` | Already returns the correct "Sync schema in Settings" error message |
| Prompt template loading | Read template file directly | `get_prompt_template_internal()` from `prompt_templates` | Handles migration, built-in templates, and error formatting |
| Format spec token counting | Separate token budget module | `estimate_tokens()` with the existing heuristic | Format spec is small; the existing heuristic is more than adequate |

---

## Common Pitfalls

### Pitfall 1: Augmented prompt built but `input_path` does not exist yet

**What goes wrong:** `build_augmented_prompt()` reads `input_path` to get the transcript content. At the time the LLM step is about to run, `input_path` exists (the engine already verified it at line 171 of `pipeline_engine.rs`). However, `build_augmented_prompt()` is called *after* the input_path existence check, so this is safe.

**Warning signs:** `Failed to read input file` error from `build_augmented_prompt()` — this should not happen because the engine verifies input_path exists before the match block.

**How to avoid:** Call `build_augmented_prompt()` inside the `ConnectorType::Llm` arm of the match block, AFTER the engine has verified `input_path.exists()`.

### Pitfall 2: All callers of connectors::llm::execute() must pass None

**What goes wrong:** After adding `augmented_prompt: Option<&str>` to `connectors::llm::execute()`, Rust's type checker will catch any callers that don't pass the new argument. But there is only ONE caller: `pipeline_engine.rs`. The existing `Llm` arm in the match block must be updated to pass `augmented_prompt.as_deref()`.

**Why it happens:** Rust requires all function arguments — no default values.

**How to avoid:** After modifying `connectors/llm.rs`, run `cargo check` immediately. The compiler will pinpoint any missing argument. Update the match arm to pass `augmented_prompt.as_deref()` (or `None` for any test callers).

**Warning signs:** `error[E0061]: this function takes N arguments but M arguments were supplied` on `connectors::llm::execute()`.

### Pitfall 3: People field format spec — alias vs. Notion user ID

**What goes wrong:** The format spec instructs the LLM to output person aliases (e.g. `"Alice"`, `"SK"`), but without context from `people_mappings`, the LLM doesn't know what aliases are valid. The spec only lists that the field is "array of strings: person aliases" — the LLM may invent names.

**Why it happens:** `people_mappings` in the profile maps aliases to Notion user IDs, but the aliases themselves should be hinted to the LLM. For example, if `people_mappings` has `alias: "SK"`, the spec should hint `["SK", ...]` as valid aliases.

**How to avoid:** In `build_notion_format_spec()`, for `people` properties, include the known aliases from `profile.people_mappings`:

```rust
"people" => {
    let aliases: Vec<&str> = profile.people_mappings.iter()
        .map(|m| m.alias.as_str())
        .collect();
    if aliases.is_empty() {
        format!("- \"{}\" (array of strings): person aliases", prop.name)
    } else {
        format!(
            "- \"{}\" (array of strings): known aliases are: {}",
            prop.name,
            aliases.join(", ")
        )
    }
}
```

**Warning signs:** LLM outputs person names that don't match any alias in the profile — all people properties are silently skipped, resulting in pages with no assignees.

### Pitfall 4: Empty profile (not yet synced) causes wrong error

**What goes wrong:** If a user configures a Notion step with `integration_id` but never syncs the schema, the profile file exists (it's created during `add_notion_integration`) but has `properties: []` and `database_id: ""`. The `build_augmented_prompt()` function will succeed (profile loads), but the format spec will be empty/useless.

**Why it happens:** `add_notion_integration()` creates an empty profile. `load_notion_profile()` succeeds on empty profiles.

**How to avoid:** In `build_augmented_prompt()`, after loading the profile, check that `profile.properties` is non-empty:

```rust
if profile.properties.is_empty() {
    return Err(format!(
        "Cannot augment prompt: Notion integration '{}' has no schema synced. \
         Open Settings > Integrations > {} > Sync Schema before running this pipeline.",
        notion_integration_id,
        profile.name
    ));
}
```

**Warning signs:** LLM output is plain text (not JSON) because the format spec was empty or degenerate, leading to a parse error in the Notion connector.

### Pitfall 5: Double-reading input_path

**What goes wrong:** `build_augmented_prompt()` reads `input_path` to get the transcript text. Then `connectors::llm::execute()` with `augmented_prompt: Some(...)` skips reading `input_path` again (the whole point). But if `execute()` still reads `input_path` in the `Some` branch, there's redundant I/O.

**How to avoid:** In the modified `llm::execute()`, when `augmented_prompt` is `Some`, skip ALL the internal template/input processing and go directly to the API call. This is also the correct design because the engine has already built the complete prompt.

### Pitfall 6: Look-ahead fails for non-contiguous LLM→Notion chains

**What goes wrong:** A pipeline `[LLM, Save, Notion]` where LLM is step 0, Save is step 1, Notion is step 2. Look-ahead at `i+1` would find `Save`, not `Notion`. The LLM would not be augmented, and the Notion step would receive unstructured output.

**Why it matters:** This is a deliberate scope limitation for Phase 3. The requirement says "an LLM step followed by a structured delivery step" — this means *directly* followed (N+1).

**How to document:** Add a comment in the look-ahead code: "Only augments when Notion is the immediately next step (N+1). Non-contiguous LLM→Notion chains are not augmented in v1."

**This is by design, not a bug.** The user should structure their pipeline as `[LLM, Notion]` or `[LLM, Notion, Save]` for augmentation to work.

---

## Code Examples

Verified patterns from direct codebase analysis:

### How the full execute_pipeline_internal() loop will look (Llm arm only)

```rust
// Source: src-tauri/src/pipeline_engine.rs — modified Llm arm
ConnectorType::Llm => {
    // N+1 look-ahead: augment prompt if next step is Notion
    let augmented = if let Some(next_step) = pipeline.steps.get(i + 1)
        && next_step.connector == ConnectorType::Notion
    {
        let integration_id = next_step.config
            .get("integration_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!(
                "Step '{}': Notion step missing integration_id (required for prompt augmentation)",
                next_step.name
            ))?;

        Some(build_augmented_prompt(&step.config, &input_path, integration_id)?)
    } else {
        None
    };

    connectors::llm::execute(
        &input_path,
        &step.config,
        &output_dir,
        &step.name,
        &step.input,
        step.description.as_deref(),
        augmented.as_deref(),
    )
    .await
}
```

### NotionIntegrationProfile structure (for reference)

```rust
// Source: src-tauri/src/integrations/notion.rs — Phase 1 output
pub struct NotionIntegrationProfile {
    pub id: String,
    pub name: String,
    pub database_id: String,
    pub database_name: String,
    pub properties: Vec<NotionPropertyDef>,   // key input for format spec
    pub people_mappings: Vec<PeopleMapping>,  // key input for people hint
    pub workspace_users: Vec<WorkspaceUser>,
    pub synced_at: String,
}

pub struct NotionPropertyDef {
    pub name: String,
    pub property_type: String,  // "title", "select", "people", etc.
    pub select_options: Vec<String>,  // non-empty for select/multi_select/status
}

pub struct PeopleMapping {
    pub alias: String,           // what LLM should output, e.g. "SK"
    pub notion_user_id: String,  // Notion UUID
    pub display_name: String,    // e.g. "Sergey K"
}
```

### execute() in connectors/notion.rs — updated flow

```rust
// Source: src-tauri/src/connectors/notion.rs — Phase 2 output, Phase 3 modifications
pub async fn execute(
    input_path: &Path,
    config: &serde_json::Value,
    output_dir: &Path,
    step_name: &str,
    input_step: &str,
    description: Option<&str>,
) -> Result<PathBuf, String> {
    // ... config + profile + client setup unchanged ...

    let raw = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input file '{}': {}", input_path.display(), e))?;

    // Step 1: Extract JSON array (handles code fences)
    let items = extract_json_array(&raw)?;

    // Step 2: NEW — validate structure against profile (AUGM-04, AUGM-05)
    validate_llm_output_for_notion(&items, &profile, &raw)?;

    // Step 3: Create pages
    for item in &items {
        let properties = build_notion_properties(item, &profile)?;
        // ... page creation unchanged ...
    }

    // ... output write unchanged ...
}
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| User manually writes format instructions in prompt template | Engine auto-injects format spec from Notion schema | AUGM-03: zero manual work required |
| LLM connector knows nothing about downstream connector | Engine performs N+1 look-ahead; LLM connector receives optional augmented prompt | Clean separation — connector independence preserved |
| Notion connector parse errors revealed only after LLM call | Profile availability checked before LLM call; clear error with "sync schema" guidance | AUGM-02: fail fast, save API cost |
| No validation between LLM output and Notion schema | `validate_llm_output_for_notion()` checks keys before page creation | AUGM-04: schema mismatch caught with raw output shown |

---

## Open Questions

1. **What happens when the LLM step and Notion step are in the same pipeline but Notion has no schema synced?**
   - What we know: `add_notion_integration()` creates an empty profile (empty `properties`, empty `database_id`)
   - What's unclear: Whether Pitfall 4 check (`profile.properties.is_empty()`) is the right gate, or whether we should also check `profile.database_id.is_empty()`
   - Recommendation: Check both — if either is empty, return the "sync schema" error. A profile with a `database_id` but no `properties` is also unusable.

2. **Should the format spec identify the REQUIRED title field explicitly?**
   - What we know: Every Notion database has exactly one `title` type property; page creation fails without it
   - What's unclear: Whether the format spec should mark it `(REQUIRED)` to guide the LLM
   - Recommendation: Yes, mark it `(string, REQUIRED)` as shown in the code example above. The title is structurally required; making it explicit improves LLM compliance.

3. **Should people aliases be listed per-property, or at the end of the spec?**
   - What we know: `people_mappings` applies globally — any people property can use any alias
   - What's unclear: Whether per-property listing (more contextual) or a global aliases section (cleaner) is better for LLM understanding
   - Recommendation: Include per-property in the field line (as shown in Pitfall 3 fix). More contextual, fewer tokens than a separate section.

4. **What if a pipeline has multiple Notion steps?**
   - What we know: The current roadmap has no multi-Notion-step pipelines; augmentation looks at N+1 only
   - What's unclear: Whether a pipeline `[LLM, Notion, LLM, Notion]` should augment both LLM steps
   - Recommendation: For v1, the N+1 look-ahead handles this correctly: step 0 (LLM) looks ahead to step 1 (Notion) → augments; step 2 (LLM) looks ahead to step 3 (Notion) → augments. This works without any special handling.

5. **Token budget hard limit?**
   - What we know: The format spec for a typical Notion database (5-15 properties) should be ~100-300 tokens
   - What's unclear: Whether a Notion database with 50+ properties (uncommon but possible) could blow the context window
   - Recommendation: The `MAX_OPTIONS_IN_SPEC = 12` constant per select field handles the main bloat source. For property count itself, no truncation needed — 50 properties × ~20 chars per line ≈ 1000 tokens, well within all provider context windows.

---

## Sources

### Primary (HIGH confidence)

- `/workspace/src-tauri/src/pipeline_engine.rs` — `execute_pipeline_internal()` loop, step iteration pattern, input_path existence check; verified lines 143-186 for look-ahead insertion point
- `/workspace/src-tauri/src/connectors/llm.rs` — `execute()` signature, `estimate_tokens()`, `context_limit_for_provider()`, token budget logic; verified lines 123-274
- `/workspace/src-tauri/src/connectors/notion.rs` — Phase 2 output; `execute()`, `extract_json_array()`, `build_notion_properties()`; verified all 487 lines
- `/workspace/src-tauri/src/integrations/notion.rs` — `NotionIntegrationProfile`, `NotionPropertyDef`, `PeopleMapping` types; `load_notion_profile()` error message; verified lines 20-58, 97-110
- `/workspace/src-tauri/src/pipelines.rs` — `Pipeline.steps: Vec<PipelineStep>`, `PipelineStep.connector: ConnectorType`; confirmed look-ahead via `pipeline.steps.get(i + 1)` is valid
- `/workspace/src-tauri/src/prompt_templates.rs` — `get_prompt_template_internal()`, `substitute_variables()` patterns; verified lines 210-212
- `/workspace/src-tauri/src/connectors/mod.rs` — `strip_frontmatter()` utility used by both llm.rs and notion.rs

### Secondary (MEDIUM confidence)

- `/workspace/.planning/phases/02-notion-connector/02-RESEARCH.md` — property type mapping table, writable vs. computed property classification
- `/workspace/.planning/phases/01-notion-integration-infrastructure/01-RESEARCH.md` — integration profile design, `get_integrations_dir()` location decision

### Tertiary (LOW confidence)

- Pre-roadmap decisions captured in phase context — "hard `Result<>` return — no silent fallthrough to non-JSON LLM output" — confirms the fail-before-LLM-call design; not independently verified via official source but consistent with codebase architecture

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new dependencies; all types from Phases 1+2 codebase; verified directly
- Architecture: HIGH — LLM connector modification is minimal (one new Option parameter); look-ahead pattern is trivial with existing Vec; all function signatures confirmed
- Pitfalls: HIGH — derived from direct codebase analysis of the exact code that will be modified; compiler will catch most issues

**Research date:** 2026-02-18
**Valid until:** 2026-03-18 (30 days — purely internal code; no external API changes)
