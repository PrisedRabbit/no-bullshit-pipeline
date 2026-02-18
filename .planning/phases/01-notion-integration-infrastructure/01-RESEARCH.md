# Phase 1: Notion Integration Infrastructure - Research

**Researched:** 2026-02-18
**Domain:** Notion API client (Rust), macOS Keychain via security-framework, per-integration JSON file storage, dev-mode credential bypass
**Confidence:** HIGH — notion-client 1.0.11 API surface verified via docs.rs; Keychain patterns verified from existing codebase; Notion API versioning confirmed via official docs

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| NOTN-01 | User can add Notion integration via API key (internal integration token) | `add_notion_integration` Tauri command validates via `retrieve_your_tokens_bot_user()`, stores metadata without token in profile JSON |
| NOTN-02 | Notion API key stored securely in macOS Keychain | `security-framework` crate already in Cargo.toml; follow existing Slack pattern: `set_generic_password(KEYCHAIN_SERVICE, "notion:{id}", token)` |
| NOTN-06 | Schema and people mappings stored as Integration Profile | `NotionIntegrationProfile` struct written to `~/.nbp/config/integrations/notion-{id}.json`; never in `AppSettings` |
| NOTN-07 | Schema re-sync available via manual button in integration settings | `sync_notion_schema` Tauri command: calls `retrieve_a_database()` + `list_all_users()`, overwrites profile JSON |
| INTG-05 | Integration profiles stored as separate JSON files per integration (not in settings.json) | Per-integration file at `~/.nbp/config/integrations/notion-{id}.json`; `get_integrations_dir()` helper added to `config.rs` |
</phase_requirements>

---

## Summary

Phase 1 establishes the backend infrastructure that all subsequent Notion phases depend on: API client initialization, credential storage, schema capture, and dev-mode ergonomics. No UI is built in this phase — only Tauri commands and data structures.

The `notion-client` crate (v1.0.11) is the right choice and its method signatures are now confirmed. The `Client::new(token, None)` constructor returns `Result<Client, NotionClientError>`. Token validation is done via `client.users.retrieve_your_tokens_bot_user()` (not a dedicated "validate" endpoint). Database listing uses `client.search.search_by_title(SearchByTitleRequest { filter: Some(Filter { property: FilterProperty::Object, value: FilterValue::Database }), .. })`. Schema reading uses `client.databases.retrieve_a_database(id)`, which returns a `Database` struct with a `properties: HashMap<String, DatabaseProperty>` field. User listing for people-mapping uses `client.users.list_all_users(None, None)`.

The Keychain pattern is already established in `integrations.rs` for Slack tokens. Notion follows the identical pattern using the existing `KEYCHAIN_SERVICE = "com.skopanev.nbp"` constant with account key `"notion:{integration_id}"`. The dev-mode Keychain bypass (using `#[cfg(debug_assertions)]` to read from a gitignored `.dev-credentials.json`) must be implemented before adding any new credentials — confirmed as a critical blocker by both the phase requirements and PITFALLS.md.

