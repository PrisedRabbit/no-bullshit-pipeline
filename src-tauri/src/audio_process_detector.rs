//! Per-process audio activity detector. Uses the HAL Process API introduced in
//! macOS 14.4 (`kAudioHardwarePropertyProcessObjectList` + per-process
//! properties) to know exactly **which** process is using the microphone — not
//! just whether any device is "running somewhere".
//!
//! Why this exists alongside the older `call_detector` module:
//!   • `kAudioDevicePropertyDeviceIsRunningSomewhere` (device-level) does not
//!     reliably flip when FaceTime / CallKit / `avconferenced` routes audio
//!     through the system daemon path. Device-level detection misses those.
//!   • The per-process API gives us PID + bundle ID + IsRunningInput flag for
//!     every audio client, which is the right signal.
//!
//! Flow:
//!   1. Background thread polls every 500ms.
//!   2. Enumerates audio processes via `kAudioHardwarePropertyProcessObjectList`.
//!   3. For each, reads PID, BundleID, IsRunningInput. Skips our own PID.
//!   4. Tracks the set of processes currently using input.
//!   5. Diff vs. previous tick: emits `call-event` `{stage: "started", app}`
//!      for new entries, `{stage: "ended", app}` for removed entries.
//!
//! Labeling priority:
//!   1. `NSRunningApplication.localizedName` for PID — Zoom, Teams, Slack get
//!      their proper user-facing names from macOS itself. Zero maintenance.
//!   2. `DAEMON_LABELS` mapping for system daemons that lack an NSRunningApp
//!      entry (`avconferenced` → "FaceTime"). Tiny list, rarely changes.
//!   3. `proc_name(pid)` short executable name as last resort.
//!
//! Self-filtering: `std::process::id()` once at startup, compared to each
//! process object's `kAudioProcessPropertyPID`. PID-based — bundle ID
//! independent, survives any future NBP rename.
//!
//! Requires macOS 14.4+. On older systems the property selectors return
//! errors and this module degrades gracefully (no events emitted).

use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter};

// --- Core Audio FFI ----------------------------------------------------------

#[repr(C)]
struct AudioObjectPropertyAddress {
    selector: u32,
    scope: u32,
    element: u32,
}

const K_AUDIO_OBJECT_SYSTEM_OBJECT: u32 = 1;
const K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL: u32 = u32::from_be_bytes(*b"glob");
const K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN: u32 = 0;

// macOS 14.4+ HAL Process API (from <CoreAudio/AudioHardware.h>):
const K_AUDIO_HARDWARE_PROPERTY_PROCESS_OBJECT_LIST: u32 = u32::from_be_bytes(*b"prs#");
const K_AUDIO_PROCESS_PROPERTY_PID: u32 = u32::from_be_bytes(*b"ppid");
const K_AUDIO_PROCESS_PROPERTY_BUNDLE_ID: u32 = u32::from_be_bytes(*b"pbid");
const K_AUDIO_PROCESS_PROPERTY_IS_RUNNING_INPUT: u32 = u32::from_be_bytes(*b"piri");

unsafe extern "C" {
    fn AudioObjectGetPropertyDataSize(
        object_id: u32,
        address: *const AudioObjectPropertyAddress,
        qualifier_data_size: u32,
        qualifier_data: *const c_void,
        out_data_size: *mut u32,
    ) -> i32;

    fn AudioObjectGetPropertyData(
        object_id: u32,
        address: *const AudioObjectPropertyAddress,
        qualifier_data_size: u32,
        qualifier_data: *const c_void,
        io_data_size: *mut u32,
        out_data: *mut c_void,
    ) -> i32;

    fn AudioObjectHasProperty(object_id: u32, address: *const AudioObjectPropertyAddress) -> bool;

    fn CFRelease(cf: *const c_void);
    fn CFGetTypeID(cf: *const c_void) -> usize;
    fn CFStringGetTypeID() -> usize;

    // libproc — short executable name for daemons that have no NSRunningApp entry.
    fn proc_name(pid: i32, buffer: *mut c_void, buffersize: u32) -> i32;
}

