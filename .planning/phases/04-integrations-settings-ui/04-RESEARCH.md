# Phase 4: Integrations Settings UI - Research

**Researched:** 2026-02-18
**Domain:** Vanilla JS frontend, Tauri invoke API, multi-step wizard UI, integration management
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INTG-01 | Integrations settings page shows Connected and Available sections | Existing `data-tab="integrations"` HTML + `renderSlackIntegrationsList()` Slack pattern establishes the layout foundation; replace with unified Connected/Available two-section pattern |
| INTG-02 | Each connected integration shows Test and Remove actions inline | `test_notion_integration` and `remove_notion_integration` commands are registered in `lib.rs`; Slack pattern already shows Test/Remove inline — same approach applies to Notion |
| INTG-03 | Save paths are first-class integrations with named locations | No Rust backend for save path integrations exists yet; frontend must treat save paths as a new integration type; Tauri dialog plugin is available for folder picker; save path profiles need new Rust commands |
| INTG-04 | Delivery step picker in pipeline builder shows only connected integrations | `allPipelineDefs` and `slackIntegrations` already loaded at page open; a new `allConnectedIntegrations` array needs to be populated by loading notion profiles + save paths + slack at settings load; pipeline builder delivery filter reads from this array |
| NOTN-03 | Setup wizard: user picks database from list fetched via Notion API | `list_notion_databases(integrationId)` command exists and returns `Vec<NotionDatabaseInfo>` (`{id, name}`); returns error string with share instructions if 0 databases found |
| NOTN-04 | Setup wizard: app reads database schema automatically | `sync_notion_schema(integrationId, databaseId, databaseName)` command exists and returns full `NotionIntegrationProfile` with `properties` array, `workspace_users`, `synced_at` |
| NOTN-05 | Setup wizard: user maps conversation aliases to Notion workspace users (people mapping) | `update_notion_people_mappings(integrationId, mappings)` command exists; `workspace_users` field on profile has `{id, name}` for dropdown; `people_mappings` array has `{alias, notion_user_id, display_name}` |
</phase_requirements>

---

## Summary

Phase 4 is a pure frontend implementation. All Notion backend commands were built in Phase 1 and are registered in `lib.rs`. The frontend needs to: (1) redesign the existing `data-tab="integrations"` settings panel into a unified Connected/Available layout; (2) implement a multi-step Notion setup wizard as a modal overlay; (3) add save path integrations (which require new Rust commands for CRUD — this is the only backend gap in this phase); and (4) wire the delivery picker in the existing pipeline builder to read from connected integrations.

The codebase uses a consistent pattern: state is loaded into a module-level variable (e.g., `slackIntegrations`, `allPipelineDefs`), then `render*()` performs a full DOM re-render from that state. The Slack integration in `main.js` is the closest existing reference implementation — it covers the Connected section list, Test/Remove buttons, and an Add modal. Phase 4 extends this exact pattern to cover Notion and save paths, and restructures the integrations tab into the two-section Connected/Available layout.

**Primary recommendation:** Follow the Slack pattern exactly for the integrations list and action buttons. Use a stepped modal (wizard) for Notion setup, identical in structure to the existing delete-modal and add-slack-modal. Save path integrations need a new Rust module (`integrations/save_path.rs`) with list/add/update/remove commands plus a new Tauri command for folder picking, before the frontend can be implemented.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Tauri `invoke` | 2.x | Call Rust commands from JS | Established pattern throughout `main.js` |
| `window.__TAURI__.dialog` | 2.x (tauri-plugin-dialog) | Folder picker for save path UI | Already in `Cargo.toml` and used in browse-storage-btn handler |
| Vanilla JS DOM | — | All UI rendering | Project constraint: no framework, no bundler |
| CSS custom properties (`--accent`, `--border`, etc.) | — | Theme-consistent styling | All existing UI uses these variables |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| SortableJS | — | Drag-and-drop (Phase 5) | NOT needed in Phase 4; delivery picker in Phase 4 only reads integrations |
| `escapeHtml()` | built-in helper | XSS prevention for user-supplied names | Every time integration name, path, or alias is rendered into innerHTML |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Modal overlay wizard | Inline accordion steps | Modal matches existing `add-slack-modal` pattern; inline would require new CSS patterns |
| Full re-render on state change | Targeted DOM patch | Full re-render matches project philosophy (`renderSlackIntegrationsList` pattern); simpler, less buggy |
| New `.js` module file | Add to `main.js` | New module per roadmap plan (`integrations-settings.js`); keeps main.js manageable; loaded via `<script src="integrations-settings.js">` before `</body>` |

