---
stepsCompleted: [1, 2, 3]
inputDocuments: []
session_topic: 'NBP Pipelines - evolving flat tags into named pipelines with steps, connectors, and file-based context'
session_goals: 'Define pipeline model, connector types, data format, and execution semantics'
selected_approach: 'AI-Recommended Techniques'
techniques_used: ['morphological-analysis', 'role-playing', 'first-principles']
ideas_generated: []
context_file: ''
---

# Brainstorming Session Results

**Facilitator:** sk
**Date:** 2026-02-03

## Session Overview

**Topic:** NBP Pipelines — evolving flat tags into named pipelines with steps, connectors, and file-based context.

**Goals:** Define pipeline model, connector types, data format, execution semantics, and extensibility strategy.

### Context

NBP is a Tauri desktop app (Rust + Vanilla JS) for audio recording with transcription/AI processing. Tags are currently `Vec<String>` in metadata.json — no actions, no integrations. Template system exists for AI-based transcript processing. Cloud AI providers (OpenAI, Google, Anthropic) are already integrated.

---

## Key Decision: Tags → Pipelines

What was "tags" becomes **pipelines**. A pipeline is a named, ordered list of steps that process a recording's data. A recording can have multiple pipelines.

---

## Core Concepts

### Transcription is systemic, not a pipeline step

Transcription is a system-level feature of a recording, configured globally (provider, model). Result is `transcript.md` in the recording directory. Pipelines work **on top of** the transcript — they don't produce it.

### Pipeline waits for transcript

Pipeline status is `waiting` until transcript exists. Once transcript appears (auto, manual paste, external tool) — steps execute. Pipelines can be added to old recordings — if transcript exists, they run immediately.

### Steps are linear, no conditions

Steps execute in order. No branching, no conditionals. Each step reads from context and writes to context.

### Context is the filesystem

No in-memory state. Each step's output is a markdown file with frontmatter in the pipeline's directory. Steps read input files by name, write output files by name. Debug = open the folder.

### Every file is markdown with frontmatter

Transcript, step outputs, status — all `.md` files with YAML frontmatter for metadata and body for content.

---

## File Structure

```
~/nbp-data/{recording-id}/
  metadata.json
  audio_mix.ogg
  raw_mic.ogg
  raw_system.ogg
  transcript.md                    ← systemic, not part of any pipeline

  pipelines/
    hltm/
      meeting_notes.md             ← step output
      action_items.md              ← step output
      slack.md                     ← step output (delivery status)
    self/
      structured.md
      save.md
    семейный-совет/
      structured.md
      save.md
```

---

## Step File Format

Every step is a markdown file with frontmatter:

```markdown
---
name: meeting_notes
description: "Структурировать транскрипт в meeting notes"
connector: llm
input: transcript
status: done
created_at: 2026-02-03T12:00:00Z
completed_at: 2026-02-03T12:00:05Z
error: null
---

## Meeting Notes

1. Решили запускать в марте
2. СШ берёт фронт
...
```

### Delivery step example (Slack via MCP):

```markdown
---
name: slack
description: "Отправить meeting notes в канал #hltm"
connector: mcp
input: meeting_notes
status: done
created_at: 2026-02-03T12:00:06Z
completed_at: 2026-02-03T12:00:07Z
error: null
---

Sent meeting_notes.md to #hltm
Message ID: 1234567890
```

### Transcript file format:

```markdown
---
source: openai
model: whisper-1
created_at: 2026-02-03T11:55:00Z
duration_sec: 340
---

Сегодня обсудили с СШ планы на март...
```

---

## Input/Output Between Steps

Each step declares `input` — the name of a file to read from context:
- `input: transcript` → reads `transcript.md` from recording root
- `input: meeting_notes` → reads `meeting_notes.md` from current pipeline directory

Each step writes its output as a file named after the step's `name` field.

First step typically takes `transcript` as input. Subsequent steps can reference any previous step's output by name.

---

## Connectors

### Built-in (3 total):

| Connector | What it does | Input → Output |
|-----------|-------------|----------------|
| `llm` | Run through LLM with a prompt | md → md |
| `save` | Copy/save file to a specified path | md → md (status) |
| `webhook` | HTTP POST to a URL | md → md (status) |

### Everything else — MCP:

Any external integration uses the `mcp` connector with server + tool config:

```yaml
- connector: mcp
  config:
    server: "slack-mcp"
    tool: "send-message"
    args: { channel: "#hltm" }

- connector: mcp
  config:
    server: "notion-mcp"
    tool: "create-page"
    args: { database: "hltm-tasks" }

- connector: mcp
  config:
    server: "telegram-mcp"
    tool: "send-message"
    args: { chat_id: "..." }

- connector: mcp
  config:
    server: "gmail-mcp"
    tool: "send-email"
    args: { to: "boss@co.com" }
```

---

## Pipeline Definition

A pipeline is defined as a named config (stored TBD — global config file or per-project):

```yaml
pipeline: "hltm"
steps:
  - connector: llm
    name: meeting_notes
    description: "Структурировать транскрипт в meeting notes"
    input: transcript
    config:
      prompt: "структурируй meeting notes: участники, решения, action items"

  - connector: llm
    name: action_items
    description: "Вытащить action items с assignee"
    input: meeting_notes
    config:
      prompt: "вытащи action items с assignee в структурированном виде"

  - connector: mcp
    name: slack
    description: "Отправить meeting notes в #hltm"
    input: meeting_notes
    config:
      server: "slack-mcp"
      tool: "send-message"
      args: { channel: "#hltm" }

  - connector: mcp
    name: notion
    description: "Создать таски в Notion"
    input: action_items
    config:
      server: "notion-mcp"
      tool: "create-page"
      args: { database: "hltm-tasks" }
```

---

## Pipeline Statuses

| Status | Meaning |
|--------|---------|
| `waiting` | No transcript yet, steps not started |
| `running` | Steps executing |
| `done` | All steps completed successfully |
| `partial` | Some steps failed, some succeeded |

---

## Use Cases

### Team meeting
`#hltm` → llm(meeting notes) → llm(action items) → slack(#hltm) → notion(tasks)

### Personal voice notes
`#self` → llm(structure thoughts) → save(~/apex/self/)

### Family discussion
`#семейный-совет` → llm(решения и планы) → save(~/apex/family/{date}-семейный-совет)

### 1-on-1 with a person
`#сш` → llm(summary) → mcp(slack DM to сш)

### Raw recording, no processing
`#raw` → (no steps, just store)

---

## UX Principles

- User sees each pipeline on a recording with status of every step
- Failed steps can be re-run individually
- Pipelines can be added to existing recordings at any time
- Transcript can be added/edited manually at any time — pipelines react
- Pipeline outputs are editable (they're just md files)

---

## Open Questions

- Where to store pipeline definitions (global config file? separate files per pipeline?)
- Pipeline definition UI — how does user create/edit pipelines?
- How to handle multiple pipelines wanting different LLM providers for the `llm` connector
- Re-run semantics: when transcript is edited, auto re-run or manual?
- Pipeline templates / sharing / export
