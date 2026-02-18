---
stepsCompleted: [1, 2, 3]
inputDocuments:
  - _bmad-output/brainstorming/brainstorming-session-2026-02-03.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/ux-design.md
session_topic: 'NBP Pipelines v2 - mental model, assembly UX, pre-assignment, and schema-aware connectors'
session_goals: 'Clarify what a pipeline IS, how to build one easily, how to assign before recording, and how to handle structured destinations like Notion'
selected_approach: 'AI-Recommended Techniques'
techniques_used: ['first-principles', 'user-journey-mapping', 'progressive-disclosure', 'constraint-removal']
ideas_generated: []
context_file: ''
---

# Brainstorming Session Results

**Facilitator:** sk
**Date:** 2026-02-18
**Builds on:** brainstorming-session-2026-02-03.md

## Session Overview

**Topic:** NBP Pipelines v2 — mental model, assembly UX, pre-assignment, and schema-aware connectors.

**Goals:**
1. Define what a pipeline IS (mental model)
2. How to assign a pipeline with zero friction (before/during/after recording)
3. How to BUILD a pipeline easily (assembly UX)
4. How integrations (Slack, Notion, etc.) connect to pipeline steps

### Context

Previous brainstorm (2026-02-03) defined the pipeline data model, file structure, and connectors. This session focuses on UX: how users think about, build, and use pipelines. Code review of current pipeline builder revealed it's too developer-oriented (raw connector dropdowns, manual field entry, no presets).

---

## Part 1: What IS a Pipeline

### Core Definition

**Pipeline = intent + automation.**

When the user hits record, they already have context in their head: "this is an HLTM standup", "this is personal notes", "this is a family discussion." The pipeline is the **name of that context** with an optional processing chain attached.

The user doesn't think "assign a pipeline." They think **"this is HLTM."** The pipeline is a consequence of that thought.

### Tags Are Dead

Tags (`Vec<String>` in metadata.json) are replaced entirely by pipelines. A pipeline with zero steps IS a tag — it's an organizational label with no automation. A pipeline with steps is a label + automation. One concept instead of two.

| Scenario | Pipeline | Steps |
|----------|----------|-------|
| Just store, no processing | `#raw` | 0 steps — pure label |
| Personal voice notes | `#self` | 1-2 steps: llm → save |
| Team standup | `#hltm` | 4 steps: llm → llm → slack → notion |
| Family discussion | `#семейный-совет` | 2 steps: llm → save |
| 1-on-1 with a person | `#сш` | 2 steps: llm → slack DM |

### Multiple Pipelines Per Recording

One recording can have multiple pipelines — different "lenses" on the same content:
- `#hltm` + `#self` → team notes go to Slack, personal takeaways saved privately
- Each pipeline writes to its own directory, no conflicts

---

## Part 2: Pipeline Assignment UX

### The Three Moments

All three must work. Moment 1 is the most valuable.

### Moment 1: BEFORE Recording (highest value)

**Pipeline chips in the app bar, next to the record button.**

```
[HLTM] [Self] [Raw] [●REC]
```

- Click a chip → recording starts immediately WITH that pipeline pre-assigned
- The chip IS the record button for that pipeline
- Last-used pipeline is remembered and highlighted

This is the **zero post-recording work** flow: select pipeline → record → stop → auto-transcribe → auto-run pipeline → done. User never touches NBP again after stopping.

### Moment 2: DURING Recording

- Pipeline chips remain active during recording
- Can assign/change mid-recording
- Pipeline waits for transcript anyway, so mid-recording assignment is fine

### Moment 3: AFTER Recording

- Detail view: dropdown/chips for pipeline assignment
- Assign to old recordings: if transcript exists, runs immediately
- Can add multiple pipelines to one recording

### Default Pipeline

Settings option: "Default pipeline for new recordings." Most users have one primary use case (80% meetings, or 80% personal notes). Default = zero actions: open → record → stop → everything automatic.

### The Automation Chain

```
Pre-assign pipeline → Record → Stop → [everything below is automatic]
                                              ↓
                                        auto-transcribe
                                              ↓
                                        pipeline executes
                                              ↓
                                        meeting notes → Slack
                                        action items → Notion
                                              ↓
                                        done, user did nothing
```