// --- Daemon → friendly-name mapping ------------------------------------------
//
// Only entries that NSRunningApplication won't resolve. Apps with a real Bundle
// (Zoom, Teams, Slack, browser tabs via Chrome helper) get their localizedName
// for free from macOS. This list stays tiny on purpose.
const DAEMON_LABELS: &[(&str, &str)] = &[
    ("avconferenced", "FaceTime"),
    ("callservicesd", "Phone Call"),
    ("continuitycaptured", "Continuity Audio"),
];

// Bundle ids for daemon-routed calls so the frontend can still resolve an app
// icon (NSWorkspace has no icon for the daemon executable itself). Keyed by the
// same proc name as DAEMON_LABELS.
const DAEMON_BUNDLE_IDS: &[(&str, &str)] = &[
    ("avconferenced", "com.apple.FaceTime"),
    ("callservicesd", "com.apple.FaceTime"),
];

// System / UI processes that legitimately open the audio input but are NOT
// calls — e.g. System Settings → Sound opens the mic to drive its input-level
// meter. Counting them as calls causes false auto-record (seen on a dev rebuild
// while the Sound pane was open) and a stream of `app_icons … UNRESOLVED` noise.
// A blanket `com.apple.*` skip is wrong: FaceTime/Phone route through daemons
// (avconferenced/callservicesd) that get mapped to com.apple.FaceTime, so we
// match exact bundle ids of known non-call system UI instead.
const IGNORED_INPUT_BUNDLE_IDS: &[&str] = &[
    "com.apple.Sound-Settings.extension", // Settings → Sound input-level meter
    "com.apple.controlcenter",            // Control Center menu-bar UI
    "com.apple.systempreferences",        // System Settings host process
];

// --- State -------------------------------------------------------------------

pub struct AudioProcessDetectorState {
    pub should_stop: Arc<AtomicBool>,
    pub thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl AudioProcessDetectorState {
    pub fn new() -> Self {
        Self {
            should_stop: Arc::new(AtomicBool::new(false)),
            thread_handle: Mutex::new(None),
        }
    }

    pub fn stop(&self) {
        self.should_stop.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.thread_handle.lock()
            && let Some(handle) = guard.take()
        {
            let _ = handle.join();
        }
    }
}

impl Drop for AudioProcessDetectorState {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Sync the detector to the desired enabled state. Hooked from
/// `config::save_settings` the same way `call_detector::sync_detector` is.
///
/// Holds the `thread_handle` mutex across check + spawn + store so a
/// concurrent enable can't race and orphan a second worker thread. Also
/// resets `should_stop` before spawning — otherwise a previous `stop()` left
/// `should_stop=true` and the new worker would exit on first tick.
pub fn sync_detector(state: &AudioProcessDetectorState, enabled: bool, app_handle: &AppHandle) {
    if enabled {
        let mut guard = match state.thread_handle.lock() {
            Ok(g) => g,
            Err(e) => {
                log::error!("audio_process_detector: thread_handle lock poisoned: {}", e);
                return;
            }
        };
        if guard.is_some() {
            return;
        }
        state.should_stop.store(false, Ordering::SeqCst);
        let should_stop = state.should_stop.clone();
        let app = app_handle.clone();
        let handle = thread::spawn(move || run_detector(app, should_stop));
        *guard = Some(handle);
    } else {
        state.stop();
    }
}

// --- Main detection loop -----------------------------------------------------

/// Per-process end-confirmation window — matches `call_detector`'s 5s
/// device-level confirmation. Defends against transient mic-release blips
/// (Zoom screen-share, FaceTime device switch, daemon restarts).
const END_CONFIRMATION: Duration = Duration::from_secs(5);

fn run_detector(app_handle: AppHandle, should_stop: Arc<AtomicBool>) {
    let self_pid = std::process::id() as i32;
    // Our own bundle id — used to skip a stale PREVIOUS NBP instance (same
    // bundle, different PID) that hasn't released the mic yet after a restart
    // (e.g. `cargo tauri dev` rebuild). PID-only self-filter misses it, so the
    // new instance would see the old one as a "call" and auto-record.
    let self_bundle = own_bundle_id();
    log::info!("audio_process_detector: self_bundle={:?}", self_bundle);

    // Currently-considered-active PIDs and their last-known labels. A PID
    // here means we have emitted `started` and not yet emitted `ended`.
    let mut tracked: HashMap<i32, String> = HashMap::new();

    // PIDs that disappeared from `current` and are now in their grace
    // period. If they reappear, we cancel; if they stay gone past
    // END_CONFIRMATION, we emit `ended`.
    let mut pending_end: HashMap<i32, std::time::Instant> = HashMap::new();

    log::info!(
        "audio_process_detector: started (self_pid={}, requires macOS 14.4+)",
        self_pid
    );

    // One-time diagnostic for pre-14.4 macOS where the HAL Process API
    // isn't available — explicit signal to support/debugging that this
    // module is intentionally idle.
    let api_available = hal_process_api_available();
    if !api_available {
        log::info!(
            "audio_process_detector: HAL Process API not available (requires macOS 14.4+) — degrading to legacy device-level detection only"
        );
    }

    while !should_stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(500));

