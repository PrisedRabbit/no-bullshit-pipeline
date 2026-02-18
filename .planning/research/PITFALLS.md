# Pitfalls Research

**Domain:** Pipeline builder, Notion API integration, schema-aware AI prompts, vanilla JS complex UIs — Tauri desktop app (Pipelines v2)
**Researched:** 2026-02-18
**Confidence:** HIGH (Notion API pitfalls verified against official docs + community post-mortems; Tauri pitfalls verified against official GitHub issues; AI structured output pitfalls verified against OWASP 2025 and multiple sources)

---

## Critical Pitfalls

### Pitfall 1: Notion API Version Mismatch (2025-09-03 Breaking Change)

**What goes wrong:**
The new `2025-09-03` Notion API version is not backwards-compatible. It replaces `database_id` with `data_source_id` for multi-source databases. If a user converts a standard Notion database into a multi-source database after the integration is set up, all subsequent `POST /v1/pages`, `GET /v1/databases`, and relation-writing calls silently fail with 400 or 500 errors.

**Why it happens:**
The integration profiles store `database_id` at setup time. The Notion API contract changes under the integration without warning the integration. Developers assume "it worked during setup = it will always work."

**How to avoid:**
Pin the `Notion-Version` header to `2022-06-28` for v1 to avoid the new model entirely. Single-user desktop apps do not need multi-source databases. Document this explicitly in the integration setup code. When writing schema-sync logic, record the API version used in the integration profile JSON so a future migration is possible.

```rust
// In integrations/notion.rs, all requests must include:
.header("Notion-Version", "2022-06-28")
// NOT "2025-09-03" — multi-source DB model is enterprise-only
```

**Warning signs:**
- 400 errors on `POST /v1/pages` that were working before
- Error message: "Databases with multiple data sources are not supported"
- User recently changed their Notion database type in the UI

**Phase to address:** Notion Integration Infrastructure (Phase 1). Pin the API version in a single constant and document the constraint.

---

### Pitfall 2: Integration Not Shared with the Notion Database

**What goes wrong:**
A Notion internal integration is created and the API key is valid, but every API call against a specific database returns 404 or 403. The integration has not been manually shared with the database through the Notion UI. The API key alone is not sufficient.

**Why it happens:**
Notion's security model requires users to explicitly connect the integration to each page/database via the Notion UI (the "..." menu → "Connect to" → integration name). This step is not part of the API flow and is invisible to the code — the token validates fine, but the database calls fail.

**How to avoid:**
The setup wizard must explicitly tell the user to share the integration with their database before attempting to list databases. This is step 1.5 in the wizard, not an afterthought. Show a specific instruction: "In Notion, open your database → ••• menu → Connections → Add [your integration name]". The error handling for 404 from `GET /v1/databases/{id}` should display this instruction, not a generic error.

**Warning signs:**
- `GET /v1/users/me` returns 200 (token is valid)
- `POST /v1/search` returns empty results even though databases exist
- Any database-specific call returns 404

**Phase to address:** Notion Integration Setup Wizard (Phase 4). The wizard UI must include this as a mandatory step with screenshot-level instructions, not just a hint.

---

### Pitfall 3: LLM Returns Non-JSON When Schema Augmentation Fails to Inject

**What goes wrong:**
The pipeline engine calls `build_augmented_prompt()` to inject the JSON format spec before the LLM step. If the look-ahead fails to find the integration profile (e.g., profile file missing, integration deleted), the augmentation silently falls through and the LLM produces free-form prose. The Notion connector then fails to parse the output and the step fails with an unhelpful "could not parse JSON" error.

**Why it happens:**
Prompt augmentation is a side effect that happens inside the pipeline engine, invisible to the user. When it silently fails (file not found, wrong integration_id), the AI receives only the base prompt with no format instructions and produces whatever format it feels like.

**How to avoid:**
The augmentation function must not silently fall through. If the next step is a structured connector and the integration profile cannot be loaded, the engine should return a hard error before calling the LLM: "Cannot execute: Notion integration 'X' profile not found. Re-sync in Settings." This prevents wasting an expensive AI API call that will produce unusable output.

