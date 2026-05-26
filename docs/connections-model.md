# Connections / Destinations / Pipelines — architecture

**Purpose.** Lock the model BEFORE writing code so we do not iterate in the
dark. This doc is the closed-decision reference for the
`connections-pipelines` branch.

---

## Mental model

```
Connection = self-contained entry (type + own auth + own target)
                                                ↑
                                                │ pipeline step picks one
                                                │
                        Pipeline = [ Step → Step → Step ]
```

Flat single-level model. Every Connection is **fully self-contained**: its
own auth (where the type needs it) AND its own target. There is no shared
"credentials parent" / two-level grouping.

- **Connection** — a named entry with a *type* (Slack, Notion, Shell,
  Claude-CLI, etc.) and a complete config (auth + target). Two separate
  Notion workspaces with different API keys = **two separate entries**. Two
  Telegram bots = two entries. Two Slack workspaces = two OAuth flows = two
  entries.
- Adding a Connection always asks for the full config in one form: type +
  auth (if the type needs it) + target. No nested setup.
- **Pipeline step** picks a single Connection (list filtered by type) plus a
  per-step template. The step never re-enters credentials — it composes
  pre-built bricks.

### Two roles for Connections

| Role | Behaviour | Failure semantics |
|---|---|---|
| **Processing** | output feeds next step | step fails → downstream halts |
| **Delivery** | terminal, sends output to external | step fails → chain continues |

Visually grouped in the Connections tab (Processing section / Delivery
section) so the user immediately sees what each capability does.

---

## In scope

Each line describes a single Connection entry — its full self-contained
config. Add another entry of the same type to get another instance with
different auth / target.

**Processing**
- **CLI agent** (Claude Code, Codex, Gemini CLI, …) — entry: which agent
  binary + preset (model, flags, default system prompt).
- **Shell script** — entry: named script (command + CWD + default args). No
  auth.

**Delivery**
- **Slack** — entry: OAuth token (one workspace) + chosen channel or DM user.
  Two workspaces = two entries with their own tokens.
- **Notion** — entry: API key (or OAuth) + specific database or parent page.
  Two workspaces with different API keys = two entries.
- **Telegram** — entry: bot token + chat_id. Two bots = two entries.
- **Web URL (webhook)** — entry: named URL endpoint (+ method/headers if
  needed). No auth at the model layer (auth, if any, is inside the URL or
  custom headers).
- **Save Local** — entry: named folder preset. No auth.

---

## Out of scope today (code stays in repo, UI hidden)

These have working connectors but no demand in the current workflow. Re-surface
when a concrete need appears — re-wiring is < 1 hour each since the code is
intact.

- **Linear** (delivery).
- **MCP** (processing — Model Context Protocol tool-calls).
- **Direct LLM API** (`ConnectorType::Llm` — direct OpenAI/Google/Anthropic
  calls with stored api_key). Replaced by CLI agents.

---

## Pipeline step — what it stores

```
Step {
    type: ConnectionType,         // determines which Connections are eligible
    connection_id: String,        // chosen entry from the Connections list
    template: String,             // text with placeholders (see I/O contract)
}
```

Step editor flow:
1. Pick **type** (Processing: Claude/Codex/Shell; Delivery: Slack/Notion/…).
2. Pick a **Connection** from the list filtered by that type. If only one of
   that type exists, pre-select it (zero clicks for the typical setup).
3. Write the **template** using the placeholder vocabulary below.

No inline creation in v1 — pre-built Connections only. (Future: "+ New" button
in the selector opens the Connections wizard inline; deferred.)

---

## Processing I/O contract

### Placeholders (template substitution)

Exactly three. Curly-brace `{key}` substitution. Missing key → empty string
(silent). Escape `{{` / `}}` if literal braces ever needed (defer until first
real complaint).

| Placeholder | Value | Notes |
|---|---|---|
| `{transcript}` | raw recording transcript | immutable, available at any step |
| `{app}` | friendly app name | "Zoom", "FaceTime", "NBP" for manual, etc. |
| `{processing_result}` | output of the **immediately previous** processing step | empty string for the first step |

**First step explicitly uses `{transcript}`** in its template.
`{processing_result}` is empty for the first step — no magical fallback.

### Output

- `stdout` (CLI / Shell) or API response (Direct LLM, if ever re-enabled) =
  string, becomes the next step's `{processing_result}`.
- Non-zero exit / API error → step fails → downstream processing halts.
- `stderr` → log only, not propagated as data.
- Empty output → empty string into next step. If the user wants pass-through,
  they pipe `cat` or equivalent — semantics stay clean.

### Future (NOT in v1 — only when a real pain shows up)

- `{steps.<step_name>}` — non-linear backref to a specific earlier step's
  output. Named by step (stable across reorder), not numbered (fragile).
- More context fields (`{title}`, `{date}`, `{duration}`, …) — by demand only;
  do not bloat the vocabulary speculatively.

---

## Resolved design decisions (do not re-open)

1. **No inline creation** of Connections from the pipeline step in v1. Only
   pre-built entries. Deferred: a "+ New" affordance in the selector.
2. **Flat model — each Connection is fully self-contained** (own auth + own
   target). Multiple Notion / Slack / Telegram entries with different keys
   are first-class. No two-level "connection → destinations" abstraction.
3. **Pre-select when only one entry of a type exists** — typical "single
   Slack workspace" / "single Notion DB" flows are one click. With multiple
   entries the user picks explicitly. No separate "default" flag — list +
   count is the signal.
4. **Connections tab merges** today's separate `Models` + `Integrations` into
   one tab with two visual sections (Processing / Delivery).