        // Fast-exit on pre-14.4: enumerate returns [] anyway, but we skip
        // the whole tick to avoid noisy work.
        if !api_available {
            continue;
        }

        let current = snapshot_input_users(self_pid, self_bundle.as_deref());

        // (a) Started transitions — PIDs in `current` not yet `tracked`.
        //     Also catch PID reuse: same PID in both maps but different
        //     label means a process with the same numeric PID is now a
        //     different app (rare but real). Emit ended(old) then started(new).
        //
        // NB: previously we had a `maybe_skip_self_mic` shortcut here that
        // refused to insert into `tracked` while NBP was recording. That
        // caused permanent drift — if a tracked PID briefly dropped from
        // `current` while NBP held the mic, `tracked` was cleared by step
        // (c) and could never be re-populated, so the eventual real "ended"
        // transition was invisible. `call_session.handle_started` already
        // guards against double-recording via its self-mic check; we trust
        // that and always track here.
        for (pid, info) in &current {
            let label = &info.label;
            match tracked.get(pid) {
                None => {
                    log::info!(
                        "audio_process_detector: started pid={} app={:?} bundle={:?}",
                        pid,
                        label,
                        info.bundle_id
                    );
                    emit_call_event(
                        &app_handle,
                        "started",
                        Some(label),
                        info.bundle_id.as_deref(),
                    );
                    tracked.insert(*pid, label.clone());
                }
                Some(prev_label) if prev_label != label => {
                    // PID reuse with different app identity. Emit only the
                    // `ended` for the old app and drop the PID from tracked.
                    // Next tick will see the PID as new (None branch) and
                    // emit `started` for the new label. Splitting across two
                    // ticks avoids a dual-emit race in the event bus —
                    // call_session.handle_ended runs first and clears the
                    // active session cleanly before handle_started creates
                    // the next one.
                    log::info!(
                        "audio_process_detector: pid={} label changed {:?} -> {:?} (PID reuse — emit ended, next tick will emit started)",
                        pid,
                        prev_label,
                        label
                    );
                    emit_call_event(&app_handle, "ended", Some(prev_label), None);
                    tracked.remove(pid);
                }
                _ => {
                    // PID still active with same label — cancel any
                    // pending-end window if armed.
                    if pending_end.remove(pid).is_some() {
                        log::debug!(
                            "audio_process_detector: pending-end for pid={} cancelled (came back)",
                            pid
                        );
                    }
                }
            }
        }

        // (b) Arm pending-end for PIDs that were tracked but disappeared.
        let current_pids: HashSet<i32> = current.keys().copied().collect();
        for pid in tracked.keys() {
            if !current_pids.contains(pid) && !pending_end.contains_key(pid) {
                log::debug!(
                    "audio_process_detector: arming end-confirmation for pid={}",
                    pid
                );
                pending_end.insert(*pid, std::time::Instant::now());
            }
        }

