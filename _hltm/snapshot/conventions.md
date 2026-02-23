# Conventions

## Naming

- Files: `snake_case.rs` (Rust), `kebab-case.js` or `camelCase.js` (JS). Rust modules match file names.
- Functions: `snake_case` (Rust), `camelCase` (JS)
- Types/Structs: `PascalCase` (Rust)
- Tauri commands: `snake_case` — registered in `lib.rs` invoke_handler
- Tauri events: `snake_case` with underscores (e.g. `llm_download_progress`, `realtime_transcript_delta`)
- Beads issue IDs: `nbp-xxx` (3 char alphanumeric)

## File Organization

- New Rust backend modules → `src-tauri/src/` (top-level .rs file or subdirectory with mod.rs)
- New connectors → `src-tauri/src/connectors/`
- New integrations → `src-tauri/src/integrations/`
- Cloud AI providers → `src-tauri/src/cloud_ai/`
- Audio processing internals → `src-tauri/src/audio_processing/`
- Frontend JS → `src/` (flat, no subdirectories)
- Static assets → `src/assets/`

## Error Handling

- Rust backend: functions return `Result<T, String>` for Tauri commands (Tauri requires String errors). Internal functions use `anyhow::Result` where convenient.
- Frontend: try/catch around Tauri invoke calls, errors shown via toast notifications or console.
- Mutex poisoning: handled with `unwrap_or_else(|e| e.into_inner())` pattern throughout audio code to prevent panic propagation from poisoned locks.

## Patterns

### Shared Audio Buffers
Global lazy_static ring buffers (`SharedAudioBuffer`, `MonoAudioBuffer`) for inter-thread audio data passing. Producer pushes, consumer pops. Oldest samples evicted when full. Used for mic, system, and transcription audio streams.

### Tauri Event Emission for Progress
Long-running operations (download, transcription, pipeline execution) emit progress events via `app_handle.emit("event_name", payload)`. Frontend listens with `window.__TAURI__.event.listen()`. Throttled to ~200ms intervals to avoid flooding.

### Provider-First Config
Provider configuration keyed by provider ID in a HashMap. Each provider has: api_key, capabilities, models. Legacy role-based API keys migrated into this structure on load/save.

### Settings Load/Save Cycle
`config::load_settings` reads JSON + merges defaults → frontend reads via Tauri command → user edits in UI → frontend calls `config::save_settings` with full settings object. Backwards compatibility via `#[serde(default)]` on all new fields.

## Anti-Patterns

### Don't use npm/npx/yarn/pnpm
Why: Project uses bun exclusively. Other package managers will create wrong lockfiles.
Instead: Use `bun` and `bunx` for all package operations.

### Don't run `cargo tauri dev` for compilation checks
Why: Opens a window and interferes with the user's workflow.
Instead: Use `cargo check` for compilation verification.

### Don't use Bash redirects for file operations
Why: Project rules mandate using Write/Edit tools only.
Instead: Use Write tool for file creation, Edit tool for modifications.

### Don't put interactive prompts in build commands
Why: Automated agents can't respond to prompts. Commands must run non-interactively.
Instead: Use flags to skip prompts or pre-configure answers.
