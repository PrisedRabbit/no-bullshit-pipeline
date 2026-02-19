# Phase 7: Pipeline Data Model and Tags Migration - Research

**Researched:** 2026-02-19
**Domain:** Rust data model evolution, serde migration patterns, lazy per-record migration, zero-step pipeline support, output directory isolation
**Confidence:** HIGH

## Summary

Phase 7 is a **Rust-heavy backend phase** with two main workstreams: (1) data model evolution to store `pipelines` references on `RecordingMetadata` and lazy-migrate existing `tags` fields transparently on access, and (2) removing the zero-step pipeline restriction so a pipeline with no steps can serve as a label.

The good news: the backend already does most of the plumbing. `pipeline_engine.rs::update_pipeline_state()` already writes a `"pipelines"` array into the raw metadata JSON using a `serde_json::Value` approach (bypassing the typed `RecordingMetadata` struct). `read_pipeline_states()` reads from that same field. The `RecordingMetadata` Rust struct just has no `pipelines: Vec<PipelineState>` field yet — so when metadata.json is round-tripped through `RecordingMetadata`, those pipeline states silently survive as extra keys (serde does not strip unknown fields on deserialization by default unless `#[serde(deny_unknown_fields)]` is specified — it is not here). This means the pipeline states already persist safely.

The critical gap is: `RecordingMetadata.tags` (a `Vec<String>`) exists in the struct but there is no `pipelines` typed field. The requirements say recordings should store pipeline references instead of raw string tags (PIPE-03), and that opening a recording with legacy `tags` should transparently show them as pipeline labels (PIPE-04). This means a lazy migration function must: on `read_metadata`, detect if `tags` is non-empty and a corresponding zero-step pipeline label does not yet exist, create those label pipelines in `pipelines.json`, and write pipeline states referencing those labels back to the recording. The `tags` field is retained (never deleted) for backward compatibility.

PIPE-01 and PIPE-05 are engine concerns: zero-step pipelines must be allowed (validation must relax), and the output directory is already per-pipeline-name (`recordings/{id}/pipelines/{pipeline-name}/`), so isolation is already working for executed pipelines. The label-only (zero-step) pipeline just needs to skip execution.