        // (c) Fire ended for PIDs whose pending-end elapsed without comeback.
        let to_emit: Vec<i32> = pending_end
            .iter()
            .filter(|(_, t)| t.elapsed() >= END_CONFIRMATION)
            .map(|(pid, _)| *pid)
            .collect();
        for pid in to_emit {
            if let Some(label) = tracked.remove(&pid) {
                log::info!(
                    "audio_process_detector: ended pid={} app={:?} (confirmed >{}s)",
                    pid,
                    label,
                    END_CONFIRMATION.as_secs()
                );
                emit_call_event(&app_handle, "ended", Some(&label), None);
            }
            pending_end.remove(&pid);
        }
    }

    log::info!("audio_process_detector: stopped");
}

/// Probes whether the HAL Process API (`kAudioHardwarePropertyProcessObjectList`
/// and friends) is available on the running macOS. Required: 14.4 Sonoma +.
/// Used by `call_detector` to decide whether to defer to this module — when
/// the per-process API is here, device-level detection causes false-positive
/// "call ended" events the moment NBP cpal opens the input device, because
/// `kAudioDevicePropertyDeviceIsRunningSomewhere` can't distinguish "Zoom
/// using mic" from "NBP using mic". This module's per-process view solves it.
pub fn hal_process_api_available() -> bool {
    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_HARDWARE_PROPERTY_PROCESS_OBJECT_LIST,
        scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };
    unsafe { AudioObjectHasProperty(K_AUDIO_OBJECT_SYSTEM_OBJECT, &address) }
}

// maybe_skip_self_mic was removed — see comment at the top of the started
// transitions loop for why.

/// A process currently using audio input: a user-facing label plus the bundle
/// id (for icon resolution on the frontend).
#[derive(Clone)]
struct AppInfo {
    label: String,
    bundle_id: Option<String>,
}

/// Single snapshot of `pid → AppInfo` for all processes currently with
/// `kAudioProcessPropertyIsRunningInput = 1`, excluding `self_pid`.
///
/// IMPORTANT: read order is PID → BundleID → IsRunningInput. The cereal
/// (Granola-style Electron module) project found that querying `piri` BEFORE
/// `pbid` on a process object crashed CoreAudio on some macOS versions —
/// keep that lesson in our order too.
fn snapshot_input_users(self_pid: i32, self_bundle: Option<&str>) -> HashMap<i32, AppInfo> {
    let mut out = HashMap::new();
    for obj_id in enumerate_processes() {
        let pid = read_process_pid(obj_id);
        if pid <= 0 || pid == self_pid {
            continue;
        }
        let bundle_id = read_process_bundle_id(obj_id);
        // Skip any other NBP instance (same bundle id, different PID) — a stale
        // dev/restart copy must never be mistaken for a third-party call.
        if let (Some(sb), Some(bid)) = (self_bundle, bundle_id.as_deref())
            && sb == bid
        {
            continue;
        }
        // Skip system UI that opens the input but isn't a call (Sound settings
        // meter, Control Center, …) — otherwise it triggers false auto-record.
        if let Some(bid) = bundle_id.as_deref()
            && IGNORED_INPUT_BUNDLE_IDS.contains(&bid)
        {
            continue;
        }
        if !read_process_is_running_input(obj_id) {
            continue;
        }
        let label = resolve_label(pid, bundle_id.as_deref());
        // Daemon mapping takes precedence — same priority as resolve_label.
        // CoreAudio reports a non-app bundle id for daemon-routed calls
        // (avconferenced for FaceTime), which NSWorkspace can't resolve to an
        // icon, so the known com.apple.* mapping must win over it.
        let resolved_bundle = daemon_bundle_id(pid).or(bundle_id);
        out.insert(
            pid,
            AppInfo {
                label,
                bundle_id: resolved_bundle,
            },
        );
    }
    out
}

/// Map a daemon process to the bundle id of the user-facing app it routes for.
fn daemon_bundle_id(pid: i32) -> Option<String> {
    let short = process_short_name(pid)?;
    DAEMON_BUNDLE_IDS
        .iter()
        .find(|(proc, _)| *proc == short)
        .map(|(_, bid)| (*bid).to_string())
}