**Installation:** No new packages. Tauri dialog plugin already installed.

---

## Architecture Patterns

### Recommended Project Structure
```
src/
├── main.js                        # Existing: no changes to core structure
├── integrations-settings.js       # NEW: all Phase 4 JS (04-01, 04-02, 04-03)
├── index.html                     # Modify: integrations tab HTML, wizard modal HTML
├── styles.css                     # Modify: integration card, wizard step styles
src-tauri/src/integrations/
├── mod.rs                         # Modify: add save_path module
├── notion.rs                      # Existing: all commands already implemented
├── slack.rs                       # Existing: reference implementation
└── save_path.rs                   # NEW: add/list/update/remove save path integrations
```

### Pattern 1: Module-Level State + Full Re-render
**What:** Each integration type has a module-level state variable loaded via `invoke()`. A `render*()` function rebuilds the entire DOM section from state on every change.
**When to use:** Always — this is the established project pattern.
**Example:**
```javascript
// Source: src/main.js (Slack integration pattern, lines 1430-1568)
let slackIntegrations = {};

async function loadSlackIntegrations() {
  try {
    slackIntegrations = await invoke('list_slack_integrations');
    renderSlackIntegrationsList();
  } catch (err) {
    console.error('Failed to load Slack integrations:', err);
  }
}

function renderSlackIntegrationsList() {
  if (!slackIntegrationsListEl) return;
  const entries = Object.entries(slackIntegrations);
  if (entries.length === 0) {
    slackIntegrationsListEl.innerHTML = '<div ...>No Slack workspaces connected yet</div>';
    return;
  }
  slackIntegrationsListEl.innerHTML = entries.map(([id, data]) => `
    <div class="integration-item" data-id="${escapeHtml(id)}" ...>
      ...
      <button class="mini-action-btn test-slack-btn" data-id="${escapeHtml(id)}">Test</button>
      <button class="mini-action-btn danger remove-slack-btn" data-id="${escapeHtml(id)}">Remove</button>
    </div>
  `).join('');
  // Attach event listeners after render
  slackIntegrationsListEl.querySelectorAll('.test-slack-btn').forEach(btn => { ... });
}
```

### Pattern 2: Multi-Step Modal Wizard
**What:** A modal overlay (`modal-overlay` class) with internal step navigation. State tracks `currentStep` and partial data (e.g., `integrationId`, `selectedDatabaseId`). Each step advances on success of the previous async `invoke()` call.
**When to use:** Notion setup wizard (API key → DB picker → schema display → people mapping).
**Example:**
```javascript
// Based on existing modal pattern in src/index.html + src/main.js
// See: add-slack-modal (lines 683-720 index.html) and its JS handler (lines 1533-1568 main.js)
let notionWizardState = {
  step: 0,            // 0=api-key, 1=share-instruction, 2=db-picker, 3=schema, 4=people-mapping
  integrationId: null,
  selectedDbId: null,
  selectedDbName: null,
  profile: null,
};

async function advanceWizardStep(nextStep) {
  notionWizardState.step = nextStep;
  renderWizardStep();
}

function renderWizardStep() {
  const { step } = notionWizardState;
  wizardBody.innerHTML = WIZARD_STEPS[step].render(notionWizardState);
  // Attach step-specific handlers
  WIZARD_STEPS[step].attach(notionWizardState);
}
```

### Pattern 3: Tauri Folder Picker for Save Path
**What:** `window.__TAURI__.dialog.open({ directory: true })` opens a native macOS folder picker. Returns selected path or null.
**When to use:** When user clicks "Browse" to configure a save path integration.
**Example:**
```javascript
// Source: src/main.js line 1282 (browse-storage-btn handler)
const selected = await window.__TAURI__.dialog.open({
  directory: true,
  multiple: false,
  defaultPath: appSettings.storage_path
});
if (selected) {
  // use selected path
}
```