**Primary recommendation:** Implement 07-01 and 07-02 in strict order. 07-01 is all Rust: add `pipelines` field to `RecordingMetadata`, implement the lazy migration in `read_metadata` (or a wrapper), relax the zero-step validation. 07-02 is minimal: ensure `execute_pipeline_internal` gracefully returns early when `steps` is empty (returning `Done` immediately), and add no new output directories for label-only pipelines.

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PIPE-01 | Pipeline with zero steps functions as a label (replaces tags concept) | Two changes needed: (a) remove `if pipeline.steps.is_empty() { return Err(...) }` from `validate_pipeline()` in `pipelines.rs`; (b) early-return `Ok(PipelineStatus::Done)` in `execute_pipeline_internal()` when `pipeline.steps.is_empty()`. Both are small surgical edits. UI chip bar already shows all pipelines; zero-step pipeline will appear as a chip. |
| PIPE-02 | User can create, edit, and delete pipelines with Processing and Delivery steps | Already implemented in Phase 5 (builder). Phase 7 must also remove the frontend guard at `pipeline-builder.js:710` (`if (pipelineEditorSteps.length === 0) { alert... return; }`) to allow zero-step pipelines to be saved as labels. |
| PIPE-03 | Recording metadata stores pipeline references instead of tags | `RecordingMetadata` struct needs a `pipelines: Vec<PipelineState>` field with `#[serde(default)]`. After migration, new recordings will have pipeline states set via `assign_pipeline()` (already works). The `tags` field is retained but treated as legacy-only. |
| PIPE-04 | Existing tag data migrates to pipeline labels automatically on access (lazy migration) | `read_metadata()` (or a `read_metadata_with_migration()` wrapper called from `list_recordings()` and `read_metadata()` Tauri command) checks: if `metadata.tags` is non-empty and none of those tags exist as pipeline-label assignments in the recording's `pipelines` field, creates zero-step pipelines for each tag in `pipelines.json` (idempotent — check if name already exists), then writes pipeline states (Waiting) to the recording metadata. |
| PIPE-05 | Multiple pipelines can be assigned to a single recording, each writing to its own output directory | Already works: `get_pipeline_output_dir()` returns `{data_dir}/{recording_id}/pipelines/{pipeline_name}/`. Multiple pipelines each write to their own subdirectory. The typed `pipelines: Vec<PipelineState>` field allows multiple entries. No new code needed for directory isolation — it was designed this way from the start. |
</phase_requirements>

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde / serde_json | existing (Cargo.toml) | Struct field addition, JSON manipulation | Already used everywhere; `#[serde(default)]` pattern well-established in this codebase |
| Rust std | — | File I/O for lazy migration writes | Already used in `update_pipeline_state()` with flock-based locking |
| Vanilla JS | ES2022 | Remove zero-step guard in `pipeline-builder.js` | No framework, consistent with rest of frontend |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `libc::flock` | existing | File locking during migration writes | Already used in `update_pipeline_state()` — same pattern should apply to migration writes that modify metadata.json |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Lazy per-record migration | Batch migration on startup (like `transcript_migration.rs`) | Batch migration requires touching all recordings at once (slow, risky for large libraries); lazy is safer and was explicitly chosen as the approach per prior decisions |
| New `pipelines` typed field on `RecordingMetadata` | Continue reading via raw JSON as `pipeline_engine.rs` does | Typed field gives compile-time safety and makes list/filter operations possible; raw JSON access was a stopgap that should be formalized |
| Delete `tags` field after migration | Retain `tags` for backward compat | Deleting breaks downgrade (if user rolls back the app); retaining is safe and the prior decision explicitly says so |

**Installation:** No new dependencies needed. All libraries already in the project.

---

## Architecture Patterns

### Recommended Project Structure

Phase 7 modifies existing files only. No new files needed:

```
src-tauri/src/
├── storage.rs          # Add `pipelines` field to RecordingMetadata; implement migrate_tags_to_pipeline_labels(); wire into read_metadata
├── pipelines.rs        # Remove zero-step validation; add zero-step pipeline create helper
├── pipeline_engine.rs  # Early-return for zero-step execution; ensure assign_pipeline works for zero-step
src/
└── pipeline-builder.js  # Remove zero-step guard at line 710
```

### Pattern 1: Adding `pipelines` Field to RecordingMetadata

**What:** Add `pub pipelines: Vec<PipelineState>` with `#[serde(default)]` to `RecordingMetadata`. This is safe because:
- Existing metadata.json files without a `pipelines` key will deserialize with an empty `Vec` (due to `#[serde(default)]`).
- The `pipeline_engine.rs` already writes `pipelines` via raw `serde_json::Value` patching — those writes will now be readable via the typed field too.
- The `write_metadata()` function will now include `pipelines` in the serialized output.

**When to use:** On any Rust struct that needs backward-compatible field addition.

**Example:**
```rust
// Source: codebase pattern from config.rs#[serde(default)] fields
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RecordingMetadata {
    pub id: String,
    pub created_at: String,
    pub title: String,
    pub tags: Vec<String>,   // Retained for backward compat — legacy field
    #[serde(default = "default_status")]
    pub status: String,
    pub audio: AudioFiles,
    #[serde(default)]
    pub health: Option<RecordingHealth>,
    #[serde(default)]
    pub pipelines: Vec<crate::pipeline_engine::PipelineState>,  // NEW: pipeline execution states
}
```

**Critical note on circular dependency:** `PipelineState` lives in `pipeline_engine.rs` and `RecordingMetadata` lives in `storage.rs`. Currently `pipeline_engine.rs` imports from `storage.rs`. Adding a reverse import (`storage.rs` importing from `pipeline_engine.rs`) creates a circular dependency. **Solution:** Move `PipelineState` and related types (`PipelineStatus`, `StepStatus`, `PipelineProgressPayload`) to a new shared module (e.g., `pipeline_types.rs`) or to `pipelines.rs`. Then both `storage.rs` and `pipeline_engine.rs` import from that shared location.