**Primary recommendation:** Add `notion-client = "1.0.11"` to Cargo.toml, create `src-tauri/src/integrations/` as a module directory (converting the existing flat `integrations.rs` to `integrations/mod.rs` + `integrations/slack.rs`), add `integrations/notion.rs` with the five Tauri commands, and add `config.rs::get_integrations_dir()` helper. Pin `Notion-Version: 2022-06-28` in a constant.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `notion-client` | `1.0.11` | Notion REST API client | Only actively maintained Rust Notion crate; verified API surface on docs.rs; uses reqwest (already in Cargo.toml); MIT license |
| `security-framework` | `3.5` (already in Cargo.toml) | macOS Keychain storage for API key | Already used for Slack tokens; same pattern applies to Notion |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde` + `serde_json` | `1.x` (already in Cargo.toml) | Serialize/deserialize `NotionIntegrationProfile` to JSON | For all profile file I/O |
| `uuid` | `1.20` (already in Cargo.toml) | Generate stable integration IDs | When user adds an integration, generate a UUID as the stable identifier |
| `chrono` | `0.4.43` (already in Cargo.toml) | ISO timestamp for `synced_at` field in profile | Record when schema was last synced |

### No New Dependencies

All required libraries are already in `src-tauri/Cargo.toml`. The only addition needed is `notion-client = "1.0.11"`.

### Installation

```toml
# Add to src-tauri/Cargo.toml [dependencies]
notion-client = "1.0.11"
```

---

## Architecture Patterns

### Recommended File Structure Change

The existing `integrations.rs` (flat file) must be converted to a module directory to accommodate the Notion integration module alongside Slack:

```
src-tauri/src/
├── integrations/
│   ├── mod.rs          # Re-exports; shared KEYCHAIN_SERVICE constant; dev-mode credential helpers
│   ├── slack.rs        # Existing Slack integration code (moved verbatim)
│   └── notion.rs       # NEW: Notion API client, Tauri commands, profile I/O
├── config.rs           # EXTEND: add get_integrations_dir() helper
└── lib.rs              # EXTEND: register new Notion Tauri commands
```

The `~/.nbp/config/` directory is managed by `config.rs`. Add `get_integrations_dir()`:

```rust
pub fn get_integrations_dir() -> PathBuf {
    get_config_dir().join("integrations")
}
```

Profile files are named `notion-{integration_id}.json` within that directory.

### Pattern 1: Notion API Client Initialization

**What:** `notion-client::endpoints::Client::new(token, None)` returns `Result<Client, NotionClientError>`. Construct the client per-request from the token retrieved from Keychain (do not store the client in Tauri state — tokens may be refreshed or removed). The second parameter is `Option<ClientBuilder>` for custom HTTP client configuration; pass `None` for default.

**When to use:** Inside each Tauri command that needs to make a Notion API call.

**Example:**
```rust
// Source: docs.rs/notion-client/1.0.11 — verified
use notion_client::endpoints::Client;

async fn get_notion_client(integration_id: &str) -> Result<Client, String> {
    let token = get_notion_token(integration_id)?;
    Client::new(token, None)
        .map_err(|e| format!("Failed to initialize Notion client: {}", e))
}
```

### Pattern 2: Token Validation via Bot User Endpoint

**What:** There is no dedicated token validation endpoint. Validation is done by calling `client.users.retrieve_your_tokens_bot_user()`. Returns `Ok(User)` if valid, `Err(NotionClientError)` if the token is invalid (401).

**When to use:** In `add_notion_integration` after receiving the API key from the frontend, before saving to Keychain.

**Example:**
```rust
// Source: docs.rs/notion-client/1.0.11/notion_client/endpoints/users/struct.UsersEndpoint.html
pub async fn retrieve_your_tokens_bot_user(
    &self
) -> Result<User, NotionClientError>

// Usage:
let client = get_notion_client_from_raw_token(&token)?;
let _bot = client.users.retrieve_your_tokens_bot_user().await
    .map_err(|e| format!("Invalid Notion API key: {}", e))?;
// Token is valid — save to Keychain
```

### Pattern 3: Listing Databases the Integration Has Access To

**What:** Use `client.search.search_by_title()` with a filter for `FilterValue::Database`. The `notion-client` search endpoint does not have a dedicated "list databases" method — search with an empty query and database filter is the correct approach.

**When to use:** In `list_notion_databases` command, after the user has validated their API key.

**Example:**
```rust
// Source: docs.rs/notion-client/1.0.11 — verified
use notion_client::endpoints::search::title::request::{
    Filter, FilterProperty, FilterValue, SearchByTitleRequest,
};

let request = SearchByTitleRequest {
    filter: Some(Filter {
        property: FilterProperty::Object,
        value: FilterValue::Database,
    }),
    query: None,
    sort: None,
    start_cursor: None,
    page_size: Some(100),
};

