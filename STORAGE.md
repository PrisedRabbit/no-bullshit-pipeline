# Storage Architecture

## On-Disk Layout (v0.1)

```
~/nbp-data/
├── 550e8400-e29b-41d4-a716-446655440000/
│   ├── raw.wav              # immutable raw audio
│   ├── metadata.json        # source of truth
│   ├── transcript.md        # derived, re-creatable
│   └── structured.json      # derived, re-creatable
├── 7c9e6679-7425-40de-944b-e07fc1f90ae7/
│   ├── raw.wav
│   ├── metadata.json
│   ├── transcript.md
│   └── structured.json
...
```

## Invariants (Do Not Break)

1. **One recording = one directory**
2. **`raw.*` is immutable** — never modified after recording stops
3. **All other files are derived** — can be regenerated from `raw.*`
4. **`metadata.json` is the only source of truth** for session data
5. **App can be deleted; data remains usable** — no proprietary formats

## Naming Rules

- **Directory name = UUID v4** (e.g., `550e8400-e29b-41d4-a716-446655440000`)
- **Machine logic MUST NOT rely on directory names**
- **File formats are explicit** (`.wav`, `.ogg`, `.md`, `.json`)
- **Human-readable content is in `metadata.json`** (`title`, `tags`)

## metadata.json Schema

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "created_at": "2026-01-03T22:15:30Z",
  "title": "project planning",
  "tags": ["project", "planning"],
  "audio": {
    "file": "raw.wav",
    "duration_sec": 152.34
  }
}
```

### Field Definitions

| Field                | Type              | Description                                        |
| -------------------- | ----------------- | -------------------------------------------------- |
| `id`                 | string (UUID v4)  | Stable, unique identifier matching directory name  |
| `created_at`         | string (ISO 8601) | UTC timestamp, used for sorting                    |
| `title`              | string            | Human-readable name (optional, can be empty)       |
| `tags`               | array of strings  | User-defined tags for filtering/grouping           |
| `audio.file`         | string            | Filename of raw audio (e.g., `raw.wav`, `raw.ogg`) |
| `audio.duration_sec` | number            | Recording duration in seconds (supports decimals)  |

## Sorting & Filtering

### Sorting

- **Primary:** `created_at` (descending = newest first)
- **Never** by directory name or filesystem metadata

### Filtering

- **By tags:** Parse all `metadata.json` files, filter by `tags` array
- **By date range:** Filter by `created_at`
- **By title:** Full-text search on `title` field

## In-Memory Index (Performance)

On app startup:

1. Scan `~/nbp-data/` for all UUID directories
2. Parse each `metadata.json`
3. Build in-memory index:
   ```rust
   struct RecordingIndex {
       by_id: HashMap<String, RecordingMetadata>, // metadata only, no audio blobs
       by_tag: HashMap<String, Vec<String>>, // tag -> [id, id, ...]
       sorted_by_date: Vec<String>, // [id, id, ...] newest first
   }
   ```
4. Rebuild index on:
   - New recording created
   - Metadata edited
   - App restart

**Performance target:** Parse 200 recordings in <50ms.

## Philosophy (Compressed)

- **Files over databases** — grep, jq, rsync work out of the box
- **Raw over processed** — `raw.*` is immutable truth
- **Ownership over convenience** — user owns files, app is optional
- **Locality** — all assets for one recording in one directory

## Rule Zero

**If it breaks ownership, locality, or debuggability — don't do it.**
