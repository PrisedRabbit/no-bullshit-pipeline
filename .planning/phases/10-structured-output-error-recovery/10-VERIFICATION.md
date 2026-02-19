---
phase: 10-structured-output-error-recovery
verified: 2026-02-19T08:15:00Z
status: passed
score: 4/4 must-haves verified
re_verification: false
---

# Phase 10: Structured Output Error Recovery Verification Report

**Phase Goal:** Pipeline steps that require structured AI output (JSON) handle failures gracefully without silent data loss or pipeline abandonment
**Verified:** 2026-02-19T08:15:00Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Success Criteria (from ROADMAP.md)

| # | Success Criterion | Status | Evidence |
|---|---|---|---|
| SC-1 | When a Notion delivery step receives invalid JSON from the AI, the system automatically retries with a stricter prompt before reporting failure | VERIFIED | `pipeline_engine.rs:483-594` — `execute_structured()` called for Notion steps; `JsonParse` match arm triggers `execute_retry()` once before failing |
| SC-2 | When structured output parsing fails after retry, the pipeline run output shows the raw AI response alongside a clear, actionable error message | VERIFIED | `notion.rs:606-611` — `write_failure_output()` writes `## Error\n{error}\n\n## Raw AI Output\n{raw}` body; used by both `execute_with_raw_preservation()` and the double-failure inline path in `pipeline_engine.rs:579-590` |
| SC-3 | When one delivery step fails, subsequent independent steps in the pipeline still execute and deliver their output | VERIFIED | `pipeline_engine.rs:649-659` — delivery step failures do NOT break out of the loop; only processing step failures (LLM) break out |
| SC-4 | Pipeline run output distinguishes between steps that succeeded, steps that failed, and steps that were skipped | VERIFIED | `pipelines.rs:80` — `StepStatus.status` accepts `"pending", "running", "done", "failed", "skipped"`; skipped `.md` files written at `pipeline_engine.rs:296-313` (inline) and `664-691` (post-loop sweep); `main.js:410-451` renders per-step icons for partial pipelines |

**Score:** 4/4 success criteria verified

---

## Required Artifacts

### Plan 10-01 Artifacts

| Artifact | Expected | Exists | Substantive | Wired | Status |
|---|---|---|---|---|---|
| `src-tauri/src/connectors/notion.rs` | Error categorization (NotionErrorKind) and raw output preservation | Yes | Yes — `NotionErrorKind` enum (lines 24-30), `NotionError` struct (lines 37-54), `write_failure_output()` (lines 582-619), `execute_structured()` (lines 748-759), `execute_with_raw_preservation()` (lines 770-811) | Yes — consumed at `pipeline_engine.rs:481-599` | VERIFIED |
| `src-tauri/src/pipeline_engine.rs` | JSON retry logic with stricter prompt and max 1 retry | Yes | Yes — retry arm at lines 495-594; `execute_retry` called exactly once; no loop | Yes — calls into `connectors::notion` and `connectors::llm` | VERIFIED |
| `src-tauri/src/connectors/llm.rs` | Public function for retry call with corrective prompt | Yes | Yes — `pub async fn execute_retry()` at line 341 with full corrective prompt construction (lines 367-382) | Yes — called from `pipeline_engine.rs:532` | VERIFIED |

### Plan 10-02 Artifacts

| Artifact | Expected | Exists | Substantive | Wired | Status |
|---|---|---|---|---|---|
| `src-tauri/src/pipeline_engine.rs` | Partial-success execution logic using `is_delivery_connector` | Yes | Yes — `is_delivery()` called at lines 393 and 649; `has_failure`/`failed_or_skipped` HashSet tracking at lines 283-286; final `PipelineStatus::Partial` at lines 695-699 | Yes — drives execution branching for all step types | VERIFIED |
| `src-tauri/src/pipelines.rs` | `StepStatus` with skipped state and `ConnectorType::is_delivery()` | Yes | Yes — `StepStatus.status: String` documents `"skipped"` at line 80; `is_delivery()` impl at lines 27-29 covers `Notion | Slack | Webhook | Save` | Yes — `is_delivery()` used in `pipeline_engine.rs` | VERIFIED |
| `src/main.js` | Per-step status rendering with `step-status-icon` | Yes | Yes — `renderPipelineStatus()` at lines 387-459 calls `get_step_outputs` for `partial` pipelines and renders done/failed/skipped rows with Unicode icons | Yes — triggered from `pipeline-progress` events and recording selection at line 764 | VERIFIED |