let response = client.search.search_by_title(request).await
    .map_err(|e| format!("Failed to list databases: {}", e))?;
// response.results: Vec<PageOrDatabase>
```

**Important:** Only databases that the integration has been explicitly shared with (via the Notion UI "Connections" menu) will appear. An empty results list does not mean no databases exist — the integration may not have been shared yet.

### Pattern 4: Reading Database Schema

**What:** `client.databases.retrieve_a_database(database_id)` returns `Database` which has `properties: HashMap<String, DatabaseProperty>`. Iterate the map to extract property types and select options.

**When to use:** In `sync_notion_schema`, after the user picks a database.

**Key `DatabaseProperty` variants relevant to NBP:**

| Variant | Inner type | How to extract options |
|---------|-----------|----------------------|
| `DatabaseProperty::Title { name, .. }` | `HashMap<(), ()>` | No options; it's the page title field |
| `DatabaseProperty::RichText { name, .. }` | `HashMap<(), ()>` | No options |
| `DatabaseProperty::Select { name, select, .. }` | `SelectPropertyValue` | `select.options` → `Vec<OptionValue>` → each has `.name: String` |
| `DatabaseProperty::MultiSelect { name, multi_select, .. }` | `SelectPropertyValue` | `multi_select.options` → same pattern |
| `DatabaseProperty::People { name, .. }` | `HashMap<(), ()>` | No options; users listed separately |
| `DatabaseProperty::Date { name, .. }` | `HashMap<(), ()>` | No options |

**Example:**
```rust
// Source: docs.rs/notion-client/1.0.11 — verified
let database = client.databases.retrieve_a_database(database_id).await
    .map_err(|e| format!("Failed to read database schema: {}", e))?;

for (prop_name, prop) in &database.properties {
    match prop {
        DatabaseProperty::Select { select, .. } => {
            let options: Vec<String> = select.options.iter()
                .map(|o| o.name.clone())
                .collect();
            // store in NotionPropertyDef
        }
        DatabaseProperty::People { .. } => {
            // people property — users fetched separately
        }
        _ => {}
    }
}
```

### Pattern 5: Listing Workspace Users for People Mapping

**What:** `client.users.list_all_users(start_cursor, page_size)` returns `ListAllUsersResponse` with `.results: Vec<User>` and `.has_more: bool`, `.next_cursor: Option<String>` for pagination.

**`User` struct fields:** `id: String`, `name: Option<String>`, `avator_url: Option<String>`, `user_type: Option<UserType>`. Note: no email field exists in this struct.

**Important constraints:**
- The Notion API does not support filtering users by name or email. Fetch the full list and filter/match client-side.
- Only users who have been added to the workspace (not just the integration) are returned.
- For workspaces with many users, implement pagination using `has_more` and `next_cursor`.

**When to use:** In `sync_notion_schema`, after reading the DB schema. Store all users in the profile as candidates for people mapping.

**Example:**
```rust
// Source: docs.rs/notion-client/1.0.11 — verified
pub async fn list_all_users(
    &self,
    start_cursor: Option<&str>,
    page_size: Option<u32>
) -> Result<ListAllUsersResponse, NotionClientError>

// Usage (first page):
let response = client.users.list_all_users(None, Some(100)).await
    .map_err(|e| format!("Failed to list users: {}", e))?;
