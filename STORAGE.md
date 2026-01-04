# Storage Architecture

## On-Disk Layout (v0.1.0)

Every recording session is a dedicated directory named with a **UUID v4**. This ensures no collisions and clean logical separation.

```
~/nbp-data/
├── 550e8400-e29b-41d4-a716-446655440000/
│   ├── raw_mic.ogg          # Normalized Microphone (Mono/Stereo)
│   ├── raw_system.ogg       # Normalized System Audio (Stereo Loopback)
│   ├── audio_mix.ogg        # The combined "Master" Mix (ready for playback)
│   ├── metadata.json        # Source of truth for session data
│   ├── transcript.md        # (v0.2) Derived transcription
│   └── structured.json      # (v0.2) Derived AI intelligence
├── metadata.projects.json   # (Optional) Global project/filter definitions
...
```

## Invariants (Do Not Break)

1. **One recording = one directory**: Locality is king.
2. **Sources are Immutable**: Once `raw_mic.ogg` and `raw_system.ogg` are written and the recording stops, they are never modified.
3. **Derived Content**: `audio_mix.ogg`, `transcript.md`, and `structured.json` are derived artifacts. They can be deleted and regenerated from the raw sources.
4. **`metadata.json` is the Authority**: No external database. If the directory exists, it must contain a valid metadata file.
5. **Universal Formats**: We use `.ogg` (Vorbis), `.json`, and `.md`. Your data is usable on any machine without NBP installed.

## metadata.json Schema

The metadata file captures technical specifics and user-defined context.

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "created_at": "2026-01-04T12:00:00Z",
  "title": "Strategy Session",
  "tags": ["work", "strategy", "q1"],
  "audio": {
    "mic": {
      "file": "raw_mic.ogg",
      "duration_sec": 305.5,
      "sample_rate": 48000,
      "channels": 1
    },
    "system": {
      "file": "raw_system.ogg",
      "duration_sec": 305.5,
      "sample_rate": 48000,
      "channels": 2
    },
    "mix": {
      "file": "audio_mix.ogg",
      "duration_sec": 305.5,
      "sample_rate": 48000,
      "channels": 2
    }
  }
}
```

### Field Definitions

| Field          | Type          | Description                                    |
| :------------- | :------------ | :--------------------------------------------- |
| `id`           | UUID          | Must match the directory name.                 |
| `created_at`   | ISO 8601      | UTC timestamp of recording start.              |
| `title`        | String        | User-defined title (defaults to Recording #N). |
| `tags`         | Array<String> | Flat list of labels.                           |
| `audio.mic`    | Object        | Metadata for the microphone track.             |
| `audio.system` | Object        | Metadata for the system audio loopback.        |
| `audio.mix`    | Object        | Metadata for the combined track.               |

## Normalization & Loudness

All audio captures are normalized to **EBU R128 (-23 LUFS)** standard before or during the encoding process. This ensures that when you playback the mix or separate tracks, the volume levels are consistent and professional.

## Performance & Indexing

On startup, NBP performs a "Parallel Scan":

1. Reads all directories in `~/nbp-data/`.
2. Rapidly parses `metadata.json` files in parallel.
3. Builds an in-memory index for instant filtering and sorting.
4. **Target**: <50ms for 500 recordings.

## Philosophy

- **Files over databases**: Grep, rsync, and Backup software just work.
- **Privacy by location**: You know exactly where your data is.
- **Portability**: Move a directory to another machine, and NBP will pick it up instantly.
- **Transparency**: No hidden files or binary blobs.

---

**Rule Zero**: If it breaks ownership, locality, or debuggability — don’t do it.