```rust
// In pipeline_engine.rs
fn build_augmented_prompt(...) -> Result<String, String> { // Returns Result, not String
    if next_step.connector == ConnectorType::Notion {
        let profile = load_notion_profile(integration_id)
            .map_err(|_| format!(
                "Notion integration '{}' not configured. Sync schema in Settings > Integrations.",
                integration_id
            ))?; // Hard fail, not silent fallthrough
        ...
    }
}
```

**Warning signs:**
- Notion step output file contains markdown prose, not a JSON array
- "Could not parse structured output as JSON array" errors
- Integration profile file was deleted or renamed

**Phase to address:** Prompt Augmentation (Phase 3). Add explicit validation that the profile exists before LLM execution begins.

---

### Pitfall 4: Notion People Property Cannot Be Queried by Name or Email

**What goes wrong:**
The people-mapping feature maps aliases ("SK", "Sergey") to Notion user IDs. The setup wizard tries to auto-populate the mapping by searching users by name or email to suggest matches. The Notion API does not support filtering users by name or email — only the full user list can be fetched, and name/email matching must happen client-side. If this constraint is misunderstood, the wizard breaks or returns "no users found" incorrectly.

**Why it happens:**
The `GET /v1/users` endpoint returns the full paginated user list without filter support. Developers assume all list APIs support search/filter parameters.

**How to avoid:**
Fetch the full user list at setup time, cache it in the integration profile, and do fuzzy matching client-side. Display the full list in the wizard and let the user manually map aliases to users. Never attempt to call Notion with a name/email filter — it will return a 400 validation error.

**Warning signs:**
- `GET /v1/users` with query parameters returning 400 errors
- "User not found" when the user clearly exists in the workspace
- Empty people dropdown in setup wizard

**Phase to address:** Notion Integration Setup Wizard (Phase 4). Fetch all users once and map locally.

---

### Pitfall 5: Vanilla JS State/DOM Desync in the Pipeline Builder

**What goes wrong:**
The pipeline builder maintains an in-memory `pipeline` state object (array of steps). As the user adds, removes, and reorders steps, the DOM (step cards rendered in the builder) goes out of sync with the state object. This manifests as: clicking "Delete step" removes the wrong step, drag-and-drop moves a step visually but the underlying array has the wrong order, or saving the pipeline writes stale state.

**Why it happens:**
Without a framework's virtual DOM diffing, developers use a mix of direct DOM manipulation and state mutations. When DOM and state diverge (e.g., drag event updates DOM but forgets to update the array), every subsequent operation acts on inconsistent data. The bug is invisible until save.

**How to avoid:**
Use a strict "single source of truth + full re-render" pattern for the builder. The state object is the authoritative source. Every user action (add, remove, drag-drop) updates state first, then calls a single `renderSteps(state)` function that clears and rebuilds the DOM from scratch. Never manipulate the DOM directly based on a user action — always go through state.

```javascript
// Pipeline builder state
let pipelineState = { name: '', steps: [] };

function addStep(preset) {
    pipelineState.steps.push(buildStep(preset));
    renderSteps(); // full re-render from state
}

function removeStep(index) {
    pipelineState.steps.splice(index, 1);
    renderSteps(); // full re-render
}

function renderSteps() {
    const container = document.getElementById('steps-list');
    container.innerHTML = ''; // clear all
    pipelineState.steps.forEach((step, i) => {
        container.appendChild(buildStepCard(step, i));
    });
}
```

**Warning signs:**
- "Save" writes a different pipeline than what's displayed
- Deleting step 2 actually deletes step 3
- Reordering steps looks correct visually but the saved order is wrong
- State has 3 steps, DOM shows 2 (or vice versa)

**Phase to address:** Pipeline Builder Redesign (Phase 5). Establish the state → render pattern before writing any drag-and-drop logic.

---

### Pitfall 6: `dragover` Missing `preventDefault()` Silently Disables All Drops

**What goes wrong:**
The step reorder drag-and-drop in the pipeline builder never fires the `drop` event. All drag operations complete visually but nothing happens on release.

**Why it happens:**
The HTML5 Drag and Drop API requires `event.preventDefault()` in the `dragover` handler to signal that the drop target accepts a drop. Without it, the browser treats the target as non-droppable and swallows the `drop` event silently. This is the most common and non-obvious drag-and-drop bug — it fails without any error message.