### Pattern 4: Event Delegation After innerHTML Replacement
**What:** After `innerHTML` is set, re-attach event listeners by querying the newly rendered elements. Never cache references to elements inside an `innerHTML` target — they are destroyed on re-render.
**When to use:** Every `render*()` function.
**Example:**
```javascript
// Source: src/main.js renderSlackIntegrationsList (lines 1484-1516)
container.innerHTML = buildHtml();
container.querySelectorAll('.action-btn').forEach(btn => {
  btn.addEventListener('click', handler);
});
```

### Pattern 5: Settings Tab Content Separation
**What:** Each settings tab is a `<div class="settings-grid settings-tab-content" data-tab="tabname">` div. The tab switcher in `main.js` toggles `active` class on both the tab button and content div. No routing.
**When to use:** All integrations tab content lives inside `data-tab="integrations"`.
**Example:**
```html
<!-- Source: src/index.html lines 598-620 (integrations tab) -->
<div class="settings-grid settings-tab-content" data-tab="integrations">
  <!-- Connected and Available sections go here -->
</div>
```

### Anti-Patterns to Avoid
- **Caching DOM references inside re-rendered containers:** `const btn = document.getElementById('test-btn')` at module level will become stale after re-render. Query inside the render function or use event delegation.
- **Storing API keys in integration profile JSON:** The Notion API key is in Keychain/dev-credentials only. The profile at `~/.nbp/integrations/notion-{id}.json` contains `id`, `name`, `database_id`, `properties`, etc. — never the key itself.
- **Blocking the wizard on the share instruction step:** The DB picker step may return an empty list with an error from the backend. This is expected when the user hasn't shared the integration yet. The 404 / empty-list error handler must show the same share instructions, not a generic error.
- **Assuming `list_notion_databases` succeeds immediately:** After `add_notion_integration`, the user must first share the integration in the Notion UI. The wizard must include a dedicated "share instruction" step before the DB picker step.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Folder picker dialog | Custom path text input | `window.__TAURI__.dialog.open({ directory: true })` | Already in use for storage path, returns native macOS picker |
| XSS-safe HTML rendering | Custom escaping | `escapeHtml()` (line 2 of main.js) | Already implemented and used throughout |
| Notion API validation | Custom API key test | `add_notion_integration(name, apiKey)` Tauri command | Validates key via bot-user endpoint; Phase 1 implementation |
| Database listing | Custom API call from JS | `list_notion_databases(integrationId)` Tauri command | Handles Notion search API, returns `[{id, name}]` |
| Schema sync | Manual property parsing | `sync_notion_schema(integrationId, dbId, dbName)` Tauri command | Returns full `NotionIntegrationProfile` with properties + users |
| People mapping save | Custom profile write | `update_notion_people_mappings(integrationId, mappings)` Tauri command | Validates all user IDs before write; Phase 1 implementation |
| Integration test | Custom API call | `test_notion_integration(integrationId)` Tauri command | Returns "Connected" string or error |
| Integration removal | Manual file deletion | `remove_notion_integration(integrationId)` Tauri command | Handles both token deletion and profile file removal atomically |

**Key insight:** All Notion backend operations are done. Phase 4 is 100% frontend wiring for Notion. Only save path integration requires new Rust code.

---

## Common Pitfalls

### Pitfall 1: Share Instruction Step Must Come Before DB Picker
**What goes wrong:** The wizard calls `list_notion_databases(id)` immediately after `add_notion_integration` succeeds. The Notion API returns 0 results (or an error) because the user hasn't yet shared the integration with a database. The UI shows a confusing "no databases found" error.
**Why it happens:** The Notion API only returns databases the integration has been explicitly shared with. This requires a manual step in the Notion UI before any database can appear.
**How to avoid:** Insert a mandatory "share instruction" step (Step 1.5 or Step 2) between API key entry and the DB picker. This step shows an illustrated instruction telling the user to open Notion → database → "..." menu → Connections → add integration. The step must complete before fetching databases.
**Warning signs:** Empty database list returned by `list_notion_databases` immediately after successful `add_notion_integration`. The backend already returns a descriptive error: `"No databases found. In Notion, open your database, click '...' menu, then 'Connections', and add your integration."` — this exact text should be shown in the 404/empty-list error handler too.

