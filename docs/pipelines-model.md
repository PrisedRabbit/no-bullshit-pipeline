# Pipelines — model

**What a pipeline is.** A named, ordered chain of self-contained steps that
runs against a recording's transcript (or a dictation's text). There is no
separate "Connection" registry and no networked delivery connectors — that
complexity was removed after talking to real users. The only built-in
destination is **Save to folder**; for anything networked (Notion, Slack, an
HTTP endpoint) write a **Shell** step that does it (`curl`, the vendor CLI, …).

```
Transcript ─▶ Step 1 ─▶ Step 2 ─▶ … ─▶ Step N
              (cli|shell|save_local)
```

## Step

Each step is fully self-contained — it carries its own type, inline config,
and template. No shared/reusable objects, no credential indirection.

```rust
PipelineStep {
  name,                  // unique within the pipeline; filesystem-safe
  step_type: StepType,   // CliAgent | Shell   (serde alias: connection_type)
  template: String,      // CLI: the prompt · Shell: the bash script body
  config: serde_json::Value,  // inline, non-secret, per-type (see below)
  description: Option<String>,
}
```

### `cli_agent`

Runs a locally-installed coding CLI (Claude Code / Codex / OpenCode / agy).

- `config`: `{ cli, model?, timeout_secs?, working_directory? }`
- `template`: the prompt. The engine substitutes placeholders (below) before
  the agent sees it, then injects the result as `config.prompt`.

### `shell`

Script-mode: the step IS a bash script. The script is **not** placeholder-
substituted (avoids shell-injection from transcript content) — raw values
arrive via env vars instead.

- `config`: `{ cwd (required), shell?=/bin/bash, env?={K:V}, timeout_secs?=120 }`
- `template`: the script body. Stdout becomes the step's output.
- env: `NBP_TRANSCRIPT`, `NBP_PROCESSING_RESULT` (previous step output, empty
  on step 1), `NBP_APP` (friendly app name).

### `save_local`

Write the step's content to a local folder — the one built-in destination.

- `config`: `{ folder_path (required) }` (`~` expanded).
- `template`: encodes WHAT to save — `{processing_result}` (default) or
  `{transcript}`, picked via a radio in the editor. The engine renders it, so
  the connector just writes the resulting string.
- File lands at `<folder>/<date>-<pipeline>.md` (collision-suffixed). The
  step's chained output is `Saved to <path>` (save is normally terminal).

## I/O contract

Placeholders for CLI templates (missing keys render as empty string):

| Placeholder | Value |
|---|---|
| `{transcript}` | full recording transcript |
| `{processing_result}` | previous step's output (empty on step 1) |
| `{app}` | friendly app name (Zoom / FaceTime / NBP / …) |

The chain is **strictly linear**: step _i_ eats the transcript (step 1) or
step _i-1_'s output. Every step writes its output as a `<step>.md` artifact
into the recording's `pipelines/<pipeline>/` folder — so **save-to-folder is
the base behaviour**, not a separate step.

## Failure semantics

A failed step halts everything downstream (later steps can't run without the
prior step's output) and marks the run `Partial`. A run with no failures is
`Done`. A zero-step pipeline is a valid label that returns `Done` immediately.

## Dictation

Quick Dictate reuses pipelines for in-memory text transforms: `cli_agent`
steps chain (each output replaces the running text); `shell` steps are skipped
(the paste-only flow has no output dir / recording context).

## Legacy migration

Pre-simplification pipelines referenced removed networked-delivery types
(notion/slack/telegram/webhook) and a `connection_id` field. `load_pipelines`
parses loosely and drops any step whose type isn't `cli_agent`/`shell`/
`save_local`; `connection_id` is ignored. Surviving steps are kept (a legacy
`save_local` step loses its folder — it lived on the Connection — so the user
re-picks it); a pipeline that still won't parse is skipped (logged), never
nuking the whole file. There is no Connections tab, no Keychain for connectors.