**How to avoid:**
Always have this boilerplate in the drop zone handler:
```javascript
dropZone.addEventListener('dragover', (e) => {
    e.preventDefault(); // REQUIRED or drop event never fires
    e.dataTransfer.dropEffect = 'move';
});
```
Additionally, use `pointer-events: none` on child elements inside drag targets to prevent child elements from absorbing `dragenter`/`dragleave` events and causing "flickering" drop target highlighting.

**Warning signs:**
- Drag starts correctly (item becomes transparent), but `drop` never fires
- No JS error in console — the event simply never occurs
- The drag cursor shows "no-drop" symbol over what should be a valid target

**Phase to address:** Pipeline Builder Redesign (Phase 5). Write a minimal drag-and-drop test before building out the full step reordering.

---

## Moderate Pitfalls

### Pitfall 7: Notion `select` Property Value Must Match Exactly

**What goes wrong:**
The Notion connector sends a `select` property value that doesn't exactly match one of the database's configured options. The API returns a 400 validation error. The LLM often capitalizes differently ("high" vs "High"), adds spaces, or generates a value not in the allowed set.

**Prevention:**
The output parser in `connectors/notion.rs` must normalize select values before sending to the API: trim whitespace, case-insensitive match against the allowed options list from the integration profile, and map to the exact stored string. If no match is found, use the configured default value (e.g., "Todo" for status, "Medium" for priority) rather than sending the unrecognized value.

```rust
fn resolve_select_value(raw: &str, options: &[String], default: Option<&str>) -> Option<String> {
    let normalized = raw.trim().to_lowercase();
    options.iter()
        .find(|opt| opt.to_lowercase() == normalized)
        .cloned()
        .or_else(|| default.map(String::from))
}
```

**Phase to address:** Notion Connector (Phase 2).

---

### Pitfall 8: Tauri Keychain Prompts in Dev Mode

**What goes wrong:**
During development (`cargo tauri dev`), the app is not code-signed. Every time the app restarts, macOS treats it as a new unauthorized app and shows a password dialog for each Keychain item. For a developer storing a Notion API key and one or two Slack tokens, this means 3-5 password dialogs per restart.

**Prevention:**
Add a development-only fallback: store credentials in a local `.dev-credentials.json` file (gitignored) when running in debug mode, bypassing Keychain. Check `#[cfg(debug_assertions)]` in the Rust credential storage functions. In production builds, always use Keychain.

```rust
#[cfg(debug_assertions)]
fn get_api_token(service: &str, id: &str) -> Result<String, String> {
    // Read from .dev-credentials.json for dev builds
    read_dev_credential(service, id)
}

#[cfg(not(debug_assertions))]
fn get_api_token(service: &str, id: &str) -> Result<String, String> {
    // Always use Keychain in production
    get_generic_password(KEYCHAIN_SERVICE, &format!("{}:{}", service, id))
        ...
}
```

**Phase to address:** Notion Integration Infrastructure (Phase 1). Implement this before integrating any new credentials.

---

### Pitfall 9: Notion `rich_text` Property Type vs. `text` — Wrong Field Name

**What goes wrong:**
Creating a Notion page with a text property uses the field key `"rich_text"` in the API payload, not `"text"`. Sending `"text"` returns `400 invalid_json`. LLM-generated JSON (when asked to produce output for Notion) often uses `"text"` because that's the natural English word.

**Prevention:**
The Notion connector's property formatter must use the exact API property type keys, not natural language equivalents. Maintain a mapping from Notion property types to their API property value containers:

| Notion Type | API Value Container Key |
|-------------|------------------------|
| title       | `title` (array of rich_text) |
| rich_text   | `rich_text` (array) |
| number      | `number` |
| select      | `select` → `{ "name": "..." }` |
| multi_select| `multi_select` → `[{"name":"..."}]` |
| date        | `date` → `{ "start": "..." }` |
| people      | `people` → `[{"id":"..."}]` |

The connector constructs property payloads from the integration profile's property types — never from LLM output keys.

**Phase to address:** Notion Connector (Phase 2).

---

### Pitfall 10: Pipeline Chip List Becomes Unusable with Many Pipelines

