//! Call-session orchestrator. Auto-records detected calls and gives the user
//! a 10-second window to abort via the popup's Ignore button.
//!
//! Lifecycle (`CallStage`):
//!   `Starting { abort_pending }` — `audio::start_recording` is in flight.
//!       Spawned in a thread because start can take 0.5–2s and we don't want
//!       to block the call-event listener. If user clicks Ignore or the call
//!       ends during this window, `abort_pending` flips true; the spawned
//!       thread, on completion, immediately stops and deletes the recording.
//!       → success + !abort_pending → `Recording { id }`
//!       → success + abort_pending  → stop + delete + cleared
//!       → start failed             → cleared + error popup
//!   `Recording { id }` — recording active, owned by this session.
//!       → call-event(ended) → stop, keep if duration > auto_discard_seconds,
//!                              emit "saved" popup
//!       → ignore_call_recording → stop + force-delete (regardless of length)
//!
//! Locking order across managed states: CallDetectorState → CallSessionState
//! → AudioState → DictationState. This module acquires CallSessionState plus
//! short reads of AudioState (current_session, is_recording) and
//! DictationState (is_active) — all released before any audio::* call.

use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::thread;
use tauri::{Emitter, Listener, Manager};

/// State machine for an active call. See module docs for transitions.
#[derive(Debug)]
enum CallStage {
    Starting { abort_pending: bool },
    Recording { id: String },
}

#[derive(Debug)]
struct ActiveCall {
    /// Session UUID — distinguishes "still my session" from "another call
    /// took over" inside spawned continuations.
    session_id: String,
    call_app: Option<String>,
    stage: CallStage,
}

pub struct CallSessionState {
    active: Mutex<Option<ActiveCall>>,
}

impl CallSessionState {
    pub fn new() -> Self {
        Self { active: Mutex::new(None) }
    }
}

// The popup's "ignore or let it run" window is enforced JS-side (10s
// auto-hide in call-popup.js). After it fades, the recording continues
// silently until the call ends; user can still stop via the main window.

/// Wire up the Rust-side listener on `call-event`. Called once from
/// `lib.rs` setup; the listener lives for the app's lifetime.
pub fn install(app_handle: &tauri::AppHandle) {
    let app = app_handle.clone();
    app_handle.listen("call-event", move |event| {
        let payload: serde_json::Value = match serde_json::from_str(event.payload()) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("call_session: bad call-event payload: {}", e);
                return;
            }
        };
        let stage = payload.get("stage").and_then(|v| v.as_str()).unwrap_or("");
        let call_app = payload.get("app").and_then(|v| v.as_str()).map(String::from);
        let bundle_id = payload.get("bundle_id").and_then(|v| v.as_str()).map(String::from);

        match stage {
            "started" => handle_started(&app, call_app, bundle_id),
            "ended" => handle_ended(&app),
            other => log::debug!("call_session: ignoring unknown stage {:?}", other),
        }
    });
}

fn handle_started(app: &tauri::AppHandle, call_app: Option<String>, bundle_id: Option<String>) {
    log::info!("[ignore-trace] handle_started: call_app={:?} bundle={:?}", call_app, bundle_id);
    // Belt-and-suspenders self-mic guards. call_detector already suppresses
    // call-event(started) when NBP itself just opened the mic, but cover the
    // race window where this listener fires after the flag flipped.
    let audio_recording = app
        .state::<crate::audio::AudioState>()
        .is_recording
        .lock()
        .map(|g| *g)
        .unwrap_or(false);
    let dictation_active = app
        .state::<crate::dictation::DictationState>()
        .is_active
        .load(Ordering::Relaxed);
    if audio_recording || dictation_active {
        log::info!(
            "[ignore-trace] handle_started: NBP-self active (recording={}, dictation={}) — skipping",
            audio_recording,
            dictation_active
        );
        return;
    }

    let session_id = uuid::Uuid::new_v4().to_string();
    let session = ActiveCall {
        session_id: session_id.clone(),
        call_app: call_app.clone(),
        stage: CallStage::Starting { abort_pending: false },
    };

    {
        let state = app.state::<CallSessionState>();
        let mut active = match state.active.lock() {
            Ok(g) => g,
            Err(e) => {
                log::error!("call_session: state lock poisoned: {}", e);
                return;
            }
        };
        // Overwrite policy with stale-detection:
        //   • None → replace freely
        //   • Starting { abort_pending: true } → already cancelled, replace
        //   • Recording { id } but AudioState.is_recording=false → STALE
        //     (the recording was stopped by another path, or the detector
        //     missed the ended event). Replace; otherwise the user is
        //     permanently locked out of new call detections.
        //   • Anything else → real conflict, drop the new event.
        if let Some(ref existing) = *active {
            let stale_recording = matches!(existing.stage, CallStage::Recording { .. })
                && !audio_recording;
            let abort_pending = matches!(
                existing.stage,
                CallStage::Starting { abort_pending: true }
            );
            if !stale_recording && !abort_pending {
                log::warn!(
                    "call_session: started while existing session {} in stage {:?} (is_recording={}) — ignoring",
                    existing.session_id,
                    existing.stage,
                    audio_recording
                );
                return;
            }
            if stale_recording {
                log::warn!(
                    "call_session: replacing stale session {} (Recording stage but AudioState.is_recording=false — detector or stop path missed clearing it)",
                    existing.session_id
                );
            }
        }
        *active = Some(session);
    }

    log::info!("call_session: starting recording for session {} (app={:?})", session_id, call_app);

    // Position the popup on the cursor's monitor before emitting.
    crate::reposition_call_popup(app);

    // Show the popup with the Ignore-only affordance. JS auto-hides after
    // IGNORE_WINDOW_SECS; recording continues regardless of popup state.
    let _ = app.emit(
        "call-popup",
        serde_json::json!({ "kind": "recording", "app": call_app }),
    );

    // Drive the long-running audio::start_recording in a worker so we don't
    // block the call-event listener. Result is committed back into
    // CallSessionState via the same session_id.
    let app_for_start = app.clone();
    let session_id_for_start = session_id.clone();
    let call_app_for_start = call_app.clone();
    thread::spawn(move || {
        run_start(app_for_start, session_id_for_start, call_app_for_start, bundle_id);
    });
}