fn enumerate_processes() -> Vec<u32> {
    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_HARDWARE_PROPERTY_PROCESS_OBJECT_LIST,
        scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };

    // Pre-flight: AudioObjectHasProperty gives us a clean negative on
    // pre-14.4 macOS where this selector doesn't exist. Avoids a stream of
    // bogus error logs from AudioObjectGetPropertyDataSize.
    if !unsafe { AudioObjectHasProperty(K_AUDIO_OBJECT_SYSTEM_OBJECT, &address) } {
        return vec![];
    }

    let mut size: u32 = 0;
    let status = unsafe {
        AudioObjectGetPropertyDataSize(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            0,
            std::ptr::null(),
            &mut size,
        )
    };
    if status != 0 || size == 0 {
        return vec![];
    }

    let count = size as usize / std::mem::size_of::<u32>();
    let mut ids = vec![0u32; count];
    let mut io_size = size;
    let status = unsafe {
        AudioObjectGetPropertyData(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            0,
            std::ptr::null(),
            &mut io_size,
            ids.as_mut_ptr() as *mut c_void,
        )
    };
    if status != 0 {
        return vec![];
    }
    ids
}

fn read_process_pid(obj_id: u32) -> i32 {
    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_PROCESS_PROPERTY_PID,
        scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };
    let mut value: i32 = 0;
    let mut size = std::mem::size_of::<i32>() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(
            obj_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            &mut value as *mut i32 as *mut c_void,
        )
    };
    if status != 0 { -1 } else { value }
}

fn read_process_is_running_input(obj_id: u32) -> bool {
    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_PROCESS_PROPERTY_IS_RUNNING_INPUT,
        scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };
    let mut value: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(
            obj_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            &mut value as *mut u32 as *mut c_void,
        )
    };
    status == 0 && value != 0
}

/// `kAudioProcessPropertyBundleID` returns a `CFStringRef` that the caller
/// owns — must CFRelease. Empty / null for daemons.
fn read_process_bundle_id(obj_id: u32) -> Option<String> {
    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_PROCESS_PROPERTY_BUNDLE_ID,
        scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };
    let mut cf_string_ptr: *const c_void = std::ptr::null();
    let mut size = std::mem::size_of::<*const c_void>() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(
            obj_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            &mut cf_string_ptr as *mut *const c_void as *mut c_void,
        )
    };
    if status != 0 || cf_string_ptr.is_null() {
        return None;
    }
    // CoreAudio has been observed returning non-CFString data here on some
    // process objects (cereal project comment). Validate type-id before
    // treating as a CFString — otherwise CFRelease / NSString-bridge can
    // crash.
    let is_cfstring = unsafe { CFGetTypeID(cf_string_ptr) == CFStringGetTypeID() };
    if !is_cfstring {
        unsafe { CFRelease(cf_string_ptr) };
        return None;
    }
    // CFString ↔ NSString are toll-free bridged. Use UTF8String via objc2.
    let result = unsafe {
        let ns_obj = cf_string_ptr as *const objc2::runtime::AnyObject;
        let utf8: *const i8 = objc2::msg_send![ns_obj, UTF8String];
        if utf8.is_null() {
            None
        } else {
            std::ffi::CStr::from_ptr(utf8)
                .to_str()
                .ok()
                .map(String::from)
                .filter(|s| !s.is_empty())
        }
    };
    unsafe { CFRelease(cf_string_ptr) };
    result
}

// --- Friendly label resolution ----------------------------------------------

fn resolve_label(pid: i32, bundle_id: Option<&str>) -> String {
    // Priority 1: daemon mapping by process name. Checked FIRST because
    // NSRunningApplication can return the raw daemon executable name (e.g.
    // "avconferenced") for some system daemons — and we want to translate
    // those into user-facing labels before falling through to NSRunningApp.
    let short = process_short_name(pid);
    if let Some(ref s) = short {
        for (proc, label) in DAEMON_LABELS {
            if s == proc {
                return (*label).to_string();
            }
        }
    }
    // Priority 2: NSRunningApplication.localizedName — covers any sandboxed
    // user app with a real Bundle. Zoom, Teams, Slack, browsers all return
    // their proper display name here. No hardcoded list.
    if let Some(name) = nsrunning_application_localized_name(pid) {
        return name;
    }
    // Priority 3: short executable name (un-mapped daemon or unknown).
    if let Some(s) = short {
        return s;
    }
    // Priority 4: bundle ID from CoreAudio (raw, not pretty).
    if let Some(bid) = bundle_id {
        return bid.to_string();
    }
    // Priority 5: last resort.
    format!("pid {}", pid)
}