**What goes wrong:**
As the user creates more pipelines (10, 20+), the app bar fills with chips and either overflows off screen or wraps into multiple lines, breaking the app bar layout and obscuring the record button.

**Prevention:**
Cap visible chips at a fixed count (5 is a good default). Add an overflow menu ("+ 3 more") using a dropdown for the remaining pipelines. Persist the user's preferred visible chips separately from the pipeline list. The last-used pipeline should always be in the visible set.

**Warning signs:**
- App bar layout breaks during development when testing with many pipelines
- Record button gets pushed off screen or below the fold

**Phase to address:** Pipeline Chips (Phase 6). Build the overflow from the start, not as a retrofit.

---

### Pitfall 11: Prompt Augmentation Makes Prompts Exceed Context Windows

**What goes wrong:**
For Notion databases with many properties and many team members, the auto-injected format spec becomes very long (listing 20+ fields, 50+ people). Combined with a long transcript (1-hour meeting), the total prompt may exceed the model's context window, causing a truncation error or degraded output quality.

**Prevention:**
Keep the injected format spec focused: only include fields marked as "required" or "commonly used" in the integration profile, not all fields. Limit the people list to the most active team members (configurable). The format spec should be < 500 tokens in typical use. In the integration profile, allow the user to mark fields as "include in prompt" vs. "optional".

**Warning signs:**
- LLM returns truncated output for long meetings
- API returns 400 "max tokens exceeded"
- Output quality degrades for complex Notion schemas

**Phase to address:** Notion Integration Setup Wizard (Phase 4) — add field relevance controls.

---

### Pitfall 12: UI Health Check Runs Synchronously During App Startup

**What goes wrong:**
The health check performs DOM queries, simulates clicks, and potentially calls Tauri commands. If run synchronously during `DOMContentLoaded`, it blocks the first render frame and makes the app feel slow to start.

**Prevention:**
Defer the health check using `requestIdleCallback` or `setTimeout(runHealthCheck, 1000)` — run it after the UI has fully rendered and the user can see it. The health check badge should appear after the app is interactive, not as a condition for being interactive.

```javascript
document.addEventListener('DOMContentLoaded', () => {
    initializeApp(); // immediate: render UI
    // Health check is deferred — runs after user sees the app
    requestIdleCallback(() => runUIHealthCheck(), { timeout: 5000 });
});
```

**Phase to address:** UI Health Check (Phase 8).

---

## Minor Pitfalls

### Pitfall 13: Notion API Rate Limit (3 req/sec) Blocks Setup Wizard

**What goes wrong:**
The setup wizard makes multiple sequential API calls: validate token → list databases → fetch schema → list users. If each call is made immediately after the previous one, no rate limiting issues occur for 3-5 calls. However, if the wizard is extended (e.g., listing 50 databases and fetching previews for each), the 429 rate limit kicks in.

**Prevention:**
For v1, the wizard makes at most 4-5 API calls total (well under the 3/sec sustained limit). No special handling needed. For future database-browsing features, add sequential queuing with a 400ms delay between calls. Never fire multiple Notion API calls in parallel.

**Phase to address:** Notion Integration Infrastructure (Phase 1).

---

### Pitfall 14: Drag-and-Drop Ghost Image Uses Wrong Element

**What goes wrong:**
The default browser drag ghost image is an out-of-focus screenshot of the entire dragged element, often at the wrong scale or with cut-off text. The ghost image floats at a position offset that feels wrong relative to the cursor.

**Prevention:**
Use `event.dataTransfer.setDragImage(element, offsetX, offsetY)` with a custom compact element that matches the drag interaction affordance. For step cards, a 200px-wide "mini card" showing just the step name is sufficient.

**Phase to address:** Pipeline Builder Redesign (Phase 5) — minor polish, not a blocker.

---

### Pitfall 15: Notion Integration Profile Becomes Stale After Schema Changes

**What goes wrong:**
A team member adds a new property to the Notion database (e.g., "Sprint" select field). The integration profile still has the old schema. The AI prompt spec doesn't mention "Sprint", so the AI ignores it. New entries are created without the Sprint field populated.