---

## Key Link Verification

### Plan 10-01 Key Links

| From | To | Via | Pattern | Status |
|---|---|---|---|---|
| `pipeline_engine.rs` | `connectors/notion.rs` | `NotionErrorKind` enum to detect JSON parse failures | `NotionErrorKind::JsonParse` | VERIFIED — line 495: `matches!(notion_err.kind, NotionErrorKind::JsonParse { .. })` |
| `pipeline_engine.rs` | `connectors/llm.rs` | `execute_retry` call for stricter prompt retry | `execute_retry` | VERIFIED — line 532: `connectors::llm::execute_retry(...)` |

### Plan 10-02 Key Links

| From | To | Via | Pattern | Status |
|---|---|---|---|---|
| `pipeline_engine.rs` | `pipelines.rs` | `ConnectorType` used to determine processing vs delivery | `is_delivery_connector` / `is_delivery()` | VERIFIED — lines 393, 649: `step.connector.is_delivery()` |
| `src/main.js` | `pipeline_engine.rs` | `get_step_outputs` returns per-step status including skipped | `get_step_outputs` | VERIFIED — `main.js:412`: `invoke('get_step_outputs', ...)` inside `partial` status branch |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| ERR-01 | 10-01-PLAN | System retries with a stricter prompt when AI returns invalid JSON for structured delivery steps (Notion, etc.) | SATISFIED | `pipeline_engine.rs:495-594` — `JsonParse` match triggers `execute_retry()` with corrective prompt containing first 2000 chars of failed output + last 500 chars of original prompt |
| ERR-02 | 10-01-PLAN | User sees the raw AI output with an actionable error message when structured output parsing fails after retry | SATISFIED | `notion.rs:605-611` — `write_failure_output()` includes `## Error` + `## Raw AI Output` sections; used in `execute_with_raw_preservation()` (line 798) and double-failure inline path (`pipeline_engine.rs:580`) |
| ERR-03 | 10-02-PLAN | Pipeline execution continues to subsequent independent steps when one delivery step fails (partial success) | SATISFIED | `pipeline_engine.rs:649-659` — delivery step failure uses `continue` not `break`; skipped files written for dependency-blocked steps; final status `PipelineStatus::Partial` when `has_failure` is true |

**No orphaned requirements.** All three ERR requirements explicitly claimed by plans and fully verified.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/main.js` | 798 | `// for now just show placeholder` | Info | Pre-existing stub in unrelated summary-loading code path — NOT in phase 10 scope (no pipeline error recovery connection) |

No blockers or warnings found in phase 10 modified files.

---

## Human Verification Required

### 1. JSON Retry — End-to-End Happy Path

**Test:** Run a pipeline where the LLM step returns valid JSON on the first try. Verify the Notion step creates pages normally and no retry file (`{step}-retry.md`) appears in the pipeline output directory.
**Expected:** Pipeline status = Done, one `.md` file per step, no `-retry.md` file.
**Why human:** Requires a live LLM API call and Notion API connection.

### 2. JSON Retry — Corrective Retry Success

**Test:** Configure a pipeline with a prompt that reliably causes the LLM to return prose instead of JSON on the first call, then valid JSON on a second call with corrective prompt. Trigger the pipeline.
**Expected:** Pipeline finds `{step}-retry.md` in the output directory, Notion pages are created, pipeline status = Done.
**Why human:** Requires engineered LLM prompt failure + retry behavior that cannot be mocked without running the app.

### 3. Double Failure — Raw Output Preserved