/// Worker: calls `audio::start_recording`, tags + assigns default pipeline,
/// then commits to Recording { id } or rolls back if Ignore/ended fired
/// during the start.
fn run_start(
    app: tauri::AppHandle,
    session_id: String,
    call_app: Option<String>,
    bundle_id: Option<String>,
) {
    let audio_state = app.state::<crate::audio::AudioState>();
    let save_mix_only = crate::config::load_settings().save_mix_only;
    let start_result = crate::audio::start_recording(app.clone(), audio_state, save_mix_only);

    match start_result {
        Ok(metadata) => {
            let recording_id = metadata.id.clone();

            // No tags, no default-pipeline auto-attach. Pipelines stay manual
            // (user picks via chip bar on the recording). Transcription is
            // triggered automatically from handle_ended — see below. Tags
            // were avoided because `migrate_tags_to_pipeline_labels` in
            // storage.rs auto-converts every tag into a zero-step pipeline,
            // which would pollute the pipelines dropdown with junk like
            // "auto-detected" / "zoom".

            // Replace the empty title `audio::start_recording` creates with
            // something readable: "Zoom · 19 May 14:23". Falls back to "Call"
            // when no app could be identified (rare — daemon-only path).
            let app_label = call_app.as_deref().unwrap_or("Call");
            let title = format!(
                "{} · {}",
                app_label,
                chrono::Local::now().format("%H:%M")
            );
            if let Ok(mut meta) = crate::storage::read_metadata(&recording_id) {
                meta.title = title;
                meta.app_bundle_id = bundle_id.clone();
                if let Err(e) = crate::storage::write_metadata(&meta) {
                    log::warn!(
                        "call_session: failed to write title for {}: {}",
                        recording_id,
                        e
                    );
                }
            }

            // Notify the frontend the recording exists so the main-window
            // list refreshes during the call (instead of only after the
            // recording_complete event fires post-finalize).
            let empty_pipelines: Vec<String> = Vec::new();
            let _ = app.emit(
                "recording_started",
                serde_json::json!({
                    "id": recording_id,
                    "pipelines": empty_pipelines,
                    "source": "call",
                }),
            );

            // Commit state: either Recording { id } or roll-back if abort
            // was requested during start (Ignore click or call ended).
            let abort = {
                let state = app.state::<CallSessionState>();
                let mut active = match state.active.lock() {
                    Ok(g) => g,
                    Err(_) => return,
                };
                match active.as_mut() {
                    Some(s) if s.session_id == session_id => match s.stage {
                        CallStage::Starting { abort_pending: true } => {
                            *active = None;
                            true
                        }
                        CallStage::Starting { abort_pending: false } => {
                            s.stage = CallStage::Recording { id: recording_id.clone() };
                            false
                        }
                        ref other => {
                            log::warn!(
                                "call_session: unexpected stage {:?} for session {} after start — treating as abort",
                                other,
                                session_id
                            );
                            *active = None;
                            true
                        }
                    },
                    _ => {
                        log::warn!(
                            "call_session: session {} lost during start — aborting",
                            session_id
                        );
                        true
                    }
                }
            };

            if abort {
                // Verify ownership before stopping (defends against the user
                // having manually replaced the recording in the meantime).
                let current_id = app
                    .state::<crate::audio::AudioState>()
                    .current_session
                    .lock()
                    .ok()
                    .and_then(|g| g.as_ref().map(|m| m.id.clone()));
                if current_id.as_ref() == Some(&recording_id) {
                    let _ = crate::audio::stop_recording(
                        app.clone(),
                        app.state::<crate::audio::AudioState>(),
                    );
                }
                // Force-delete: short recordings would be auto-discarded by
                // stop_recording anyway, but if user clicked Ignore on a
                // longer call we still want it gone.
                let dir = crate::storage::get_recording_dir(&recording_id);
                if dir.exists() {
                    let _ = std::fs::remove_dir_all(&dir);
                }
                // Always invalidate — the dir may already be gone (stop_recording's
                // discard path removed it), but the list cache still holds a row
                // with status="recording". Without this nudge, loadRecordings
                // would serve the stale entry and the live timer keeps ticking.
                crate::storage::invalidate_list_cache();
                // Tell the frontend the recording is gone so the list drops
                // its row immediately. recording_complete from a failed
                // finalize might also fire later, but waiting for it would
                // leave the user staring at a stale "Recording 0:42" row.
                let _ = app.emit("recording_discarded", &recording_id);
                log::info!(
                    "call_session: session {} aborted during start, recording {} deleted",
                    session_id,
                    recording_id
                );
            } else {
                log::info!(
                    "call_session: session {} → recording {}",
                    session_id,
                    recording_id
                );
            }
        }
        Err(e) => {
            // Clear session and surface the error to the popup.
            {
                let state = app.state::<CallSessionState>();
                if let Ok(mut active) = state.active.lock() {
                    if let Some(ref s) = *active {
                        if s.session_id == session_id {
                            *active = None;
                        }
                    }
                }
            }
            let msg = format!("Recording failed: {}", e);
            log::warn!("call_session: {}", msg);
            let _ = app.emit(
                "call-popup",
                serde_json::json!({
                    "kind": "error",
                    "app": call_app,
                    "message": msg,
                }),
            );
        }
    }
}