### Pitfall 2: Stale Integration List in Delivery Picker
**What goes wrong:** The pipeline builder delivery picker shows integrations loaded at app start. If the user adds a new integration in the Integrations tab and then goes to the pipeline builder, the new integration doesn't appear.
**Why it happens:** `slackIntegrations` is loaded once at startup. Notion profiles and save paths would have the same problem.
**How to avoid:** The delivery picker must call `loadAllConnectedIntegrations()` (or equivalent) whenever the Pipelines tab becomes active, or whenever an integration is added/removed. Simplest approach: reload all integrations when the settings Integrations tab is opened and when the Pipelines tab is opened.

### Pitfall 3: People Mapping Row Deletion Race
**What goes wrong:** When the user adds/removes mapping rows dynamically and clicks Save, the final `mappings` array is built by querying the DOM, but a deleted row's reference may still exist if re-render hasn't fired.
**Why it happens:** DOM manipulation between renders can leave stale state.
**How to avoid:** Use the state-first pattern. Maintain `wizardState.mappings` as the authoritative array. Add/remove from the array, then call `renderMappingsRows()`. Never read mappings from the DOM directly — read them from state, refreshed when the user edits a field.

### Pitfall 4: Save Path Integration Has No Backend Yet
**What goes wrong:** Save path integrations (INTG-03) appear in Connected section alongside Notion, but there are no Tauri commands to store, list, or remove named save paths. Calling `invoke('list_save_path_integrations')` before adding the command crashes silently.
**Why it happens:** Phase 1 built Notion backend only. Save paths are a new integration type.
**How to avoid:** Plan 04-03 must include creating `integrations/save_path.rs` with Tauri commands (`add_save_path_integration`, `list_save_path_integrations`, `update_save_path_integration`, `remove_save_path_integration`) before the frontend can be wired. The save path profile JSON (`~/.nbp/integrations/save-path-{id}.json`) follows the same pattern as Notion profiles.

### Pitfall 5: Wizard State Not Cleaned on Close/Reopen
**What goes wrong:** User opens wizard, enters an API key, cancels, then opens wizard again. The previous state (partial `integrationId`, partial `wizardState`) persists, causing the wizard to start at a wrong step or show stale data.
**Why it happens:** Module-level wizard state not reset on modal close.
**How to avoid:** Reset `notionWizardState` to initial values whenever the wizard modal closes (both Cancel and successful completion). On successful completion, call `remove_notion_integration` on any partial `integrationId` if the user cancels mid-wizard after step 1 (the integration profile was created by `add_notion_integration` but never completed).

### Pitfall 6: `list_notion_profiles` Not Registered as Tauri Command
**What goes wrong:** The frontend needs to list all Notion integrations to populate the Connected section. `list_notion_profiles()` exists as a Rust function in `integrations/notion.rs` (line 123) but is NOT a `#[tauri::command]` and is NOT registered in `lib.rs`.
**Why it happens:** Phase 1 implemented the function but it was not exposed as a command because Phase 4 (UI) was deferred.
**How to avoid:** Plan 04-01 must add `#[tauri::command]` to `list_notion_profiles` and register it in `lib.rs` `invoke_handler!`. This is a prerequisite for the Connected section rendering.

---

## Code Examples

Verified patterns from official sources:

### Tauri Command: List Notion Profiles (requires `#[tauri::command]` annotation)
```rust
// Source: src-tauri/src/integrations/notion.rs lines 123-153
// Currently a plain function — needs #[tauri::command] added for Phase 4
#[tauri::command]  // ADD THIS
pub fn list_notion_profiles() -> Result<Vec<NotionIntegrationProfile>, String> {
    let dir = crate::config::get_integrations_dir();
    // ... (existing implementation unchanged)
}
```

### Tauri Command: Call from JS (add_notion_integration)
```javascript
// Source: src-tauri/src/integrations/notion.rs (add_notion_integration signature)
// Returns: integration ID string on success, error string on failure
try {
  const integrationId = await invoke('add_notion_integration', {
    name: 'My Notion',
    apiKey: apiKeyInputValue
  });
  // integrationId is a UUID string
} catch (err) {
  // err is the Rust error string
  showError(err);
}
```

