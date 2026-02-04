---
stepsCompleted: ['step-01-init', 'step-02-discovery', 'step-03-success', 'step-04-journeys', 'step-05-domain-skipped', 'step-06-innovation', 'step-07-project-type', 'step-08-scoping', 'step-09-functional', 'step-10-nonfunctional', 'step-11-polish', 'step-12-complete']
workflowStatus: complete
completedAt: '2026-02-04'
inputDocuments:
  - input/brief.md
  - input/brief-enhanced.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/project-context.md
  - _bmad-output/planning-artifacts/epics.md
workflowType: 'prd-bugfix'
projectType: brownfield
classification:
  projectType: desktop_app
  domain: general
  complexity: medium
  projectContext: brownfield-bugfix
---

# PRD: Code Review Bug Fix Sprint

**Author:** sk
**Date:** 2026-02-04
**Type:** Brownfield Bug Fix
**Iteration:** v0.4.1
**Sprint:** Code Review Findings

## Executive Summary

A comprehensive code review of NBP identified 17 issues across Critical (3), Significant (7), and Minor (7) severity levels. This PRD covers the resolution of all 17 findings. No new features are introduced. All changes are fixes, refactors, and cleanup to existing functionality.

**Core Objective:** Improve code safety, eliminate security vulnerabilities, remove dead code, and fix correctness issues identified during code review.

## Success Criteria

### Acceptance

1. All 3 Critical issues resolved
2. All 7 Significant issues resolved
3. All 7 Minor issues addressed
4. Existing tests pass without regression
5. `cargo check` passes cleanly
6. No new bugs introduced
7. Existing functionality preserved

### Measurable Outcomes

| Metric | Target |
|--------|--------|
| Unsafe code blocks | Zero unnecessary `unsafe` in AudioState |
| XSS attack surface | Zero raw innerHTML with user data |
| Data corruption risk | Zero non-atomic metadata writes |
| Dead code | Zero template/scaffolding leftovers |
| Debug println! in production | Zero |
| API key exposure | Zero console logging of secrets |
| App startup time | Reduced by 500ms+ (permission check optimization) |

## Scope

### In Scope

All 17 issues from the code review (`input/brief-enhanced.md`), organized by severity.

### Out of Scope

- No new features
- No UI redesign
- No new dependencies (except where strictly needed for resampling fix)
- No architecture changes beyond targeted refactors

## Functional Requirements

### Critical Issues

#### FR-CR1: Remove Unnecessary `unsafe impl Send + Sync` for AudioState

**File:** `src-tauri/src/audio.rs:44-45`

**Current:** Manual `unsafe impl Send for AudioState` and `unsafe impl Sync for AudioState` bypass compiler safety checks. All fields in `AudioState` are `Mutex<T>` where `T: Send`, which means Rust auto-derives `Send + Sync`.

**Required:** Remove both `unsafe impl` lines. The compiler will verify safety automatically.

**Acceptance:**
- Given `AudioState` contains only `Mutex<T>` fields with `T: Send`
- When the unsafe impls are removed
- Then `cargo check` compiles without errors
- And future non-Send types added to AudioState will be caught by the compiler

#### FR-CR2: Fix XSS via innerHTML with User Data

**Files:** `src/main.js:469-480`, `src/main.js:526-550`, `src/main.js:700-705`

**Current:** Tag names and recording titles are injected directly into `innerHTML` without escaping. The `escapeHtml()` function exists (line 12-16) but is not used in these rendering paths. A tag like `<img src=x onerror=alert(1)>` executes arbitrary JavaScript.

**Required:** Apply `escapeHtml()` to all user-provided data before inserting into `innerHTML`:
- Tag names in tag rendering
- Recording titles in `renderRecordingsList()`
- Any other user-provided content inserted via innerHTML

**Acceptance:**
- Given a tag name containing `<script>alert(1)</script>`
- When rendered in the tag list
- Then the HTML entities are escaped and no script executes
- And the `escapeHtml()` function is used consistently in all innerHTML paths

#### FR-CR3: Atomic `write_metadata` in Storage

**File:** `src-tauri/src/storage.rs:123-128`

**Current:** `File::create` truncates the file before writing. If serialization fails after truncation, `metadata.json` is corrupted (empty or partial). Other modules (`pipeline_engine.rs`, `transcription.rs`) correctly use temp-file + rename for atomic writes.

**Required:** Implement temp-file + rename pattern consistent with other modules:
1. Write to `metadata.json.tmp` in the same directory
2. On successful write, rename `metadata.json.tmp` to `metadata.json`
3. On failure, leave original `metadata.json` intact

**Acceptance:**
- Given metadata serialization is in progress
- When the write to temp file succeeds
- Then atomic rename replaces the original file
- And if serialization fails, original metadata.json is preserved

### Significant Issues

#### FR-CR4: Extract Shared Mono-to-Stereo / De-interleave Logic

**File:** `src-tauri/src/mic_audio.rs:282-308` and `src-tauri/src/mic_audio.rs:422-445`