fn handle_ended(app: &tauri::AppHandle) {
    log::info!("[ignore-trace] handle_ended called");
    let owned = {
        let state = app.state::<CallSessionState>();
        let mut active = match state.active.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        log::info!(
            "[ignore-trace] handle_ended: active = {:?}",
            active.as_ref().map(|s| format!(
                "session_id={} app={:?} stage={:?}",
                s.session_id, s.call_app, s.stage
            ))
        );
        match active.as_mut() {
            None => {
                log::info!("[ignore-trace] handle_ended: active=None (already cleared by ignore?) — returning");
                return;
            }
            Some(s) => match s.stage {
                CallStage::Starting { .. } => {
                    log::info!(
                        "[ignore-trace] handle_ended: stage=Starting (session {}) — flagging abort_pending",
                        s.session_id
                    );
                    s.stage = CallStage::Starting { abort_pending: true };
                    return;
                }
                CallStage::Recording { ref id } => {
                    let rid = id.clone();
                    let call_app = s.call_app.clone();
                    *active = None;
                    log::info!("[ignore-trace] handle_ended: stage=Recording id={} — cleared *active", rid);
                    Some((rid, call_app))
                }
            },
        }
    };

    let Some((rid, call_app)) = owned else { return };

    // Verify ownership before stopping. If the user manually replaced the
    // recording via main-window stop+start, don't yank the unrelated one.
    let audio_state = app.state::<crate::audio::AudioState>();
    let current_id = audio_state
        .current_session
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|m| m.id.clone()));

    if current_id.as_ref() != Some(&rid) {
        log::info!(
            "call_session: owned recording {} no longer current (current={:?}) — not stopping",
            rid,
            current_id
        );
        return;
    }

    log::info!("call_session: call ended, stopping owned recording {}", rid);
    if let Err(e) = crate::audio::stop_recording(app.clone(), audio_state) {
        log::warn!("call_session: stop_recording failed for {}: {}", rid, e);
    }

    // Only claim "saved" if the recording survived the < auto_discard_seconds
    // gate inside stop_recording.
    if crate::storage::get_recording_dir(&rid).exists() {
        let _ = app.emit(
            "call-popup",
            serde_json::json!({ "kind": "saved", "app": call_app, "recording_id": rid.clone() }),
        );

        // Force auto-transcribe regardless of JS appSettings gate. User's
        // contract: every call recording always gets transcribed. Pipelines
        // remain manual. transcribe_recording itself still respects
        // settings.transcription.enabled on the Rust side — if user has
        // disabled transcription globally, this is a polite no-op. Runs in
        // the Tauri async runtime so handle_ended returns promptly.
        let app_for_tx = app.clone();
        let rid_for_tx = rid.clone();
        tauri::async_runtime::spawn(async move {
            let ts_state = app_for_tx.state::<crate::transcription::TranscriptionState>();
            match crate::transcription::transcribe_recording(
                app_for_tx.clone(),
                rid_for_tx.clone(),
                ts_state,
            )
            .await
            {
                Ok(_) => log::info!("call_session: auto-transcribed {}", rid_for_tx),
                Err(e) => log::warn!(
                    "call_session: auto-transcribe failed for {}: {}",
                    rid_for_tx,
                    e
                ),
            }
        });
    } else {
        log::info!(
            "call_session: owned recording {} was too short, auto-discarded — no saved popup",
            rid
        );
    }
}