// response.results: Vec<User>
// response.has_more: bool
// response.next_cursor: Option<String>
```

### Pattern 6: Per-Integration JSON Profile Storage

**What:** Each Notion integration is stored as a separate JSON file at `~/.nbp/config/integrations/notion-{id}.json`. The directory is created on first write. The profile never contains the API token — the token lives only in Keychain.

**When to use:** After `sync_notion_schema` completes successfully.

**Example:**
```rust
pub fn save_notion_profile(profile: &NotionIntegrationProfile) -> Result<(), String> {
    let dir = config::get_integrations_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("notion-{}.json", profile.id));
    let content = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("Failed to serialize profile: {}", e))?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_notion_profile(integration_id: &str) -> Result<NotionIntegrationProfile, String> {
    let path = config::get_integrations_dir().join(format!("notion-{}.json", integration_id));
    let content = fs::read_to_string(&path)
        .map_err(|_| format!("Notion integration '{}' profile not found. Sync schema in Settings > Integrations.", integration_id))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse integration profile: {}", e))
}
```

### Pattern 7: Dev-Mode Keychain Bypass

**What:** Use `#[cfg(debug_assertions)]` to read/write credentials from a local `.dev-credentials.json` file instead of Keychain during `cargo tauri dev`. The file is gitignored. Production builds always use Keychain.

**When to use:** In all token storage/retrieval functions in `integrations/mod.rs`.

**Example:**
```rust
// Source: PITFALLS.md — verified pattern for macOS Keychain dev-mode issue
#[cfg(debug_assertions)]
fn save_token_to_store(service: &str, id: &str, token: &str) -> Result<(), String> {
    // Read .dev-credentials.json, update, write back
    let path = PathBuf::from(".dev-credentials.json");
    let mut creds: serde_json::Value = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    creds[format!("{}:{}", service, id)] = serde_json::Value::String(token.to_string());
    fs::write(&path, serde_json::to_string_pretty(&creds).unwrap())
        .map_err(|e| e.to_string())
}

#[cfg(not(debug_assertions))]
fn save_token_to_store(service: &str, id: &str, token: &str) -> Result<(), String> {
    let account = format!("{}:{}", service, id);
    set_generic_password(KEYCHAIN_SERVICE, &account, token.as_bytes())
        .map_err(|e| format!("Failed to save token to Keychain: {}", e))
}
```

### Pattern 8: Tauri State for Keychain Service Constant

The `KEYCHAIN_SERVICE` constant (`"com.skopanev.nbp"`) is already defined in `integrations.rs`. When converting to the module structure, move it to `integrations/mod.rs` so both `slack.rs` and `notion.rs` can share it.

### Anti-Patterns to Avoid

- **Storing the Notion token in the integration profile JSON:** The profile is written to disk in `~/.nbp/config/integrations/` which has standard macOS file permissions. The token must only ever be in Keychain (or `.dev-credentials.json` during dev). Never in profile JSON, never in `settings.json`.
- **Storing `NotionIntegrationProfile` inside `AppSettings`:** The `AppSettings` struct is loaded into a `Mutex` at app startup and held for the app lifetime. Adding large schema profiles there causes mutex contention and slow serialization on every settings save. Use separate files.
- **Fetching schema on every pipeline execution:** Schema is a setup-time snapshot. The `sync_notion_schema` command is the only place schema is fetched. Pipeline execution reads from the JSON profile.
- **Logging the API token:** Never include the raw token in error messages, Tauri events, or log output. Log "token retrieved" not the token value.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Notion API types (Database, User, Select options) | Hand-written serde structs for Notion property types | `notion-client` crate | Notion property types are complex heterogeneous enums; the crate already models them correctly |
| HTTP client for Notion API | Raw `reqwest` calls with manual header management | `notion-client::Client` | Notion-Version header pinning, error type normalization, and type-safe responses are handled by the crate |
| Keychain storage | Custom credential file | `security-framework` (already present) | Keychain is the macOS-native secure store; the crate binding is already in Cargo.toml |
| JSON schema for integration profiles | Ad-hoc HashMap structures | Typed `NotionIntegrationProfile` struct with serde | Compile-time field validation, easy serialization, clear intent |

**Key insight:** The `notion-client` crate handles the complex part of Notion integration — the heterogeneous property type system. Do not reproduce that work.

---

## Common Pitfalls

### Pitfall 1: notion-client method signatures — verify with cargo doc before use