**Current:** ~25 lines of identical planar conversion logic is duplicated in the drain loop. Changes to one copy may not be reflected in the other.

**Required:** Extract the shared conversion logic into a helper function within `mic_audio.rs`. Both call sites should use the shared function.

**Acceptance:**
- Given the mono-to-stereo / de-interleave logic
- When either call site is invoked
- Then the same extracted function is used
- And no behavior change occurs in audio processing

#### FR-CR5: Atomic SharedAudioBuffer Channel Locking

**File:** `src-tauri/src/audio_processing/shared_buffer.rs:67-76`

**Current:** Left and right channels are locked with separate `Mutex` acquisitions. Between the two locks, a reader can pop from left but not right (or vice versa), causing L/R channel desync.

**Required:** Replace separate `Mutex<VecDeque>` for left and right with a single `Mutex<(VecDeque, VecDeque)>` to guarantee atomic access to both channels.

**Acceptance:**
- Given left and right audio channels
- When samples are pushed or popped
- Then both channels are accessed under a single lock
- And L/R channel desync is impossible

#### FR-CR6: Remove Dead `greet` Command

**File:** `src-tauri/src/lib.rs:58-60`

**Current:** The `greet` command is scaffolding from the Tauri template, still registered in the invoke handler.

**Required:** Remove the `greet` function and its registration from the invoke handler.

**Acceptance:**
- Given the `greet` command exists
- When removed
- Then `cargo check` compiles without errors
- And the invoke handler no longer references `greet`

#### FR-CR7: Fix `getDuration` to Check Mix Duration

**File:** `src/main.js:553-556`

**Current:** `getDuration` only checks `rec.audio.mic?.duration_sec` and `rec.audio.system?.duration_sec`. Since the default save mode is `save_mix_only`, mic and system are `null`, so it returns `0` for most recordings.

**Required:** Check `rec.audio.mix?.duration_sec` first, then fall back to mic/system durations.

**Acceptance:**
- Given a recording saved with `save_mix_only` mode
- When `getDuration` is called
- Then the mix duration is returned (not 0)

#### FR-CR8: Optimize `loadAudioDuration` to Use Single Metadata Read

**File:** `src/main.js:1062-1077`

**Current:** `loadAudioDuration` calls `list_recordings` (fetches all recordings) just to get the duration for one recording.

**Required:** Replace `list_recordings` call with `read_metadata` using the specific recording ID.

**Acceptance:**
- Given a recording detail view opens
- When `loadAudioDuration` is called
- Then only `read_metadata(recording_id)` is invoked (not `list_recordings`)

#### FR-CR9: Proper Downsampling with Anti-Aliasing Filter

**File:** `src-tauri/src/transcription.rs:625-627`

**Current:** Downsampling from 48kHz to 16kHz takes every 3rd sample (decimation) without an anti-aliasing filter, introducing aliasing artifacts. The real-time mic pipeline uses `rubato` with sinc interpolation.

**Required:** Use `rubato` (already a dependency) for proper sinc-interpolation-based resampling, consistent with the mic pipeline approach.

**Acceptance:**
- Given 48kHz audio needs downsampling to 16kHz
- When transcription prepares audio for Whisper
- Then sinc interpolation resampling is used (not naive decimation)
- And audio quality matches the real-time mic pipeline approach

#### FR-CR10: Lightweight Permission Check

**File:** `src-tauri/src/permissions.rs:38-47`

**Current:** System audio permission verification creates an actual `SystemAudioRecorder`, waits 500ms, and writes to `/tmp`. This heavyweight check runs on every app launch and adds 500ms+ startup delay.

**Required:** Use a lightweight check that does not create actual recordings. Options include:
- Check if Screen Recording permission is granted via macOS API without starting a recording
- Cache the permission state and only re-verify when explicitly requested
- Use a shorter timeout or non-recording API call

**Acceptance:**
- Given the app launches with onboarding completed
- When permission check runs
- Then no actual recording is started
- And startup delay from permission check is eliminated or reduced to <50ms

### Minor Issues

#### FR-CR11: Remove Duplicate Comment

**File:** `src-tauri/src/permissions.rs:57-58`

**Current:** `// Update mic cache too` comment is duplicated on consecutive lines.

**Required:** Remove the duplicate comment line.

#### FR-CR12: Fix Misplaced `cfg_attr` on `get_app_version`

**File:** `src-tauri/src/lib.rs:62-63`

**Current:** `#[cfg_attr(mobile, tauri::mobile_entry_point)]` is on the version getter function instead of the app entry point. It does nothing but is misleading.

**Required:** Remove the misplaced attribute from `get_app_version` and ensure it is correctly placed on the actual entry point (if needed).

#### FR-CR13: Fix Soft Clip Formula Discontinuity

**File:** `src-tauri/src/audio_processing/realtime_mixer.rs:212-218`

**Current:** At `x = 1.0`, the else branch evaluates to `0.0` while the if branch returns `1.0`. This creates a hard jump at the clipping threshold. The offline mixer (`mixer.rs:146`) uses smooth `tanh`-based clipping.