---

## Part 3: Pipeline Assembly (Builder UX)

### Current Problem

The current pipeline builder is developer-oriented:
- "Connector" dropdown (user doesn't know what a connector is)
- Manual step name entry
- Provider + Model always visible (95% use the default)
- 6 fields per step — too many
- No presets — every step built from scratch

### Solution: Two Types of Steps

Forget "connectors." Users think in two categories:

| Category | What it does | Examples |
|----------|-------------|---------|
| **Processing** | AI takes text → produces different text | Meeting Notes, Action Items, Summary, Custom |
| **Delivery** | Takes result → sends it somewhere | Slack, Save to file, Webhook, Notion |

### The Step Picker

"+ Add Step" opens a picker, not a form:

```
┌─────────────────────────────────────────────┐
│  What to add?                               │
│                                             │
│  PROCESSING                                 │
│  ┌─────────────────────────────────────┐    │
│  │ Meeting Notes  — participants,      │    │
│  │                  decisions, tasks    │    │
│  │ Action Items   — tasks with assignee│    │
│  │ Summary        — short extract      │    │
│  │ Structure      — chaos → structure  │    │
│  │ Custom prompt  — write your own     │    │
│  └─────────────────────────────────────┘    │
│                                             │
│  DELIVERY                                   │
│  ┌─────────────────────────────────────┐    │
│  │ Slack          — send to channel    │    │
│  │ Notion         — create DB entry    │    │
│  │ Save to file   — save as file       │    │
│  │ Webhook        — HTTP POST          │    │
│  └─────────────────────────────────────┘    │
│                                             │
│  Delivery shows ONLY connected integrations │
│  If nothing connected: "Set up in Settings" │
└─────────────────────────────────────────────┘
```

One click → step added with smart defaults. No forms for presets.

### Smart Defaults Eliminate 80% of Fields

| Field | Current | Should be |
|-------|---------|-----------|
| Name | Manual entry | Auto from template (`meeting_notes`, `action_items`) |
| Connector | Dropdown | Hidden — determined by step type choice |
| Input | Dropdown | Auto: first = transcript, rest = previous step output |
| Provider | Always visible | Hidden — from global settings. "Override" for power users |
| Model | Always visible | Hidden — from global settings |
| Description | Manual entry | Auto from template, editable |

Result: for a preset step — **0 fields**. For custom prompt — **1 field** (textarea). For Slack — **2 fields** (workspace + channel).

### Assembly Example: HLTM Pipeline in 30 Seconds

```
1. [+ New Pipeline]     → name: "hltm"
2. [+ Add Step]         → click "Meeting Notes"        → done
3. [+ Add Step]         → click "Action Items"          → done
4. [+ Add Step]         → click "Slack" → #hltm         → done
5. [+ Add Step]         → click "Notion" → HLTM Tasks   → done
6. [Save]
```

Pipeline preview shows visual chain:
```
transcript → meeting_notes → action_items
                  ↓                ↓
             Slack #hltm     Notion HLTM Tasks
```

### Custom Prompt Step

When "Custom prompt" is selected:

```
┌────────────────────────────────────────┐
│ Name:   [my_analysis_______]           │
│                                        │
│ Prompt:                                │
│ ┌────────────────────────────────────┐ │
│ │ Проанализируй разговор и выдели   │ │
│ │ ключевые инсайты...               │ │
│ └────────────────────────────────────┘ │
│                                        │
│ □ Save as reusable template            │
│                                        │
│ [Advanced ▾]  ← provider/model override│
└────────────────────────────────────────┘
```

### Input Chaining: Simple Rule

```
Step 1: input = transcript (ALWAYS)
Step N: input = output of previous step (default)
        or = transcript (toggle)
        or = output of specific step (power user, dropdown)
```

90% of pipelines are linear chains. Don't complicate the UI for edge cases.

### Prompt Templates: Inline + Reusable

Two modes:
1. **Quick**: write prompt inline in the step. Used only in this pipeline.
2. **Reusable**: save as template → visible in other pipelines.

Built-in templates ship out of the box:
- Meeting Notes — participants, decisions, action items
- Summary — short extract
- Action Items — tasks with assignee
- Journal — thoughts as diary entry
- Structure — chaos → structured thinking

Can edit built-in templates, can create custom ones.

### Provider / Model

Global default in settings: "OpenAI / gpt-4o" (or whatever configured).

Processing steps use global default. "Override" button for per-step provider/model selection (e.g., Claude for action items because it structures better).

---

## Part 4: Integrations Architecture

### Three Layers

```
Layer 1: INTEGRATIONS (one-time setup)
  Settings > Integrations
  "Connect Slack", "Add Save path", "Connect Notion"

Layer 2: PIPELINE BUILDER (when creating pipeline)
  Settings > Pipelines > Edit
  "Processing: Meeting Notes" → "Delivery: Slack #hltm"
  Shows ONLY connected integrations

Layer 3: PIPELINE CHIPS (every day, during recording)
  App bar
  "Select hltm" → Record → automation kicks in
```

Each layer is simpler than the previous. Setup once → build pipeline once → use pipeline daily with one click.

### Integrations Settings

```
┌─ Settings > Integrations ─────────────────────────────┐
│                                                        │
│  CONNECTED                                             │
│  ┌──────────────────────────────────────────────┐     │
│  │ Slack    HLTM Workspace         [Test] [✕]  │     │
│  │ Slack    Personal Workspace     [Test] [✕]  │     │
│  │ Notion   HLTM Tasks (5 fields)  [Sync] [✕]  │     │
│  │ Save     ~/Documents/notes/     [Edit] [✕]  │     │
│  │ Save     ~/apex/self/           [Edit] [✕]  │     │
│  └──────────────────────────────────────────────┘     │
│                                                        │
│  AVAILABLE                                             │
│  ┌──────────────────────────────────────────────┐     │
│  │ Slack       + Add workspace                   │     │
│  │ Notion      + Connect                         │     │
│  │ Webhook     + Add endpoint                    │     │
│  │ Telegram    + Connect               (soon)   │     │
│  └──────────────────────────────────────────────┘     │
└────────────────────────────────────────────────────────┘
```

Key insight: **Save paths are integrations too.** Pre-configure named save locations (`~/Documents/notes/`, `~/apex/self/`) and select them in pipeline builder.

### Delivery Step Picker Shows Only Connected

```
┌─────────────────────────────────────────────┐
│  DELIVERY                                   │
│                                             │
│  Slack                                      │
│  ├── HLTM Workspace                         │
│  │   └── #hltm, #general, #random...       │
│  └── Personal Workspace                     │
│      └── @me, #ideas...                     │
│                                             │
│  Notion                                     │
│  └── HLTM Tasks (5 fields, 3 people)        │
│                                             │
│  Save                                       │
│  ├── ~/Documents/notes/                     │
│  └── ~/apex/self/                           │
│                                             │
│  Webhook                                    │
│  └── https://hooks.company.com/meetings     │
│                                             │
│  ─────────────────────────────────────────  │
│  Manage integrations...                     │
└─────────────────────────────────────────────┘
```

One click on `#hltm` → step created: connector=slack, workspace=HLTM, target=#hltm. Zero fields.

---

## Part 5: Schema-Aware Connectors (Notion, Linear, etc.)

### The Problem

Slack is simple: `send(text, channel)`. Done.

Notion is complex: `create_page(database, { title, assignee, due_date, priority, status, ... })`. Needs to know database schema, valid field values, and people mapping.

### Solution: Smart Setup Wizard

When connecting Notion, the app runs a setup wizard:

**Step 1:** API key / OAuth → connect to workspace

**Step 2:** Pick database from list (app reads available databases via Notion API)

**Step 3:** Schema auto-read — app reads database properties automatically:
```
Title       Text        ✅ mapped
Assignee    People      ✅ mapped
Due Date    Date        ✅ mapped
Priority    Select      ✅ mapped → High, Medium, Low
Status      Select      ⚙ default: [Todo ▾]
```

**Step 4:** People mapping — map names-in-conversation to Notion users:
```
"СК"  → Sergey Kopanev     ✅
"СШ"  → [Select user ▾]
"ДП"  → [Select user ▾]
```

Result: **Integration Profile** stored with schema, mappings, and defaults. Created once, used by every pipeline that targets this database.

### The Key Trick: Schema Informs the AI Prompt

When a pipeline has the chain:
```
transcript → [AI: Action Items] → [Notion: HLTM Tasks]
```

The system **automatically augments** the AI step's prompt with schema information from the Notion integration profile:

```
Base prompt (from user/template):
  "Extract action items from the transcript"

+ Auto-injected from Notion schema:
  "Output as JSON array. Each item must have:
   - title: string
   - assignee: one of [СК, СШ, ДП]
   - due_date: ISO date or null
   - priority: one of [High, Medium, Low]"
```

The user **never writes this format spec manually**. The system knows the schema → knows the required format → augments the prompt automatically.

AI outputs structured JSON:
```json
[
  { "title": "Deploy v2", "assignee": "СК", "due_date": "2026-02-20", "priority": "High" },
  { "title": "Update docs", "assignee": "СШ", "due_date": "2026-02-25", "priority": "Medium" }
]
```

Notion connector maps `"СК" → Notion user ID`, `"High" → Select option`, creates pages.

### Pattern: Schema-Aware Connector

This generalizes to any structured destination:

| Destination | Setup reads | Injects into AI prompt |
|-------------|------------|----------------------|
| Notion | DB fields, People, Select options | JSON format with valid values |
| Linear | Projects, Teams, Labels | Issue format with valid assignees |
| Jira | Projects, Issue Types, Users | Structured ticket format |
| Google Sheets | Columns, format | Row format |
| Slack | Nothing (just text) | Nothing — markdown as is |
| Save | Nothing (just file) | Nothing — markdown as is |

Slack/Save are "dumb" connectors (text → destination). Notion/Linear/Jira are "smart" connectors (structured → database). Same pattern, different complexity.

### Architecture Requirements

1. **Integration Profile** — stores schema, mappings, defaults. Created during setup wizard, updatable via re-sync
2. **Prompt Augmentation** — pipeline engine, before calling an AI step, checks: "what's the next step after this? Notion? Augment the prompt with format instructions from the integration profile"
3. **Output Parser** — structured connector parses AI output (JSON from frontmatter or body) and maps to API calls
4. **Re-sync** — "Update schema" button in integration settings (when Notion DB fields change)

---

## Open Questions (Remaining)

1. **Pipeline chips in app bar** — show all pipelines, or top N + overflow?
2. **Notion OAuth vs API key** — OAuth is better UX but more complex to implement. API key (internal integration) is simpler for v1.
3. **Schema re-sync** — automatic on each pipeline run, or manual button?
4. **Prompt augmentation visibility** — show the user what was auto-injected, or keep it invisible?
5. **Error recovery for structured output** — what if AI doesn't return valid JSON? Retry with stricter prompt? Show raw output and let user fix?

---

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Tags concept | Removed — replaced by pipelines | One concept instead of two. Pipeline with 0 steps = label. |
| Pipeline assignment | Pre-assign via chips in app bar | Most valuable moment is BEFORE recording. Chip = record button for that pipeline. |
| Step types in builder | "Processing" and "Delivery" (not "connectors") | User mental model, not implementation detail. |
| Builder approach | Preset picker, not form | One click for preset. Zero fields for known step types. |
| Provider/Model | Global default, per-step override hidden in "Advanced" | 95% of users use one provider. Don't show config they don't need. |
| Input chaining | Auto (previous step), override available | 90% linear chains. Don't complicate for edge cases. |
| Integrations | First-class concept in Settings, referenced by builder | Setup once, use everywhere. Builder shows only connected. |
| Save paths | Treated as integrations | Pre-configured named locations, not raw text input. |
| Structured destinations | Schema-aware setup wizard + prompt augmentation | Notion/Linear need schema. Setup reads it, AI prompt uses it. |
| Prompt templates | Inline + reusable, with built-in presets | Progressive disclosure: pick preset → customize → save as template. |