**Test:** Configure a pipeline where both the original and retry LLM calls return invalid JSON. Run the pipeline.
**Expected:** Pipeline status = Partial. The Notion step's `.md` file contains `## Raw AI Output` section with the original failed text. No silent data loss.
**Why human:** Requires deliberate construction of a consistently-failing JSON response.

### 4. Partial Success — Independent Delivery Steps

**Test:** Build a pipeline `[LLM, Notion(failing), Slack(input=LLM)]`. Run it. The Notion step fails.
**Expected:** Slack step still executes and delivers. Pipeline status = Partial. UI shows: LLM (checkmark), Notion (red X + error), Slack (checkmark).
**Why human:** Requires live Notion + Slack integration with a Notion step configured to fail.

### 5. Per-Step UI Rendering

**Test:** After running a partial pipeline, inspect the pipeline status section in the UI.
**Expected:** Each step has a correctly colored icon (green checkmark, red X, or gray circle) with step name visible. Failed steps show truncated error text with full error accessible on hover via `title` attribute.
**Why human:** Visual rendering and CSS styling require visual inspection.

---

## Detailed Verification Notes

### Retry Logic (ERR-01, ERR-02)

The retry path in `pipeline_engine.rs` is fully traced:

1. `execute_structured()` called at line 483 — returns `Result<PathBuf, NotionError>`
2. `Err(NotionError { kind: JsonParse { .. } })` matched at line 495
3. `raw_output` extracted from error variant at line 497-499
4. Previous LLM step located by index backward scan at lines 514-518
5. `build_augmented_prompt()` re-built for retry context at lines 526-530
6. `connectors::llm::execute_retry()` called at line 532 — exactly once, no loop
7. Retry success path: `execute_with_raw_preservation()` called at line 553 with retry output path
8. Retry failure path: failure `.md` written inline at lines 570-592 with `## Raw AI Output` section

**Max 1 retry confirmed:** No `loop`, `while`, or recursive call pattern exists in the retry branch. The `execute_retry` call site is not wrapped in any iteration construct.

### Partial-Success Logic (ERR-03)

The execution model in `pipeline_engine.rs`:

- `has_failure: bool` + `failed_or_skipped: HashSet<String>` initialized at lines 283-287
- Skip check at top of loop (line 293): skips if `step.input` is in `failed_or_skipped` and input is not `"transcript"`
- Processing failure path (lines 649-655): marks all remaining steps as skipped, then `break`
- Delivery failure path (lines 657-659): `continue` to next step — the loop does NOT break
- Post-loop sweep (lines 664-692): writes `.md` files for any skipped steps that never entered the loop body (processing-halt scenario)
- Final status (lines 695-699): `PipelineStatus::Partial` when `has_failure`, `Done` otherwise

### Frontend Per-Step Display (ERR-03)

`renderPipelineStatus()` in `main.js` only renders per-step detail when `state.status === 'partial'` (line 410). For `done` pipelines no step detail is shown. CSS classes verified in `styles.css` lines 3118-3172 with correct color values: green `#4caf50`, red `#f44336`, gray `#9e9e9e`.

---

## Summary

Phase 10 achieves its goal. All four success criteria are fully implemented and wired:

- **ERR-01 (retry):** `NotionErrorKind::JsonParse` drives a single corrective-prompt retry through `execute_retry()` in `llm.rs`. The retry is bounded to exactly once with no retry loop.
- **ERR-02 (raw output):** `write_failure_output()` in `notion.rs` appends `## Raw AI Output` to the step `.md` file on any final failure. Used consistently across both retry-failure branches.
- **ERR-03 (partial success):** Delivery connectors (Notion, Slack, Webhook, Save) are classified via `ConnectorType::is_delivery()`. Their failures use `continue` not `break`. Processing step (LLM) failures still halt downstream. The frontend renders per-step done/failed/skipped indicators for partial pipelines.

No structural gaps, stubs, or missing wiring found. Five human verification tests identified for live API behavior confirmation.

---

_Verified: 2026-02-19T08:15:00Z_
_Verifier: Claude (gsd-verifier)_