**Prevention:**
Show the `synced_at` timestamp prominently in the integration card in Settings. Add a "Sync now" button that is easy to find. On pipeline execution, if the profile is older than 7 days, show a warning badge (not an error — the profile still works for existing fields). Do not auto-sync on every execution (adds latency, unnecessary API calls).

**Phase to address:** Notion Integration Settings UI (Phase 4).

---

### Pitfall 16: `ConnectorType::Mcp` Left as Unimplemented Placeholder

**What goes wrong:**
The existing `pipeline_engine.rs` has `ConnectorType::Mcp => Err("MCP connector not yet implemented")`. If a user somehow has a pipeline with an MCP step in storage (from future experiments), pipeline execution fails silently on that step and emits a confusing error.

**Prevention:**
The pipeline builder's preset picker should never expose MCP as a selectable step type until it's implemented. The validation in `validate_pipeline()` should reject pipelines with MCP steps with a clear message: "MCP connector is not available in this version."

**Phase to address:** Pipeline Builder Redesign (Phase 5) — filter MCP from the picker.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Storing Notion schema in `settings.json` alongside app settings | Simpler code, one file | Settings file bloat, slow writes, mutex contention, hard to sync per-integration | Never — use separate profile files from day one |
| Fetching Notion schema on every pipeline run (no cache) | Always fresh schema | 2-3 extra API calls per run, network dependency, rate limit risk, slower pipelines | Never — snapshot at setup time |
| One large `main.js` with all builder and integration logic | Fewer files to manage | File becomes 2000+ lines, hard to debug, difficult to unit-test sections | Never — extract `pipeline-builder.js` and `integrations-settings.js` before writing them |
| Skipping `preventDefault()` in drag-and-drop handler | Saves 1 line | Drop events never fire; entire feature is broken | Never |
| Embedding format spec in user-editable prompt templates | User can customize | User must update spec when DB schema changes; spec becomes inconsistent | Never for structured connectors |
| Allowing any pipeline chips count in app bar | No limit code needed | App bar overflows, record button gets pushed off screen | Never — cap at 5 from the start |
| Using training-data knowledge of Notion API without verifying current docs | Faster to build | Breaks on API version changes (2025-09-03 introduced breaking changes) | Never — verify Notion API docs before implementation |

---

## Integration Gotchas

Common mistakes when connecting to external services.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Notion | Not sharing the integration with the database in the Notion UI | Include explicit step in setup wizard: "Open database → ••• → Connections → Add integration" |
| Notion | Sending `database_id` in relation properties with API v2025-09-03 | Pin to `Notion-Version: 2022-06-28` for v1; document this |
| Notion | Using `"text"` as property container instead of `"rich_text"` | Maintain a hardcoded property type → API key mapping in the connector |
| Notion | Filtering `GET /v1/users` by name/email | Fetch all users, match client-side |
| Notion | Sending select value that doesn't match allowed options exactly | Normalize and case-insensitive match against stored options; fall back to profile default |
| Notion | Sending date as a plain string instead of `{ "start": "2026-02-18" }` | Use the correct date object structure in the property payload |
| Slack | Already implemented correctly in `integrations.rs` — minimal gotchas | N/A |
| macOS Keychain | Dev mode prompts 1 password dialog per credential per restart | Use `#[cfg(debug_assertions)]` to skip Keychain in dev; never in production |

---

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Re-fetching Notion schema on every pipeline run | Slow pipelines, rate limit 429s | Cache in integration profile; manual sync button | >3 pipelines/day running |
| Rendering pipeline chips without an overflow limit | App bar layout break, hidden record button | Cap at 5 chips + overflow | >6 pipelines |
| Full DOM rebuild of recording list on every pipeline-progress event | UI jank during pipeline execution | Update only the specific recording row's status badge | >20 recordings in list |
| Firing multiple concurrent Notion API calls during setup | 429 rate limit errors | Sequential calls with a single queue | 2+ simultaneous calls |
| Augmenting prompts with full property/people list (no filtering) | Context window exceeded, LLM truncates | Mark fields as "include in prompt" in profile; limit to required fields | Notion DB with 15+ properties or 20+ team members |

---

## Security Mistakes