**Required:** Align the realtime mixer's clipping function with the offline mixer's `tanh`-based approach for consistent, smooth clipping behavior.

#### FR-CR14: Remove Debug `println!` from Production Code

**Files:** `src-tauri/src/mic_audio.rs:81`, `src-tauri/src/system_audio.rs:392-393`, `src-tauri/src/audio_processing/realtime_mixer.rs:49`, `src-tauri/src/permissions.rs:79-155`

**Current:** Debug `println!` statements remain in production code paths.

**Required:** Remove all debug `println!` statements from production paths. Use `#[cfg(debug_assertions)]` guard for any statements that should only appear during development, or replace with proper logging if needed.

#### FR-CR15: Import Shared Structs in Integration Tests

**File:** `src-tauri/tests/integration_tests.rs`

**Current:** Tests re-declare local copies of structs (`TestAudioInfo`, `TestAudioFiles`) that mirror real types. If real types change, tests will not catch the divergence.

**Required:** Import the actual types from the crate instead of re-declaring. Where direct import is not possible, document why and add comments noting which real types they mirror.

#### FR-CR16: Support Additional Sample Rates in OGG-to-WAV Conversion

**File:** `src-tauri/src/transcription.rs:625-633`

**Current:** Only 48kHz and 16kHz are supported. Any other sample rate (e.g., 44.1kHz from some microphones) returns `Err("Unsupported sample rate")`, breaking transcription.

**Required:** Support arbitrary sample rates by using the resampler (FR-CR9's approach via rubato) to convert any input rate to 16kHz for Whisper.

**Acceptance:**
- Given audio at 44.1kHz sample rate
- When transcription prepares audio for Whisper
- Then the audio is resampled to 16kHz without error

#### FR-CR17: Remove API Key Console Logging

**File:** `src/main.js:1176`

**Current:** `console.log("Loaded settings:", appSettings)` logs the full settings object including API keys to the WebView console.

**Required:** Remove the console.log statement, or redact sensitive fields before logging.

**Acceptance:**
- Given settings are loaded (with API keys configured)
- When `loadSettings()` runs
- Then no API keys appear in console output

## Non-Functional Requirements

- NFR-CR1: All fixes must pass `cargo check` without warnings
- NFR-CR2: No new dependencies introduced (rubato already present for FR-CR9/FR-CR16 resampling)
- NFR-CR3: Existing tests must continue to pass
- NFR-CR4: No user-visible behavior changes (except corrected speaker colors, corrected durations, and faster startup)
- NFR-CR5: Code follows existing project conventions documented in CLAUDE.md

## Epic Structure

### Epic 7: Code Review Bug Fixes

**Single epic** containing all 17 stories organized by priority.

| Story | Issue | Priority | Estimate | Dependency |
|-------|-------|----------|----------|------------|
| 7.1 | Remove unsafe Send+Sync for AudioState | Critical | XS | None |
| 7.2 | Fix XSS via innerHTML with user data | Critical | S | None |
| 7.3 | Atomic write_metadata in storage | Critical | S | None |
| 7.4 | Extract shared mono-to-stereo logic | Significant | S | None |
| 7.5 | Atomic SharedAudioBuffer channel locking | Significant | M | None |
| 7.6 | Remove dead greet command | Significant | XS | None |
| 7.7 | Fix getDuration to check mix duration | Significant | XS | None |
| 7.8 | Optimize loadAudioDuration | Significant | XS | None |
| 7.9 | Proper downsampling with anti-aliasing | Significant | M | None |
| 7.10 | Lightweight permission check | Significant | M | None |
| 7.11 | Remove duplicate comment | Minor | XS | None |
| 7.12 | Fix misplaced cfg_attr | Minor | XS | 7.6 |
| 7.13 | Fix soft clip formula discontinuity | Minor | S | None |
| 7.14 | Remove debug println! | Minor | S | None |
| 7.15 | Import shared structs in tests | Minor | S | None |
| 7.16 | Support additional sample rates | Minor | S | 7.9 |
| 7.17 | Remove API key console logging | Minor | XS | None |

**Implementation Order:**

1. **Phase 1 (Critical):** 7.1, 7.2, 7.3 (can be parallelized)
2. **Phase 2 (Significant - Rust):** 7.4, 7.5, 7.6, 7.9, 7.10
3. **Phase 3 (Significant - JS):** 7.7, 7.8
4. **Phase 4 (Minor):** 7.11, 7.12, 7.13, 7.14, 7.15, 7.16, 7.17

**Rationale:** Critical issues first for safety. Rust changes grouped together. JS changes grouped together. Minor issues last as they have lowest impact. Story 7.16 depends on 7.9 (shared resampler approach). Story 7.12 depends on 7.6 (both touch lib.rs).

## UX Impact

**Minimal.** User-visible changes:
1. **Speaker colors** will now work correctly (if affected by prior BUG-1 fix)
2. **Recording durations** will display correctly in list view (FR-CR7)
3. **Faster startup** from lightweight permission check (FR-CR10)
4. **XSS prevention** makes the app safer (FR-CR2)

No UI layout, interaction, or design changes are required.