Alternatively, duplicate `PipelineState` as a storage-side type with identical serialization — but this is error-prone. Moving to `pipelines.rs` is cleaner since `pipelines.rs` is already a shared types module.

### Pattern 2: Lazy Migration — migrate_tags_to_pipeline_labels()

**What:** A function that takes a `RecordingMetadata` and, if `tags` is non-empty and no pipeline labels are already assigned for those tags, creates zero-step pipeline definitions in `pipelines.json` for each tag, then writes `Waiting` pipeline states into the recording's `pipelines` field. Returns a `bool` indicating whether migration occurred.

**When to use:** Called from `read_metadata()` (or a thin wrapper) whenever metadata is accessed. Must be idempotent — running twice on the same recording must produce the same result.

**Example:**
```rust
/// Migrate legacy `tags` to zero-step pipeline labels on recording access.
/// Returns Ok(true) if migration was performed, Ok(false) if already migrated or no tags.
pub fn migrate_tags_to_pipeline_labels(metadata: &mut RecordingMetadata) -> Result<bool, String> {
    // Guard: no tags or already has pipeline states for all tags
    if metadata.tags.is_empty() {
        return Ok(false);
    }

    // Check if all tags already have a corresponding pipeline state
    let existing_names: std::collections::HashSet<&str> =
        metadata.pipelines.iter().map(|s| s.name.as_str()).collect();
    let unmigrated_tags: Vec<&String> = metadata.tags.iter()
        .filter(|t| !existing_names.contains(t.as_str()))
        .collect();

    if unmigrated_tags.is_empty() {
        return Ok(false); // Already migrated
    }

    // Ensure each tag has a corresponding zero-step pipeline in pipelines.json
    let mut pipelines = crate::pipelines::load_pipelines()?;
    for tag in &unmigrated_tags {
        if !pipelines.contains_key(*tag) {
            pipelines.insert((*tag).clone(), crate::pipelines::Pipeline {
                name: (*tag).clone(),
                description: format!("Label (migrated from tag '{}')", tag),
                steps: vec![], // Zero steps = label only
            });
        }
    }
    crate::pipelines::save_pipelines_to_disk(&pipelines)?;

    // Add Waiting pipeline states for unmigrated tags
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    for tag in unmigrated_tags {
        metadata.pipelines.push(crate::pipeline_engine::PipelineState {
            name: tag.clone(),
            status: crate::pipeline_engine::PipelineStatus::Done, // Label = already "done" (no steps to run)
            started_at: Some(now.clone()),
            completed_at: Some(now.clone()),
            current_step: None,
            error: None,
        });
    }

    // Write updated metadata back to disk
    write_metadata(metadata)?;

    Ok(true)
}
```

**Status for migrated labels:** Use `PipelineStatus::Done` (not `Waiting`) for tag-migrated labels. A label has no steps to execute, so marking it as `Done` immediately avoids confusing the user with a "Waiting" status for a label that will never execute.

### Pattern 3: Zero-Step Pipeline Validation Relaxation

**What:** Remove the `steps.is_empty()` check in `validate_pipeline()`. Also add a special case in `execute_pipeline_internal()`.

**Example (pipelines.rs):**
```rust
// REMOVE this block entirely:
// if pipeline.steps.is_empty() {
//     return Err("Pipeline must have at least one step".to_string());
// }
```