Domain-specific security issues beyond general web security.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Storing Notion API key in `settings.json` (plaintext, world-readable) | Token leaked if backup or config is shared | Always store in macOS Keychain via `security_framework`; already pattern used for Slack |
| Logging the Notion API token in Tauri event or error output | Token exposed in logs | Never include token values in error messages or Tauri events; log "token retrieved" not "token: xoxb-..." |
| Passing Notion integration ID to the frontend (JS) in a way that includes the token | Frontend can extract and exfiltrate the token | Backend holds the token; frontend only passes integration IDs; Rust fetches token from Keychain per request |
| Trusting LLM output as valid JSON without parsing/validation | Malformed JSON crashes the connector; adversarial transcripts could attempt prompt injection via injected instructions in the audio content | Always parse and validate against the integration profile schema; sanitize before sending to Notion API |
| Not validating Notion property names from LLM output | LLM could generate a property name not in the schema, causing API errors or data written to wrong fields | Only send properties that exist in the integration profile schema; discard unrecognized keys |

---

## UX Pitfalls

Common user experience mistakes in this domain.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Making pipeline chip click navigate to pipeline detail view | Interrupts user's intent to record; adds friction to most-used action | Chip click = immediate `startRecordingWithPipeline(name)`; edit pipeline is a separate path (Settings) |
| Showing AI-injected format spec to user in prompt preview | Confusing; users don't know what Notion property format instructions are | Keep augmentation invisible; show base prompt only; optionally show "AI receives additional format instructions from Notion schema" as a subtle note |
| Requiring user to re-enter integration credentials after schema sync | Frustrating; breaks trust in the integration | Schema sync only re-fetches DB structure; never invalidates the stored API key |
| Showing a single "Error" badge in app bar without details | User doesn't know what to fix | Health check badge must link to a specific diagnostic report listing each failed check and suggested fix |
| Asking user to name each integration step manually | Cognitive overhead; most users don't care about step names | Auto-generate names from presets ("meeting_notes", "action_items"); user can rename if desired |
| Showing Notion wizard as a full-page navigation (leaving settings) | Loses wizard context mid-flow; back button confusion | Wizard should be a modal overlay within the Settings page, not a navigation destination |

---

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **Notion integration setup:** Often missing the "share with database" step in the wizard — verify the wizard explicitly shows this with instructions, not just assumes it's done
- [ ] **Pipeline builder save:** Often saves the visually-displayed state but there's a state/DOM desync — verify saved JSON matches what's displayed by re-loading and comparing
- [ ] **Drag-and-drop reorder:** Often works visually but the underlying state array order is wrong — verify save order matches drag order with a test: drag step 1 to position 3, save, reload, confirm order
- [ ] **Schema-aware prompt augmentation:** Often the format spec is injected but the LLM ignores it — verify with a real transcript that the output is a valid JSON array before calling it complete
- [ ] **Notion page creation:** Often creates a page but leaves title blank (if `title` property mapping is wrong) — verify the created Notion page has the correct title property populated
- [ ] **People mapping:** Often the alias lookup works for exact matches but fails for aliases with different casing — verify case-insensitive matching in the connector
- [ ] **Pipeline chips:** Often the "last used" chip isn't persisted across app restarts — verify the highlighted state survives a restart via localStorage or settings
- [ ] **UI health check badge:** Often the badge shows green because the check runs before all async data (integrations, pipelines) has loaded — verify the check runs after all invoke() calls on startup have completed

---

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Notion API version mismatch breaking existing pipelines | LOW | Pin `Notion-Version` header to `2022-06-28`; re-test; no data migration needed |
| Integration profile JSON corrupted | LOW | Delete profile file; re-run setup wizard to re-sync; no user data lost |
| State/DOM desync in builder (user saved wrong pipeline) | MEDIUM | User must re-open pipeline and manually correct steps; no automatic recovery |
| LLM output not parseable as JSON (one run) | LOW | Re-run pipeline; pipeline engine shows exact error; user can re-trigger |
| Notion API key expired (token revoked) | LOW | User re-enters API key in Settings > Integrations; token is replaced in Keychain |
| Notion database schema changed (profile stale) | LOW | Click "Sync" in Settings > Integrations; no data loss; old runs are unaffected |
| Pipeline chip overflow breaks app bar layout | LOW | Cap at N in code; existing chips still work; layout fix is a CSS change |
| Tags → Pipeline labels migration corrupts metadata | HIGH | Never run migration on all recordings at once; run per-recording lazily on access; keep `tags` field for 2 versions before removing |