### Tauri Command: sync_notion_schema (DB picker → schema display)
```javascript
// Source: src-tauri/src/integrations/notion.rs (sync_notion_schema signature)
// Returns: NotionIntegrationProfile with {id, name, database_id, database_name, properties, people_mappings, workspace_users, synced_at}
const profile = await invoke('sync_notion_schema', {
  integrationId: notionWizardState.integrationId,
  databaseId: selectedDbId,
  databaseName: selectedDbName
});
// profile.properties: [{name, property_type, select_options[]}]
// profile.workspace_users: [{id, name}]
// profile.synced_at: ISO 8601 string
```

### Tauri Command: update_notion_people_mappings
```javascript
// Source: src-tauri/src/integrations/notion.rs (update_notion_people_mappings signature)
// mappings: [{alias, notion_user_id, display_name}]
await invoke('update_notion_people_mappings', {
  integrationId: notionWizardState.integrationId,
  mappings: notionWizardState.mappings.map(m => ({
    alias: m.alias,
    notion_user_id: m.notionUserId,
    display_name: m.displayName
  }))
});
```

### Connected Integration Card (HTML pattern from Slack)
```javascript
// Source: src/main.js lines 1458-1481 (renderSlackIntegrationsList)
// Adapts to: notion integration card in Connected section
function renderNotionIntegrationCard(profile) {
  const safeName = escapeHtml(profile.name);
  const safeDbName = escapeHtml(profile.database_name || 'No database selected');
  const syncedAt = profile.synced_at
    ? new Date(profile.synced_at).toLocaleDateString()
    : 'Never synced';
  return `
    <div class="integration-item" data-id="${escapeHtml(profile.id)}">
      <div class="integration-item-icon notion-icon">N</div>
      <div class="integration-item-info">
        <div class="integration-item-name">${safeName}</div>
        <div class="integration-item-detail">${safeDbName} · Synced ${syncedAt}</div>
      </div>
      <div class="integration-item-actions">
        <button class="mini-action-btn test-notion-btn" data-id="${escapeHtml(profile.id)}">Test</button>
        <button class="mini-action-btn remove-notion-btn danger" data-id="${escapeHtml(profile.id)}">Remove</button>
      </div>
    </div>
  `;
}
```

### Folder Picker for Save Path
```javascript
// Source: src/main.js lines 1282-1294 (browse-storage-btn handler)
async function browseSavePathFolder(currentPath) {
  const selected = await window.__TAURI__.dialog.open({
    directory: true,
    multiple: false,
    defaultPath: currentPath || undefined
  });
  return selected; // string path or null
}
```

### Tab Switch Pattern (for reference)
```javascript
// Source: src/main.js lines 1219-1231 (settings tab switcher)
document.querySelectorAll('.settings-tab').forEach(t => {
  t.classList.toggle('active', t.dataset.tab === tabName);
});
document.querySelectorAll('.settings-tab-content').forEach(c => {
  c.classList.toggle('active', c.dataset.tab === tabName);
});
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Slack integrations in `settings.json` | Slack integrations still in `settings.json` via `IntegrationsConfig` | Phase 1 | Only Notion is in `~/.nbp/integrations/` as separate JSON files; Slack remains different for now |
| Raw text path in pipeline save step | Named save path integrations (INTG-03) | Phase 4 | Requires new Rust backend; no existing implementation |
| Integrations tab shows only Slack | Integrations tab shows Connected/Available for all types | Phase 4 | Current HTML at line 598-620 of index.html needs full replacement |

**Deprecated/outdated:**
- Current `data-tab="integrations"` HTML (index.html lines 598-620): The existing "Slack Workspaces" section will be fully replaced by the Connected/Available layout. The Slack rendering logic in `main.js` (`loadSlackIntegrations`, `renderSlackIntegrationsList`) will be merged into the new `integrations-settings.js` module.

---

## Open Questions

1. **Save Path Integration Backend Architecture**
   - What we know: No Rust backend for save path integrations exists. The decision is to store them as separate JSON files in `~/.nbp/integrations/`. The save connector already uses `path` in its step config.
   - What's unclear: Should save path profiles be stored as `save-path-{id}.json` in `~/.nbp/integrations/` (same dir as notion profiles)? What fields does the profile need? Minimum: `{id, name, path}`.
   - Recommendation: Store as `save-path-{id}.json` in `~/.nbp/integrations/`. Fields: `id` (UUID), `name` (user-visible, e.g. "Notes folder"), `path` (absolute or `~`-prefixed). Create `integrations/save_path.rs` following the same structure as `notion.rs`. Plan 04-03 must include this work.

2. **`list_notion_profiles` Command Registration Gap**
   - What we know: The function exists in `notion.rs` (line 123) but is not a `#[tauri::command]` and is not in `lib.rs`'s `invoke_handler!`.
   - What's unclear: Whether there are any other unregistered commands needed for Phase 4.
   - Recommendation: In 04-01, annotate `list_notion_profiles` with `#[tauri::command]` and register it. Verify no other Phase 1 commands needed for Phase 4 UI are missing from `invoke_handler!`.