**Example (pipeline_engine.rs):**
```rust
pub async fn execute_pipeline_internal(
    recording_id: &str,
    pipeline_name: &str,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<PipelineStatus, String> {
    let pipelines = load_pipelines()?;
    let pipeline = pipelines.get(pipeline_name)
        .ok_or_else(|| format!("Pipeline '{}' not found", pipeline_name))?
        .clone();

    validate_pipeline(&pipeline)?;

    // PIPE-01: Zero-step pipeline = label only; skip execution, return Done immediately
    if pipeline.steps.is_empty() {
        update_pipeline_state(recording_id, pipeline_name, PipelineStatus::Done, None, None)?;
        return Ok(PipelineStatus::Done);
    }

    // ... rest of execution ...
}
```

### Pattern 4: Frontend Zero-Step Guard Removal

**What:** Remove the alert guard in `pipeline-builder.js` that prevents saving a pipeline with zero steps.

**Example (pipeline-builder.js line 710):**
```javascript
// REMOVE this line:
// if (pipelineEditorSteps.length === 0) { alert('Pipeline must have at least one step'); return; }
```

After removal, the user can save a pipeline with zero steps, which functions as a label. The UI chip bar already renders all pipelines; the step count display (`p.steps.length step(s)`) at line 357 already handles 0 gracefully ("0 steps").

### Anti-Patterns to Avoid

- **Do not batch-migrate all recordings on startup.** Lazy migration (on access) is the explicit design decision. Batch would block startup and risk data loss on crash during migration.
- **Do not delete the `tags` field from `RecordingMetadata`.** Keep it for backward compat. Downgraded users would lose tag data if the field was removed from the struct.
- **Do not create circular imports between `storage.rs` and `pipeline_engine.rs`.** Move `PipelineState` to a shared location before adding it to `RecordingMetadata`.
- **Do not use `Waiting` status for migrated tag labels.** Use `Done` — a label has no steps to execute, so `Waiting` would create a false impression that something needs to run.
- **Do not run `save_pipelines_to_disk` for each tag separately.** Load once, insert all new zero-step pipelines, save once. This avoids race conditions and is more efficient.
- **Do not skip the flock-based locking when writing migrated metadata.** The `update_pipeline_state` function already uses `libc::flock` for concurrent safety. The migration function should either call `write_metadata()` (which uses the atomic temp-file pattern) after acquiring the lock, or use a similar mutex approach.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Tag-to-pipeline name conflict detection | Custom dedup logic | `HashMap::contains_key()` check before inserting | Pipelines are stored as `HashMap<String, Pipeline>` — existence check is O(1) and idempotent |
| File lock for migration | Manual `flock` syscall in migration function | Call existing `write_metadata()` which does atomic temp+rename | `write_metadata()` already implements the safe pattern; no need to duplicate |
| Zero-step detection in UI | JS count check | Remove the guard and rely on Rust `validate_pipeline()` | Rust is the authoritative validator; JS guard is defensive but now incorrect |

**Key insight:** The pipeline state persistence mechanism already works via `pipeline_engine.rs`. Phase 7 is mostly removing constraints (zero-step guard, missing `pipelines` struct field) and adding the lazy migration bridge between old `tags` and new `pipelines`.

---

## Common Pitfalls

### Pitfall 1: Circular Module Dependency

**What goes wrong:** Adding `pipelines: Vec<PipelineState>` to `RecordingMetadata` creates a compile error because `PipelineState` is in `pipeline_engine.rs`, which already imports `storage.rs`.

**Why it happens:** Rust does not allow circular module dependencies. `pipeline_engine.rs` uses `use crate::storage::get_data_dir`. Adding the reverse import creates a compile cycle.

**How to avoid:** Before adding the `pipelines` field, move `PipelineState`, `PipelineStatus`, `StepStatus`, and `PipelineProgressPayload` from `pipeline_engine.rs` to `pipelines.rs` (the shared types module). Both `storage.rs` and `pipeline_engine.rs` can then import from `pipelines.rs` without a cycle. Update `pipeline_engine.rs` to remove the now-duplicated type definitions and add `use crate::pipelines::{PipelineState, PipelineStatus, ...}`.

**Warning signs:** `cargo check` error: `cycle detected when computing type of ...`