5. **Telegram** — new Connection type (bot token + chat_id as one entry).
6. **Per-step template/format** lives **at the step level** — the Connection
   answers "which configured target", the template answers "what and how".
7. **No backward compatibility for existing pipelines.** The ~3 users on
   board will recreate their pipelines manually after the model lands. Saves
   a non-trivial migration layer; freedom to design the new shape cleanly
   without compromises for legacy field mapping. On settings load with old
   shape: empty the pipelines list (logged, not silent) — the user is
   expected to re-add via the new editor.

---

## Closed during review (open questions the doc previously ducked)

These were raised by the ensemble panel review of this doc + current code.
Each is locked here so we do not re-litigate during implementation.

1. **Inter-step transport = in-memory String.** Runner holds a
   `prev_output: String` and substitutes `{processing_result}` directly. No
   on-disk handoff between steps. The doc's wording ("stdout → next step's
   `{processing_result}`") is now literally true.
2. **Per-step output files stay — as UI ARTIFACTS, not transport.** After
   each step the engine writes its output to a per-step file in the recording
   folder so the pipeline status panel + debugging keep working. Files are an
   **output** of the run, not the glue between steps.
3. **Linear chain only in v1.** The `step.input` field disappears entirely.
   Every Processing step reads from the immediately previous Processing
   step's output. No fan-out / fan-in / arbitrary back-references. The
   `{steps.<step_name>}` mechanism for non-linear access is **deferred** — add
   only when a concrete pain shows up.
4. **Connections storage shape = `connections: Vec<Connection>` in
   `AppSettings`.** Flat list (NOT keyed by type — keeps insertion order
   stable for UI). Each entry: `{ id, name, type, role, config:
   serde_json::Value, created_at }`. Per-type grouping is a UI concern, not a
   storage concern.
5. **Secrets stay in the existing Keychain helper** (`integrations/mod.rs`),
   keyed by `{type}:{connection_id}` (e.g. `slack:abc123`, `telegram:def456`).
   Tokens are **referenced** by Connection id, not duplicated per entry. No
   bloat.
6. **`{app}` resolver lives in Rust.** Today the friendly app name
   (`Zoom` / `FaceTime` / `NBP`) is only computed in JS for the recording row
   icon. Add a Rust-side `bundle_id → friendly_name` resolver (mirror the JS
   logic) so the pipeline runner can substitute `{app}` without round-tripping
   to the frontend.
7. **Zero-Connection step UI: show, disable, link.** If the user picks a step
   type with zero Connections of that type, **show the type** in the selector
   but block "Save Step" with the message *"Create a {type} Connection first"*
   and a link to the Connections tab. Hiding the type makes setup failure
   look like missing functionality.
8. **Connection deletion: warn if referenced.** When deleting a Connection,
   scan pipelines for `connection_id` references; if found, show warning +
   allow force-delete. Runtime: a step pointing at a missing Connection
   fails with a clear "Connection not found" error.
9. **Old pipelines: wipe on first load with new schema.** `load_pipelines`
   tries the new shape; on parse error (old shape detected), it logs and
   starts with empty list. Per decision #7 in the main resolved list — 3
   users rebuild manually.
10. **Quick Dictate pipelines.** Dictation already runs pipelines in-memory
    (`dictation.rs::run_text_pipeline`) — the new in-memory transport fits
    naturally. Shell is allowed in dictation pipelines (stdin/stdout works
    the same in-memory).
11. **Hidden `ConnectionType` variants kept in the Rust enum.** `Mcp`, `Llm`,
    and `Linear` stay as enum variants even though hidden from the new
    Connections UI. Keeps the serialization format forward-compatible so
    re-surfacing them later does not require a settings migration.

## Migration from today's `ConnectorType`

| Old | New role | New connection type | Notes |
|---|---|---|---|
| `CliAgent` | Processing | CLI agent (per binary) | entry config = model/flags/sysprompt preset |
| `Llm` | dropped from UI | — | Code in `connectors/llm.rs` stays |
| `Mcp` | dropped from UI | — | Code in `connectors/mcp.rs` stays |
| `Save` | Delivery | Save Local | entry config = folder preset |
| `Webhook` | Delivery | Web URL | entry config = named URL endpoint |
| `Slack` | Delivery | Slack | entry config = channel / DM user |
| `Notion` | Delivery | Notion | entry config = database / parent page |
| `Linear` | dropped from UI | — | Code in `connectors/linear.rs` stays |
| *(new)* `Shell` | Processing | Shell script | entry config = named script + CWD + default args |
| *(new)* `Telegram` | Delivery | Telegram | entry config = chat_id |

### Settings tab structure (after migration)

| Tab | Status |
|---|---|
| ASR | unchanged |
| Recording | unchanged |
| Dictation | unchanged |
| **Connections** | **NEW** — replaces `Models` + `Integrations`. Two sections: Processing / Delivery. |
| ~~Models~~ | removed |
| ~~Integrations~~ | removed |
| Prompts | unchanged |
| Pipelines | unchanged (but the step editor changes per Pipeline step model above) |
| Theme | unchanged |

---

## What this doc deliberately does NOT cover

- Concrete UI mockups (wait for design system pick — see TODO).
- DB schema for storing Connections + Destinations (TBD during implementation
  — likely a new section in `settings.json` keyed by connection type + a list
  of destinations per connection).
- ~~Backward compatibility for existing pipelines~~ — **explicitly dropped**
  (decision #7). Old pipelines are wiped on first load with the new model;
  the ~3 users rebuild manually via the new editor.
- Authentication / OAuth flows — already implemented per connector under
  `src-tauri/src/integrations/`, only the surfacing changes.