---

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Notion API version mismatch | Phase 1: Notion Infrastructure | Pin header constant; integration test against real API |
| Integration not shared with DB | Phase 4: Integration Setup Wizard | Wizard includes explicit share step; error message on 404 shows instructions |
| LLM returns non-JSON (augmentation fails) | Phase 3: Prompt Augmentation | `build_augmented_prompt()` returns `Result<>`, not silent fallthrough |
| People property not queryable by name | Phase 4: Integration Setup Wizard | Wizard fetches full user list; maps client-side |
| `select` value case mismatch | Phase 2: Notion Connector | `resolve_select_value()` is case-insensitive with fallback to default |
| Vanilla JS state/DOM desync | Phase 5: Pipeline Builder | State-first pattern established before any drag-and-drop |
| Missing `preventDefault()` in dragover | Phase 5: Pipeline Builder | Manual test: drag a step and confirm drop fires before building full feature |
| Notion `rich_text` vs `text` property key | Phase 2: Notion Connector | Property type mapping table; verified against Notion API docs |
| Dev mode Keychain prompts | Phase 1: Notion Infrastructure | `#[cfg(debug_assertions)]` dev credential bypass implemented first |
| Prompt augmentation exceeds context | Phase 4: Integration Setup Wizard | Profile includes field relevance toggle; default limits to 10 fields |
| Tags migration data corruption | Phase 7: Tags → Pipeline Labels | Lazy per-recording migration; backup tags field for 2 versions |
| Health check runs on startup synchronously | Phase 8: UI Health Check | `requestIdleCallback` deferred execution; verified via startup profiling |
| Integration profile stored in settings.json | Phase 1: Notion Infrastructure | Separate profile files from day one; never in `AppSettings` struct |
| Chip count overflow | Phase 6: Pipeline Chips | 5-chip cap with overflow menu implemented from initial render |

---

## Sources

- Notion API Upgrade Guide (2025-09-03): https://developers.notion.com/docs/upgrade-guide-2025-09-03 (HIGH confidence)
- Notion API Request Limits: https://developers.notion.com/reference/request-limits (HIGH confidence)
- Notion API Authorization: https://developers.notion.com/docs/authorization (HIGH confidence)
- Notion API User Reference: https://developers.notion.com/reference/user (HIGH confidence)
- Notion MCP Server Bug: date properties silently dropped: https://github.com/makenotion/notion-mcp-server/issues/121 (MEDIUM confidence)
- Notion MCP Server Bug: schema validation failure on page update: https://github.com/makenotion/notion-mcp-server/issues/153 (MEDIUM confidence)
- OWASP LLM01:2025 Prompt Injection: https://genai.owasp.org/llmrisk/llm01-prompt-injection/ (HIGH confidence)
- LLM Structured Output Reliability Guide (2025): https://www.cognitivetoday.com/2025/10/structured-output-ai-reliability/ (MEDIUM confidence)
- LLM Output Parsing and Retry Strategies: https://apxml.com/courses/prompt-engineering-llm-application-development/chapter-7-output-parsing-validation-reliability/handling-parsing-errors (MEDIUM confidence)
- HTML5 Drag and Drop API Pitfalls: https://medium.com/@reiberdatschi/common-pitfalls-with-html5-drag-n-drop-api-9f011a09ee6c (MEDIUM confidence)
- Tauri macOS Keychain dev mode prompts: https://github.com/tauri-apps/tauri/issues/8662 (HIGH confidence — official GitHub issue)
- Tauri State Management: https://v2.tauri.app/develop/state-management/ (HIGH confidence)
- Existing codebase: `/workspace/src-tauri/src/pipeline_engine.rs`, `/workspace/src-tauri/src/integrations.rs`, `/workspace/src-tauri/src/connectors/` (HIGH confidence — direct analysis)

---
*Pitfalls research for: NBP Pipelines v2 — Notion API integration, schema-aware AI output, vanilla JS pipeline builder, Tauri desktop patterns*
*Researched: 2026-02-18*