**What goes wrong:** The additional_context blocker notes that `notion-client 1.0.11` method signatures were "not fully verified." This research has now verified them via docs.rs. The key verified signatures are documented in the Code Examples section below. However, there is one residual uncertainty: the exact enum variant structure of `DatabaseProperty` in pattern matching may require adjusting based on what `cargo doc` produces after adding the dependency.

**Why it happens:** docs.rs documentation for this crate is only 3.52% documented (their own metric). The type structure is verified but variant field access syntax may differ slightly.

**How to avoid:** After adding `notion-client = "1.0.11"` to Cargo.toml, run `cargo doc --open` and inspect `notion_client::objects::database::DatabaseProperty` before writing the schema-parsing code. Pay particular attention to how `Select` and `MultiSelect` variants expose `select` vs `multi_select` inner fields.

**Warning signs:** Compile errors like "no field `select` on type `DatabaseProperty`" or pattern match exhaustiveness errors.

### Pitfall 2: Integration not shared with the Notion database

**What goes wrong:** `search_by_title` with `FilterValue::Database` returns an empty list even when the user has databases. The integration has not been shared with any database via the Notion UI.

**Why it happens:** Notion's security model requires the user to explicitly "Connect" the integration to each database via the Notion UI (database → ••• menu → Connections → Add connection). The API key alone is insufficient.

**How to avoid:** The error message for an empty database list from `list_notion_databases` must say: "No databases found. In Notion, open your database → ••• → Connections → [your integration name]." Do not say "no databases exist."

**Warning signs:** `retrieve_your_tokens_bot_user()` returns 200 (token valid), but `search_by_title` returns 0 results.

### Pitfall 3: API version mismatch

**What goes wrong:** Using `Notion-Version: 2025-09-03` triggers breaking changes for multi-source databases. Although pinning to `2022-06-28` is correct for Phase 1, verify what version the `notion-client` crate sends by default.

**Why it happens:** The `notion-client` crate "is always up-to-date with the latest Notion API version" per its own README — this may mean it defaults to the latest version. If the crate bumped to `2025-09-03`, it would require adjusting.

**How to avoid:** After adding the dependency, run `cargo doc` and check if `Client` or `ClientBuilder` exposes a `notion_version` parameter. If no version override is possible, check the crate source to see what version header it sends. For Phase 1's operations (user validation, database listing, schema reading), the `2025-09-03` breaking change (multi-source databases) is unlikely to cause issues — it primarily affects `POST /v1/pages` with relation properties. Still, pin to `2022-06-28` by passing through `ClientBuilder` if available.

**Warning signs:** After upgrading notion-client, `retrieve_a_database` returns changed property shapes or unexpected fields.

### Pitfall 4: Dev-mode .dev-credentials.json security

**What goes wrong:** `.dev-credentials.json` is committed to git, exposing API keys.

**How to avoid:** Add to `.gitignore` before writing the dev-mode bypass code. This must be done in the same task that implements the bypass.

---

## Code Examples

Verified patterns from official sources (docs.rs):

### Client Initialization

```rust
// Source: docs.rs/notion-client/1.0.11/notion_client/endpoints/struct.Client.html
use notion_client::endpoints::Client;

pub fn new(
    token: String,
    builder: Option<ClientBuilder>,
) -> Result<Self, NotionClientError>

// In integrations/notion.rs:
async fn make_client(integration_id: &str) -> Result<Client, String> {
    let token = get_notion_token(integration_id)?;
    Client::new(token, None)
        .map_err(|e| format!("Failed to create Notion client: {}", e))
}
```

### Token Validation

```rust
// Source: docs.rs/notion-client/1.0.11/notion_client/endpoints/users/struct.UsersEndpoint.html
pub async fn retrieve_your_tokens_bot_user(
    &self
) -> Result<User, NotionClientError>
```

### Database Listing

