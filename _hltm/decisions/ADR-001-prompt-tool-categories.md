# ADR-001: Keep Prompt and Tool Categories Separate

**Status**: Accepted  
**Date**: 2026-02-25  
**Issue**: nbp-2rk  
**Decision Maker**: Architecture review

## Context

The pipeline step model uses three categories for UI grouping:
- **Prompt**: LLM-based text transformation
- **Tool**: External action execution
- **Output**: Delivery to external services

CLI Agent connector is categorized as `Tool` but accepts a `prompt` config field, creating apparent category overlap with the LLM connector (which is `Prompt`).

### Current Category Mapping

| Connector | Category | Has Prompt? | Primary Action |
|-----------|----------|-------------|----------------|
| LLM | Prompt | Yes (template/inline) | API call to LLM |
| CLI Agent | Tool | Yes (instruction) | Spawn subprocess |
| MCP | Tool | No (args only) | HTTP call to MCP server |
| Save/Slack/Notion/Linear/Webhook | Output | No | Deliver result |

## Decision

**Option 1: Keep separate categories and treat CLI Agent prompt as tool instruction payload.**

## Rationale

### 1. User Mental Model Clarity

The category distinction maps to user intent:
- **Prompt** = "I want the AI to transform/enrich this text"
- **Tool** = "I want to execute something that produces output"
- **Output** = "I want to send the result somewhere"

CLI Agent fits "Tool" because the user's primary intent is "run an external agent," not "call an LLM API directly."

### 2. Prompt Semantics Differ

| Aspect | LLM Connector | CLI Agent |
|--------|---------------|-----------|
| Prompt purpose | Template for text transformation | Instruction for external tool |
| Execution | App calls API directly | App spawns subprocess |
| Control | App controls request/response | External tool controls execution |
| Output source | API response | Subprocess stdout |

The CLI Agent's `prompt` field is semantically an **instruction payload** for the external tool, analogous to MCP's `args` field. Both are configuration for external execution.

### 3. Execution Semantics

- **LLM (Prompt)**: App constructs request, calls API, parses response. Tokens consumed by app's API call.
- **CLI Agent (Tool)**: App spawns subprocess with args, subprocess calls its own LLM. Tokens consumed by external tool.
- **MCP (Tool)**: App makes HTTP request, external server executes. Side effects by external server.

CLI Agent and MCP share the same execution pattern: app delegates to external system. The fact that CLI Agent's external system happens to use an LLM internally is an implementation detail.

### 4. Migration Cost

Zero changes required. Current implementation already follows this model.

### 5. Extensibility

Future tools that accept prompt-like inputs follow the same pattern:
- "Run Custom Script" tool → `script` + `args` → Tool category
- "Shell Command" tool → `command` → Tool category

## Rules for Prompt-Capable Tools

1. **Category**: Tools that execute external systems remain in `Tool` category regardless of whether they accept text instructions.

2. **Field Naming**: Use `prompt` for user-facing labels when the tool expects natural language instructions. Under the hood, it's an instruction payload.

3. **Documentation**: Tool config schemas should clarify that prompts are instructions for the external tool, not direct LLM prompts.

4. **UI Treatment**: Tool prompts should be clearly distinguished from LLM prompts in the step editor (different section, different placeholder text).

## Alternatives Considered

### Option 2: Merge Prompt + Tool into "Action/Step" with capability flags

**Rejected because**:
- Major refactoring required (schema, UI, validation)
- Loses meaningful user-facing categorization
- Introduces complexity for no user benefit
- Existing three-category model is intuitive

### Option 3: Keep separate in UX but unify underlying schema

**Rejected because**:
- Adds hidden complexity without solving perceived inconsistency
- The "inconsistency" is actually correct semantic distinction
- No user-facing improvement

## Consequences

### Positive
- No migration or refactoring needed
- Clear, consistent mental model
- Extensible for future tools

### Negative
- Some users may initially look for CLI Agent in "Prompt" section
- Mitigation: CLI Agent appears in Tool section with clear icon and description

## Follow-up Tasks

1. **UI Enhancement**: Add tooltip or hint in CLI Agent step editor explaining it executes external tools (not direct API calls)
2. **Documentation**: Update user docs to explain category semantics

## References

- `src-tauri/src/pipelines.rs:33-40` - Category mapping
- `src-tauri/src/connectors/cli_agent.rs` - CLI Agent implementation
- `src/pipeline-builder.js:248-271` - Tool presets in UI