/// This process's own bundle id via `NSBundle.mainBundle`. Stable across NBP
/// instances (same app = same bundle), so it identifies a stale prior copy that
/// PID comparison can't.
fn own_bundle_id() -> Option<String> {
    use objc2::rc::autoreleasepool;
    use objc2::runtime::AnyClass;
    autoreleasepool(|_| unsafe {
        let cls = AnyClass::get(c"NSBundle")?;
        let main: *mut objc2::runtime::AnyObject = objc2::msg_send![cls, mainBundle];
        if main.is_null() {
            return None;
        }
        let bid: *mut objc2::runtime::AnyObject = objc2::msg_send![main, bundleIdentifier];
        if bid.is_null() {
            return None;
        }
        let utf8: *const i8 = objc2::msg_send![bid, UTF8String];
        if utf8.is_null() {
            return None;
        }
        std::ffi::CStr::from_ptr(utf8)
            .to_str()
            .ok()
            .map(String::from)
            .filter(|s| !s.is_empty())
    })
}

fn nsrunning_application_localized_name(pid: i32) -> Option<String> {
    use objc2::rc::autoreleasepool;
    use objc2::runtime::AnyClass;
    // `runningApplicationWithProcessIdentifier:` and `localizedName` return
    // autoreleased NSObjects. On a background polling thread there is no
    // ambient autorelease pool — without an explicit pool here they would
    // accumulate forever (memory leak). The Vec<String> we return is
    // copied/owned by Rust before the pool drains.
    autoreleasepool(|_| unsafe {
        let cls = AnyClass::get(c"NSRunningApplication")?;
        let app: *mut objc2::runtime::AnyObject =
            objc2::msg_send![cls, runningApplicationWithProcessIdentifier: pid];
        if app.is_null() {
            return None;
        }
        let name_obj: *mut objc2::runtime::AnyObject = objc2::msg_send![app, localizedName];
        if name_obj.is_null() {
            return None;
        }
        let utf8: *const i8 = objc2::msg_send![name_obj, UTF8String];
        if utf8.is_null() {
            return None;
        }
        std::ffi::CStr::from_ptr(utf8)
            .to_str()
            .ok()
            .map(String::from)
            .filter(|s| !s.is_empty())
    })
}

fn process_short_name(pid: i32) -> Option<String> {
    let mut buf = [0u8; 256];
    let written = unsafe { proc_name(pid, buf.as_mut_ptr() as *mut c_void, buf.len() as u32) };
    if written <= 0 {
        return None;
    }
    // Hard clamp against the buffer size. libc shouldn't return more than
    // `buffersize`, but Rust slicing panics on overshoot — defend explicitly
    // so a kernel/libc quirk never crashes the polling thread.
    let written_clamped = (written as usize).min(buf.len());
    let end = buf
        .iter()
        .take(written_clamped)
        .position(|&b| b == 0)
        .unwrap_or(written_clamped);
    std::str::from_utf8(&buf[..end])
        .ok()
        .map(String::from)
        .filter(|s| !s.is_empty())
}

// --- Event emit --------------------------------------------------------------

fn emit_call_event(
    app_handle: &AppHandle,
    stage: &str,
    call_app: Option<&str>,
    bundle_id: Option<&str>,
) {
    let payload = serde_json::json!({
        "stage": stage,
        "app": call_app,
        "bundle_id": bundle_id,
    });
    if let Err(e) = app_handle.emit("call-event", payload) {
        log::warn!("audio_process_detector: emit call-event failed: {}", e);
    }
}