```rust
// Source: docs.rs/notion-client/1.0.11 — Filter, FilterProperty, FilterValue verified
use notion_client::endpoints::search::title::request::{
    Filter, FilterProperty, FilterValue, SearchByTitleRequest,
};

let request = SearchByTitleRequest {
    filter: Some(Filter {
        property: FilterProperty::Object,
        value: FilterValue::Database,
    }),
    query: None,
    sort: None,
    start_cursor: None,
    page_size: Some(100),
};
let response = client.search.search_by_title(request).await?;
```

### Schema Reading

```rust
// Source: docs.rs/notion-client/1.0.11/notion_client/endpoints/databases/struct.DatabasesEndpoint.html
pub async fn retrieve_a_database(
    &self,
    database_id: &str
) -> Result<Database, NotionClientError>

// Database.properties: HashMap<String, DatabaseProperty>
// OptionValue: { name: String, color: Option<Color>, id: Option<String> }
```

### User Listing

```rust
// Source: docs.rs/notion-client/1.0.11/notion_client/endpoints/users/struct.UsersEndpoint.html
pub async fn list_all_users(
    &self,
    start_cursor: Option<&str>,
    page_size: Option<u32>
) -> Result<ListAllUsersResponse, NotionClientError>

// ListAllUsersResponse: { results: Vec<User>, next_cursor: Option<String>, has_more: bool }
// User: { id: String, name: Option<String>, avator_url: Option<String>, user_type: Option<UserType> }
// Note: No email field on User struct.
```

### NotionIntegrationProfile Struct (to implement)

```rust
// Defined in integrations/notion.rs
// Source: architecture research — ARCHITECTURE.md
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotionIntegrationProfile {
    pub id: String,                          // UUID, stable identifier
    pub name: String,                        // User-given name for this integration
    pub database_id: String,                 // Notion database UUID
    pub database_name: String,               // Human-readable DB name
    pub properties: Vec<NotionPropertyDef>,  // Schema snapshot
    pub people_mappings: Vec<PeopleMapping>, // alias → notion_user_id
    pub workspace_users: Vec<WorkspaceUser>, // all users from GET /v1/users (for wizard UI)
    pub synced_at: String,                   // ISO timestamp of last sync
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotionPropertyDef {
    pub name: String,
    pub property_type: String,        // "title", "people", "select", "multi_select", "date", "rich_text"
    pub select_options: Vec<String>,  // populated for select/multi_select only
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PeopleMapping {
    pub alias: String,           // "SK", "Sergey"
    pub notion_user_id: String,  // UUID from Notion API
    pub display_name: String,    // "Sergey Kopanev"
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorkspaceUser {
    pub id: String,
    pub name: Option<String>,
}
```

### Tauri Commands to Register (Phase 1 scope)