### Pitfall 2: Migration Runs Every Time on Recordings with Tags but No Pipelines

**What goes wrong:** The lazy migration check is too eager — it runs the migration logic (and disk I/O) every time a recording with tags is accessed, even after it has already been migrated.

**Why it happens:** The guard condition `if metadata.tags.is_empty()` does not detect "already migrated" — it only detects "no tags". If a recording has tags AND already has pipeline states for those tags, the migration should be skipped.

**How to avoid:** Use the set-based check: collect existing pipeline names from `metadata.pipelines`, compare against `metadata.tags`. Only proceed if any tag is not yet represented in `pipelines`. This is the `unmigrated_tags` pattern shown in the code example above.

**Warning signs:** Repeated writes to `pipelines.json` and `metadata.json` on every `list_recordings()` call, visible as file modification timestamps changing on each app open.

### Pitfall 3: Tag Names That Are Invalid Pipeline Names

**What goes wrong:** Old tags may contain characters that are invalid pipeline names (e.g., `/`, `:`, `\`, null bytes). `validate_pipeline()` enforces `pipeline.name` must not contain these characters. Creating a pipeline from such a tag would fail.

**Why it happens:** Tags were stored as arbitrary strings; pipeline names have a filesystem-safety constraint (because pipeline output dirs use the name as a directory component).

**How to avoid:** In `migrate_tags_to_pipeline_labels()`, sanitize tag strings before using them as pipeline names. Replace `/`, `:`, `\`, null bytes with `-` (or another safe char). If the sanitized name conflicts with an existing pipeline name, append a suffix. Log the sanitization so it is traceable.

**Warning signs:** Migration returns an error for recordings with tags like `project/feature`, `work:client`, etc.

### Pitfall 4: `assign_pipeline` Fails for Zero-Step Pipelines

**What goes wrong:** `assign_pipeline()` in `pipeline_engine.rs` calls `load_pipelines()` and checks `pipelines.contains_key(&pipeline_name)`. If the zero-step pipeline was created in `pipelines.json` by the migration, this should work. But if the migration writes to `pipelines.json` without properly refreshing the in-memory state, `assign_pipeline` may fail to find the new pipeline.

**Why it happens:** `load_pipelines()` reads fresh from disk each time (no in-memory cache), so this should be fine. But if the migration and `assign_pipeline` run concurrently, there could be a TOCTOU race.

**How to avoid:** The migration must complete (including `save_pipelines_to_disk`) before the pipeline states are written to metadata. Since the migration is called inside `read_metadata` (which is synchronous), and `assign_pipeline` can only be called later (after the UI receives the migrated metadata), sequential ordering is guaranteed in practice.

**Warning signs:** `assign_pipeline` returning "Pipeline not found" for a tag-name that should have been created by migration.

### Pitfall 5: `write_metadata()` Overwrites Pipeline States Written by `pipeline_engine.rs`

**What goes wrong:** After Phase 7 adds `pipelines: Vec<PipelineState>` to `RecordingMetadata`, calling `write_metadata()` will serialize the entire struct including `pipelines`. But `pipeline_engine.rs::update_pipeline_state()` currently does a raw `serde_json::Value` patch — it reads the raw JSON, modifies `json["pipelines"]`, and writes back. If `storage.rs::write_metadata()` is called concurrently (e.g., title update), it will serialize the `RecordingMetadata` struct, which may have a stale in-memory `pipelines` field, overwriting the engine's pipeline states.

**Why it happens:** Two writers with different approaches to the same file: struct-based and raw-JSON-based.

**How to avoid:** After adding the `pipelines` field to `RecordingMetadata`, review all callers of `write_metadata()`:
- `update_tags()` — reads `read_metadata()` first, which now returns up-to-date `pipelines` from disk, so round-trip is safe as long as flock is acquired.
- `update_title()` — same.
- `create_recording()` — writes new metadata with empty `pipelines: vec![]` — correct.
- `migrate_tags_to_pipeline_labels()` — must call `write_metadata()` with the fully-updated struct.

The flock in `pipeline_engine.rs` protects against concurrent writes, but `write_metadata()` in `storage.rs` does NOT currently acquire the flock. This is a latent bug that Phase 7 must not worsen. The safest fix: have `update_tags()` and `update_title()` also acquire the `.metadata.lock` flock before their read-modify-write cycle, OR accept that title/tag updates are rare and the window is tiny. The phase brief says to retain tags for backward compat, so `update_tags` may become dead code post-migration.

### Pitfall 6: `list_recordings()` Performance Impact

**What goes wrong:** `list_recordings()` calls `read_metadata()` for every recording directory. If `read_metadata()` now triggers lazy migration (which writes to disk), `list_recordings()` becomes O(n) with O(n) writes on first run — potentially slow for users with many recordings.

**Why it happens:** Migration is triggered on access; `list_recordings()` is the main access path.

**How to avoid:** Accept this as a one-time cost. After migration, `unmigrated_tags` will be empty for all recordings, so the migration check is just a set comparison with no I/O. Consider adding a debug log on first migration of each recording for observability. Do not add a global "migration done" flag to settings (unlike the transcript migration did) — lazy per-recording is the chosen approach.

---

## Code Examples

Verified patterns from codebase analysis:

### Current RecordingMetadata (storage.rs)
```rust
// Source: /workspace/src-tauri/src/storage.rs lines 8-19
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RecordingMetadata {
    pub id: String,
    pub created_at: String,
    pub title: String,
    pub tags: Vec<String>,       // Existing — keep for backward compat
    #[serde(default = "default_status")]
    pub status: String,
    pub audio: AudioFiles,
    #[serde(default)]
    pub health: Option<RecordingHealth>,
    // NEW FIELD to add:
    // #[serde(default)]
    // pub pipelines: Vec<PipelineState>,
}
```

### Current PipelineState (pipeline_engine.rs — to be moved to pipelines.rs)
```rust
// Source: /workspace/src-tauri/src/pipeline_engine.rs lines 22-33
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PipelineState {
    pub name: String,
    pub status: PipelineStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_step: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
```

### Current Zero-Step Guard (to be removed from validate_pipeline)
```rust
// Source: /workspace/src-tauri/src/pipelines.rs lines 67-70
// REMOVE THIS:
if pipeline.steps.is_empty() {
    return Err("Pipeline must have at least one step".to_string());
}
```

### Current Zero-Step Guard (to be removed from pipeline-builder.js)
```javascript
// Source: /workspace/src/pipeline-builder.js line 710
// REMOVE THIS:
if (pipelineEditorSteps.length === 0) { alert('Pipeline must have at least one step'); return; }
```

### Pipeline Output Directory (already correct for PIPE-05)
```rust
// Source: /workspace/src-tauri/src/pipeline_engine.rs lines 56-61
fn get_pipeline_output_dir(recording_id: &str, pipeline_name: &str) -> PathBuf {
    get_data_dir()
        .join(recording_id)
        .join("pipelines")
        .join(pipeline_name)  // Already per-pipeline isolation!
}
```

### existing update_pipeline_state writes to JSON["pipelines"] raw
```rust
// Source: /workspace/src-tauri/src/pipeline_engine.rs lines 579-612
// Key behavior: reads raw JSON, patches json["pipelines"], writes back
// This is the pattern that will align with the typed field after Phase 7
let mut pipeline_states: Vec<PipelineState> = json
    .get("pipelines")
    .and_then(|v| serde_json::from_value::<Vec<PipelineState>>(v.clone()).ok())
    .unwrap_or_default();
// ... modify ...
json["pipelines"] = serde_json::to_value(&pipeline_states)...;
```

### Existing Transcript Migration Pattern (reference for migration structure)
```rust
// Source: /workspace/src-tauri/src/transcript_migration.rs
// Batch migration pattern — NOT what we're doing. Lazy is different.
// The key lesson: migration function returns Result<bool, String> (true = migrated)
// and is idempotent (checks for already-migrated before doing work)
fn migrate_transcript(path: &Path) -> Result<bool, String> {
    // Already has frontmatter - skip
    if content.starts_with("---") {
        return Ok(false);
    }
    // ... do migration ...
    Ok(true)
}
```

### Actual metadata.json shape (inferred from codebase — no live data in sandbox)
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "created_at": "2026-01-15T14:30:00Z",
  "title": "Team standup",
  "tags": ["work", "standup"],
  "status": "ready",
  "audio": {
    "mic": {"file": "mic.ogg", "duration_sec": 300.0, "sample_rate": 44100, "channels": 1},
    "system": null,
    "mix": {"file": "mix.ogg", "duration_sec": 300.0, "sample_rate": 44100, "channels": 2}
  },
  "health": {"status": "ok", "issues": []},
  "pipelines": [
    {
      "name": "meeting-notes",
      "status": "done",
      "started_at": "2026-01-15T14:35:00Z",
      "completed_at": "2026-01-15T14:35:10Z"
    }
  ]
}
```

Note: The `"pipelines"` array is already written by `pipeline_engine.rs::update_pipeline_state()`. The `RecordingMetadata` struct just doesn't have the typed field yet — Phase 7 formalizes this.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tags as raw `Vec<String>` in metadata | Pipeline labels as zero-step `Pipeline` entries | Phase 7 | Unified mental model: everything is a pipeline, including labels |
| `pipelines` field written via raw JSON patching | `pipelines: Vec<PipelineState>` typed field in `RecordingMetadata` | Phase 7 | Type safety, compile-time correctness, simpler read paths |
| `validate_pipeline` rejects zero steps | Zero steps allowed (label-only) | Phase 7 | Pipelines can represent both labels and execution plans |
| Batch startup migration (transcript_migration pattern) | Lazy per-recording migration (on access) | Phase 7 | No startup delay, no all-or-nothing risk |

**Deprecated/outdated:**
- `update_tags` Tauri command: still registered in `lib.rs`, but after Phase 7 the conceptual model changes. Tags are still serialized to disk (backward compat) but will be shadowed by pipeline labels. The command itself does not need to be removed, but it becomes legacy.

---

## Open Questions

1. **Where to move PipelineState types to avoid circular dependency?**
   - What we know: `PipelineState` is in `pipeline_engine.rs`; `RecordingMetadata` is in `storage.rs`; `pipeline_engine.rs` imports `storage.rs`.
   - What's unclear: Should types move to `pipelines.rs` (existing shared module) or a new `pipeline_types.rs`?
   - Recommendation: Move to `pipelines.rs` — it already exports `Pipeline`, `PipelineStep`, `ConnectorType`. Adding `PipelineState`, `PipelineStatus`, `StepStatus`, `PipelineProgressPayload` is a natural grouping. `pipeline_engine.rs` becomes purely behavioral.

2. **Tag name sanitization: what characters to replace?**
   - What we know: Tags are arbitrary strings; pipeline names cannot contain `/`, `\`, `:`, null bytes.
   - What's unclear: Should other special characters (spaces, dots, etc.) also be replaced for filesystem safety? The existing check only rejects those four chars.
   - Recommendation: Only replace the four prohibited chars (replace with `-`). Don't over-sanitize — tags like "work.notes" or "team meeting" should become "work.notes" and "team meeting" pipeline names (spaces are allowed in pipeline names currently, since the check only excludes `/`, `\`, `:`, null).

3. **Should migrated label pipelines use `Done` or a new `Label` status?**
   - What we know: Current `PipelineStatus` enum has `Waiting`, `Running`, `Done`, `Partial`. No "Label" status.
   - What's unclear: Does using `Done` for a label confuse the UI (which shows "Done" with a green badge)?
   - Recommendation: Use `Done` — a label is "done" by definition (nothing to execute). Alternatively, add a `#[serde(rename = "label")]` variant `Label` to `PipelineStatus` for semantic clarity, but this adds enum complexity. Simpler to use `Done`. The UI already shows `Done` as green/success. A pipeline label appearing as "Done" in the status section is semantically correct.

4. **Does `assign_pipeline` need to be called for migrated labels?**
   - What we know: `assign_pipeline()` just calls `update_pipeline_state(..., PipelineStatus::Waiting, ...)`. The migration writes pipeline states directly.
   - What's unclear: Whether the migration should use `assign_pipeline()` or directly manipulate the `pipelines` field.
   - Recommendation: Do NOT call `assign_pipeline()` from the migration — it validates that the pipeline exists in `pipelines.json` and sets `Waiting` status. The migration should write the label pipelines to `pipelines.json` first, then directly write `Done` pipeline states to the recording metadata via `write_metadata()`. This avoids a second disk round-trip.

5. **Should filtering/listing by pipeline label be implemented in Phase 7?**
   - What we know: PIPE-01 says "label pipeline functions as a label in all listing and filtering views." The success criteria mentions "listing and filtering views."
   - What's unclear: Is there a filtering UI for pipelines/labels? Looking at the current codebase, there is no tag-based filtering UI in main.js — the `tags` field existed but there was no active filtering by tags.
   - Recommendation: The listing view already shows all recordings. For "filtering" to work, the UI would need a way to filter recordings by pipeline label. This is not currently in the UI and may need to be added in Phase 7 or deferred. **The planner should explicitly decide whether filtering UI is in scope for Phase 7 or deferred.** Based on the roadmap, no explicit filtering UI plan is mentioned for Phase 7, so this may be implied as "show pipeline labels on recordings" rather than "filter by label."

---

## Sources

### Primary (HIGH confidence)
- Direct codebase reading: `/workspace/src-tauri/src/storage.rs` — `RecordingMetadata` struct, `write_metadata()`, `read_metadata()`, `list_recordings()`, `update_tags()`, `create_recording()`
- Direct codebase reading: `/workspace/src-tauri/src/pipelines.rs` — `Pipeline`, `PipelineStep`, `validate_pipeline()`, `load_pipelines()`, `save_pipelines_to_disk()`, zero-step guard at line 68-70
- Direct codebase reading: `/workspace/src-tauri/src/pipeline_engine.rs` — `PipelineState`, `PipelineStatus`, `update_pipeline_state()` raw JSON patching pattern, `read_pipeline_states()`, `execute_pipeline_internal()`, `assign_pipeline()`, `get_pipeline_output_dir()`
- Direct codebase reading: `/workspace/src-tauri/src/transcript_migration.rs` — lazy vs batch migration patterns, `migrate_transcript()` idempotency pattern
- Direct codebase reading: `/workspace/src-tauri/src/lib.rs` — registered Tauri commands, module dependencies
- Direct codebase reading: `/workspace/src/pipeline-builder.js` — zero-step frontend guard at line 710, `save_pipeline` invocation
- Direct codebase reading: `/workspace/.planning/REQUIREMENTS.md` — PIPE-01 through PIPE-05 definitions
- Direct codebase reading: `/workspace/.planning/ROADMAP.md` — Phase 7 plan outlines (07-01, 07-02)

### Secondary (MEDIUM confidence)
- Pattern inference: Circular dependency resolution by moving shared types to `pipelines.rs` — inferred from Rust module system rules and existing import graph

### Tertiary (LOW confidence)
- None — all claims verified from codebase

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new libraries; all patterns directly in codebase
- Architecture: HIGH — data model, migration pattern, zero-step support all verified from source
- Pitfalls: HIGH — circular dependency is a compile-time certainty; migration idempotency checked against transcript_migration.rs pattern; write-conflict risk documented from pipeline_engine.rs read
- Open questions: MEDIUM — filtering UI scope is unclear from roadmap; PipelineStatus enum naming is a design choice

**Research date:** 2026-02-19
**Valid until:** 2026-03-19 (stable codebase; no external dependencies)