/// User clicked Ignore on the popup. Stops the current call recording and
/// force-deletes its directory regardless of duration.
#[tauri::command]
pub fn ignore_call_recording(app: tauri::AppHandle) {
    log::info!("[ignore-trace] entered ignore_call_recording");
    let owned = {
        let state = app.state::<CallSessionState>();
        let mut active = match state.active.lock() {
            Ok(g) => g,
            Err(_) => {
                log::warn!("[ignore-trace] active lock poisoned, bailing");
                return;
            }
        };
        log::info!(
            "[ignore-trace] active snapshot: {:?}",
            active.as_ref().map(|s| format!(
                "session_id={} app={:?} stage={:?}",
                s.session_id, s.call_app, s.stage
            ))
        );
        match active.as_mut() {
            None => {
                log::info!("[ignore-trace] active=None, returning early");
                return;
            }
            Some(s) => match s.stage {
                CallStage::Starting { .. } => {
                    log::info!(
                        "[ignore-trace] stage=Starting (session {}) — flagging abort_pending",
                        s.session_id
                    );
                    s.stage = CallStage::Starting { abort_pending: true };
                    return;
                }
                CallStage::Recording { ref id } => {
                    let rid = id.clone();
                    *active = None;
                    log::info!("[ignore-trace] stage=Recording id={} — cleared *active", rid);
                    Some(rid)
                }
            },
        }
    };

    let Some(rid) = owned else { return };

    // Verify ownership and stop.
    let audio_state = app.state::<crate::audio::AudioState>();
    let current_id = audio_state
        .current_session
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|m| m.id.clone()));
    log::info!(
        "[ignore-trace] audio.current_session.id = {:?}, target rid = {}",
        current_id,
        rid
    );
    if current_id.as_ref() == Some(&rid) {
        log::info!("[ignore-trace] ownership matches, calling stop_recording");
        match crate::audio::stop_recording(app.clone(), audio_state) {
            Ok(_) => log::info!("[ignore-trace] stop_recording returned Ok"),
            Err(e) => log::warn!("[ignore-trace] stop_recording returned Err: {}", e),
        }
    } else {
        log::warn!(
            "[ignore-trace] ownership MISMATCH (current={:?}, target={}) — skipping stop_recording",
            current_id,
            rid
        );
    }

    // Force-delete the recording directory. stop_recording may have already
    // done it for short recordings; for longer ones we still want it gone
    // because the user explicitly said Ignore.
    let dir = crate::storage::get_recording_dir(&rid);
    let dir_existed = dir.exists();
    log::info!(
        "[ignore-trace] dir={} exists_before_remove={}",
        dir.display(),
        dir_existed
    );
    if dir_existed {
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            log::warn!("[ignore-trace] failed to delete {} after ignore: {}", rid, e);
        } else {
            log::info!("[ignore-trace] removed dir {}", dir.display());
        }
    }
    let dir_after = dir.exists();
    log::info!("[ignore-trace] dir exists_after_remove={}", dir_after);

    // Always invalidate the list cache here — regardless of which path
    // deleted the dir (stop_recording's discard, our remove_dir_all, or
    // finalize racing with us). Without this, loadRecordings on the
    // frontend serves the stale row and the UI keeps showing the recording
    // that disk no longer has.
    crate::storage::invalidate_list_cache();
    log::info!("[ignore-trace] invalidate_list_cache() done");

    // Notify the frontend so the recordings list refreshes immediately.
    // Without this, the row keeps ticking its live timer until something
    // else triggers a list reload.
    let _ = app.emit("recording_discarded", &rid);
    log::info!("[ignore-trace] emitted recording_discarded({})", rid);

    log::info!("[ignore-trace] DONE: recording {} ignored", rid);
}