```rust
// In lib.rs invoke_handler, add:
integrations::notion::add_notion_integration,
integrations::notion::list_notion_databases,
integrations::notion::sync_notion_schema,
integrations::notion::update_notion_people_mappings,
integrations::notion::test_notion_integration,
integrations::notion::remove_notion_integration,
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| Single flat `integrations.rs` (Slack only) | Module directory `integrations/` with `slack.rs` + `notion.rs` | Required to add Notion without making the Slack file unmanageable |
| Integration config in `settings.json` | Per-integration JSON files in `~/.nbp/config/integrations/` | Required per INTG-05; prevents settings.json bloat |
| Keychain only (production) | Keychain in production + `.dev-credentials.json` in debug | Required for dev ergonomics; macOS prompts per restart without code signing |
| Raw reqwest for Slack | Typed `notion-client` crate for Notion | Notion property types are too complex to hand-roll |

**Deprecated/outdated:**
- `jakeswenson/notion` crate: unmaintained since 2022; do not use
- `notionrs` crate: explicitly alpha/unstable as of 2026-02; do not use
- OAuth 2.0 for Notion: out of scope (pre-roadmap decision); API key only

---

## Open Questions

1. **What Notion-Version header does notion-client 1.0.11 send by default?**
   - What we know: The crate claims to be "always up-to-date with the latest Notion API version" — this suggests it may default to `2025-09-03`
   - What's unclear: Whether `ClientBuilder` allows overriding the version header; whether the default version causes issues for Phase 1 operations
   - Recommendation: Check immediately after running `cargo add notion-client@1.0.11` by reading the crate source or running `cargo doc`. For Phase 1 operations (user validation, DB listing, schema reading), `2025-09-03` is unlikely to break anything since the breaking change is specific to page creation with multi-source databases. Flag for Phase 2 (Notion Connector) where page creation occurs.

2. **Does SearchByTitleResponse include both pages AND databases, or only the filtered type?**
   - What we know: `FilterValue::Database` filter should restrict results to databases only
   - What's unclear: The response type `SearchByTitleResponse` may return a `Vec<PageOrDatabase>` union type — if so, the plan must downcast or match on the result type to extract only `Database` variants
   - Recommendation: Verify response type during implementation. The plan for `list_notion_databases` should account for type filtering in the response handler.

3. **Does notion-client 1.0.11 require a specific reqwest feature set?**
   - What we know: notion-client depends on reqwest; the project already has `reqwest = { version = "0.13", features = [...] }`
   - What's unclear: Whether notion-client's internal reqwest version matches 0.13 or if it pins to a different minor version
   - Recommendation: Run `cargo check` after adding the dependency. If there's a version conflict, check the notion-client dependency tree with `cargo tree -i reqwest`.

---

## Sources

### Primary (HIGH confidence)

- `docs.rs/notion-client/1.0.11` — `Client` struct, `new()` signature, `UsersEndpoint`, `DatabasesEndpoint`, `SearchEndpoint` methods all verified
- `docs.rs/notion-client/1.0.11/notion_client/objects/database/struct.Database.html` — `properties: HashMap<String, DatabaseProperty>` verified
- `docs.rs/notion-client/1.0.11/notion_client/objects/database/struct.OptionValue.html` — `OptionValue { name, color, id }` verified
- `docs.rs/notion-client/1.0.11/notion_client/endpoints/users/list/response/struct.ListAllUsersResponse.html` — `{ results, next_cursor, has_more }` verified
- `docs.rs/notion-client/1.0.11/notion_client/endpoints/search/title/request/enum.FilterValue.html` — `Page` and `Database` variants verified
- `/workspace/src-tauri/src/integrations.rs` — existing Slack Keychain pattern (verified directly)
- `/workspace/src-tauri/src/config.rs` — `get_config_dir()`, `KEYCHAIN_SERVICE` location, file permission pattern (verified directly)
- `/workspace/src-tauri/Cargo.toml` — all existing dependencies confirmed; notion-client not yet added
- `developers.notion.com/reference/versioning` — `2022-06-28` still valid; no deprecation timeline

### Secondary (MEDIUM confidence)

- `github.com/takassh/notion-client` README — `Client::new(token, None)` initialization pattern
- `developers.notion.com/reference/authentication` — bearer token Authorization header confirmed

### Tertiary (LOW confidence)

- `DatabaseProperty` variant field access syntax for `Select`/`MultiSelect` inner fields — verified struct exists but exact Rust match arm syntax not confirmed; run `cargo doc` to verify before writing pattern-matching code

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — notion-client 1.0.11 confirmed on docs.rs; all other dependencies already in Cargo.toml
- Architecture: HIGH — existing codebase analyzed directly; Slack pattern confirmed as model
- Pitfalls: HIGH — Keychain dev-mode issue confirmed via Tauri GitHub; Integration sharing requirement confirmed via official Notion docs; API version status confirmed
- API method signatures: HIGH for most; MEDIUM for DatabaseProperty variant field access syntax

**Research date:** 2026-02-18
**Valid until:** 2026-03-18 (30 days — notion-client is actively updated; recheck if version bumps before implementation)