3. **Notion Wizard Cancel Mid-Flow Cleanup**
   - What we know: `add_notion_integration` creates a profile on disk. If the user cancels after step 1 (API key), a partial profile exists at `~/.nbp/integrations/notion-{id}.json`.
   - What's unclear: Should partial profiles be cleaned up on cancel, or are they harmless?
   - Recommendation: On wizard cancel after step 1 (when `notionWizardState.integrationId` is non-null but `database_id` is empty), call `remove_notion_integration(integrationId)` to clean up. This is the safest approach — partial profiles with empty `database_id` would show up in Connected with no database name.

4. **People Mapping UX for Wizard Step 4**
   - What we know: `profile.workspace_users` returns `[{id, name}]` after schema sync. The user needs to map aliases (free-text labels like "me", "СК") to Notion user IDs.
   - What's unclear: Does the wizard pre-populate alias rows from existing `people` type properties in the schema, or does the user add rows manually?
   - Recommendation: Pre-populate one empty row per `people`-type property found in `profile.properties` (there may be 0-3 such properties). Each row has an alias text input and a Notion user dropdown. The user can add more rows. This is the lowest-friction approach.

---

## Sources

### Primary (HIGH confidence)
- `src-tauri/src/integrations/notion.rs` — All Tauri commands for Notion, types (`NotionIntegrationProfile`, `NotionDatabaseInfo`, `PeopleMapping`, `WorkspaceUser`), profile I/O functions
- `src-tauri/src/lib.rs` — Full `invoke_handler!` registration, confirms which commands are callable from JS
- `src/main.js` (Slack section, lines 1430-1568) — Reference implementation for integration list rendering, Test/Remove actions, Add modal pattern
- `src/index.html` (lines 598-620) — Current integrations tab HTML structure to be replaced
- `src/main.js` (browse-storage-btn, lines 1279-1295) — Tauri dialog plugin usage pattern
- `src-tauri/src/connectors/save.rs` — Save connector current implementation (path-based, no named location concept)
- `src-tauri/src/config.rs` — `get_integrations_dir()` returns `~/.nbp/integrations/`

### Secondary (MEDIUM confidence)
- `brainstorming-session-2026-02-18.md` (Part 4: Integrations Architecture, Part 5: Schema-Aware Connectors) — Detailed mockups of Connected/Available layout, wizard steps, delivery picker UX; aligns exactly with requirements
- `.planning/REQUIREMENTS.md` — Canonical requirement definitions for INTG-01..04, NOTN-03..05

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries are in use in the codebase already; no new dependencies
- Architecture: HIGH — derived directly from existing code patterns (`main.js` Slack section) and brainstorming mockups; zero ambiguity on approach
- Pitfalls: HIGH — Pitfalls 1, 4, 6 are concrete gaps found in the actual code (share instruction ordering, save path backend missing, `list_notion_profiles` not registered)
- Open questions: MEDIUM — backend design for save paths and people mapping UX require decisions in planning

**Research date:** 2026-02-18
**Valid until:** Stable — this is an internal codebase; no external API changes will invalidate this research
