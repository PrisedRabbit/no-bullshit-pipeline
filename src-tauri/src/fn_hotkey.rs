//! Low-level Fn (🌐 Globe) hotkey capture for Quick Dictate.
//!
//! `tauri-plugin-global-shortcut` (Carbon `RegisterEventHotKey` under the hood)
//! cannot bind the Fn key — Fn is a modifier *flag*
//! (`kCGEventFlagMaskSecondaryFn = 0x800000`), not a keycode. So Fn-based
//! shortcuts ("fn", "fn+d", …) take a separate path: an active `CGEventTap`
//! running on a dedicated thread with its own CFRunLoop.
//!
//! Active tap (`kCGEventTapOptionDefault`) lets us *swallow* the base key of
//! `Fn+key` combos so it doesn't leak into the focused app. An active keyboard
//! tap requires the **Accessibility** permission — the same one already used for
//! the ⌘V auto-paste (`dictation::is_ax_trusted`).
//!
//! Mirrors the raw CoreGraphics FFI pattern from `dictation.rs`.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};

use crate::config::{DictationShortcut, DictationTriggerMode};
use crate::dictation::{self, DictationState};
use crate::ShortcutRegistration;

// --- CoreGraphics / CoreFoundation FFI ------------------------------------

type CGEventRef = *const c_void;
type CGEventTapProxy = *const c_void;
type CGEventTapCallBack =
    extern "C" fn(CGEventTapProxy, u32, CGEventRef, *mut c_void) -> CGEventRef;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> *const c_void; // CFMachPortRef
    fn CGEventTapEnable(tap: *const c_void, enable: bool);
    fn CGEventGetFlags(event: CGEventRef) -> u64;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
}

#[allow(non_upper_case_globals)]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: *const c_void,
        order: isize,
    ) -> *const c_void;
    fn CFRunLoopGetCurrent() -> *const c_void;
    fn CFRunLoopAddSource(rl: *const c_void, source: *const c_void, mode: *const c_void);
    fn CFRunLoopRun();
    static kCFRunLoopCommonModes: *const c_void;
}

const K_CG_SESSION_EVENT_TAP: u32 = 1;
const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

const K_CG_EVENT_KEY_DOWN: u32 = 10;
const K_CG_EVENT_KEY_UP: u32 = 11;
const K_CG_EVENT_FLAGS_CHANGED: u32 = 12;
const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;

const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;
const K_CG_EVENT_FLAG_MASK_SECONDARY_FN: u64 = 0x0080_0000;

// --- Registry / shared state ----------------------------------------------

#[derive(Clone)]
struct FnBinding {
    id: String,
    /// `None` = Fn solo; `Some(keycode)` = Fn + that key.
    base_keycode: Option<u16>,
    mode: DictationTriggerMode,
    is_clipboard: bool,
}

struct FnState {
    app: Option<AppHandle>,
    bindings: Vec<FnBinding>,
    /// Current Fn flag state, tracked across flagsChanged events.
    fn_down: bool,
    /// Shortcut currently held in push-to-talk: (id, base_keycode).
    ptt_active: Option<(String, Option<u16>)>,
}

lazy_static::lazy_static! {
    static ref FN_STATE: Mutex<FnState> = Mutex::new(FnState {
        app: None,
        bindings: Vec::new(),
        fn_down: false,
        ptt_active: None,
    });
}

/// Mach port of the live tap (as usize so it's Send). 0 = not created.
static TAP_PORT: AtomicUsize = AtomicUsize::new(0);
/// Whether the tap thread is up and the tap was created successfully.
static TAP_READY: AtomicBool = AtomicBool::new(false);
/// When true, the tap reports the pressed Fn accelerator to the frontend (for
/// the Settings hotkey field) instead of triggering dictation.
static CAPTURE_MODE: AtomicBool = AtomicBool::new(false);

// --- Public API -----------------------------------------------------------

/// Re-register all Fn-based shortcuts. Returns a status row per shortcut so the
/// UI can render the same badges as plugin shortcuts.
pub fn reload(app: &AppHandle, fn_shortcuts: &[DictationShortcut]) -> Vec<ShortcutRegistration> {
    let mut results = Vec::with_capacity(fn_shortcuts.len());
    let mut bindings = Vec::new();

    // Parse first so we know whether we need the tap at all.
    let parsed: Vec<(&DictationShortcut, Option<Option<u16>>)> = fn_shortcuts
        .iter()
        .map(|sc| (sc, parse_fn_hotkey(&sc.hotkey)))
        .collect();

    let need_tap = parsed.iter().any(|(_, spec)| spec.is_some());
    let tap_ok = if need_tap { ensure_tap_thread() } else { true };

    for (sc, spec) in &parsed {
        match spec {
            None => results.push(ShortcutRegistration {
                id: sc.id.clone(),
                hotkey: sc.hotkey.clone(),
                status: "error".into(),
                error: Some("Unsupported Fn hotkey".into()),
            }),
            Some(base) => {
                if !tap_ok {
                    results.push(ShortcutRegistration {
                        id: sc.id.clone(),
                        hotkey: sc.hotkey.clone(),
                        status: "error".into(),
                        error: Some("Accessibility permission needed for Fn key".into()),
                    });
                    continue;
                }
                bindings.push(FnBinding {
                    id: sc.id.clone(),
                    base_keycode: *base,
                    mode: sc.trigger_mode,
                    is_clipboard: matches!(
                        sc.input_source,
                        crate::config::DictationInputSource::Clipboard
                    ),
                });
                log::info!(
                    "dictation: registered Fn shortcut '{}' ({} → {})",
                    sc.name,
                    sc.hotkey,
                    sc.id
                );
                results.push(ShortcutRegistration {
                    id: sc.id.clone(),
                    hotkey: sc.hotkey.clone(),
                    status: "registered".into(),
                    error: None,
                });
            }
        }
    }

    if let Ok(mut st) = FN_STATE.lock() {
        st.app = Some(app.clone());
        st.bindings = bindings;
        st.ptt_active = None;
    }
    // Idle the tap when nothing is bound so we don't add latency to every
    // keystroke system-wide; re-enable when there's something to listen for
    // (bindings or capture mode).
    recompute_tap_enabled();

    results
}

/// Drop all Fn bindings and idle the tap (master toggle off / no fn shortcuts).
pub fn unregister_all() {
    if let Ok(mut st) = FN_STATE.lock() {
        st.bindings.clear();
        st.ptt_active = None;
    }
    recompute_tap_enabled();
}

/// Arm/disarm capture mode for the Settings hotkey field. While armed, the tap
/// reports the pressed Fn accelerator via the `fn_hotkey_captured` event instead
/// of triggering dictation. Returns whether the tap is live (false →
/// Accessibility not granted, so Fn can't be captured).
pub fn set_capture(app: &AppHandle, on: bool) -> bool {
    if let Ok(mut st) = FN_STATE.lock() {
        st.app = Some(app.clone());
        st.fn_down = false;
    }
    CAPTURE_MODE.store(on, Ordering::Relaxed);
    let live = if on { ensure_tap_thread() } else { true };
    recompute_tap_enabled();
    live
}

// --- Tap lifecycle --------------------------------------------------------

fn set_tap_enabled(enable: bool) {
    let port = TAP_PORT.load(Ordering::Relaxed);
    if port != 0 {
        unsafe { CGEventTapEnable(port as *const c_void, enable) };
    }
}

/// Enable the tap when there are bindings to dispatch or capture mode is armed;
/// idle it otherwise so we don't tax every system-wide keystroke.
fn recompute_tap_enabled() {
    let has_bindings = FN_STATE
        .lock()
        .map(|s| !s.bindings.is_empty())
        .unwrap_or(false);
    let want = has_bindings || CAPTURE_MODE.load(Ordering::Relaxed);
    set_tap_enabled(want && TAP_READY.load(Ordering::Relaxed));
}

/// Start the tap thread once. Returns true if the tap is live. Safe to call on
/// every reload — only the first successful call spins up the thread.
fn ensure_tap_thread() -> bool {
    if TAP_READY.load(Ordering::Relaxed) {
        return true;
    }
    // Bail early if the OS won't grant us an active keyboard tap anyway.
    if !dictation::is_ax_trusted() {
        return false;
    }

    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel::<usize>();
    std::thread::Builder::new()
        .name("fn-hotkey-tap".into())
        .spawn(move || unsafe {
            let mask = (1u64 << K_CG_EVENT_KEY_DOWN)
                | (1u64 << K_CG_EVENT_KEY_UP)
                | (1u64 << K_CG_EVENT_FLAGS_CHANGED);
            let port = CGEventTapCreate(
                K_CG_SESSION_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_DEFAULT,
                mask,
                tap_callback,
                std::ptr::null_mut(),
            );
            if port.is_null() {
                let _ = tx.send(0);
                return;
            }
            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), port, 0);
            let rl = CFRunLoopGetCurrent();
            CFRunLoopAddSource(rl, source, kCFRunLoopCommonModes);
            // Start disabled; reload() flips it on once bindings are installed.
            CGEventTapEnable(port, false);
            let _ = tx.send(port as usize);
            CFRunLoopRun(); // blocks this thread forever
        })
        .ok();

    let port = rx.recv().unwrap_or(0);
    if port != 0 {
        TAP_PORT.store(port, Ordering::Relaxed);
        TAP_READY.store(true, Ordering::Relaxed);
        true
    } else {
        false
    }
}

// --- The tap callback -----------------------------------------------------

extern "C" fn tap_callback(
    _proxy: CGEventTapProxy,
    type_: u32,
    event: CGEventRef,
    _user_info: *mut c_void,
) -> CGEventRef {
    // macOS disables the tap if our callback is slow or on certain user input;
    // re-enable and pass the event through.
    if type_ == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT
        || type_ == K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT
    {
        set_tap_enabled(true);
        return event;
    }

    let flags = unsafe { CGEventGetFlags(event) };
    let fn_now = (flags & K_CG_EVENT_FLAG_MASK_SECONDARY_FN) != 0;

    // Settings hotkey field is armed → report the Fn accelerator, don't dispatch.
    if CAPTURE_MODE.load(Ordering::Relaxed) {
        return capture_event(type_, event, fn_now);
    }

    let mut swallow = false;

    if let Ok(mut st) = FN_STATE.lock() {
        let Some(app) = st.app.clone() else {
            return event;
        };

        match type_ {
            K_CG_EVENT_FLAGS_CHANGED => {
                let was = st.fn_down;
                st.fn_down = fn_now;
                if !was && fn_now {
                    // Fn pressed → solo bindings only.
                    if let Some(b) = st.bindings.iter().find(|b| b.base_keycode.is_none()).cloned()
                    {
                        fire_down(&mut st, &app, &b);
                    }
                } else if was && !fn_now {
                    // Fn released → end any push-to-talk session (solo or combo).
                    if st.ptt_active.take().is_some() {
                        spawn_stop(&app);
                    }
                }
            }
            K_CG_EVENT_KEY_DOWN => {
                if fn_now {
                    let kc =
                        unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) }
                            as u16;
                    if let Some(b) = st
                        .bindings
                        .iter()
                        .find(|b| b.base_keycode == Some(kc))
                        .cloned()
                    {
                        swallow = true; // don't leak the base key into the app
                        fire_down(&mut st, &app, &b);
                    }
                }
            }
            K_CG_EVENT_KEY_UP => {
                let kc =
                    unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) }
                        as u16;
                if let Some((_, Some(active_kc))) = st.ptt_active.clone() {
                    if active_kc == kc {
                        st.ptt_active = None;
                        swallow = true;
                        spawn_stop(&app);
                    }
                }
            }
            _ => {}
        }
    }

    if swallow {
        std::ptr::null()
    } else {
        event
    }
}

/// Capture-mode handler: emit the pressed Fn accelerator to the frontend.
/// Fn alone → "fn"; Fn + key → "fn+<token>" (and swallow the key).
fn capture_event(type_: u32, event: CGEventRef, fn_now: bool) -> CGEventRef {
    let app = FN_STATE.lock().ok().and_then(|s| s.app.clone());
    let Some(app) = app else {
        return event;
    };
    match type_ {
        K_CG_EVENT_FLAGS_CHANGED => {
            let was = FN_STATE
                .lock()
                .map(|mut s| {
                    let w = s.fn_down;
                    s.fn_down = fn_now;
                    w
                })
                .unwrap_or(false);
            if !was && fn_now {
                let _ = app.emit("fn_hotkey_captured", "fn".to_string());
            }
            event
        }
        K_CG_EVENT_KEY_DOWN if fn_now => {
            let kc = unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) }
                as u16;
            if let Some(tok) = keycode_to_token(kc) {
                let _ = app.emit("fn_hotkey_captured", format!("fn+{}", tok));
            }
            std::ptr::null() // swallow so the base key doesn't act
        }
        _ => event,
    }
}

/// Dispatch the "pressed" edge for a binding, honoring its mode.
fn fire_down(st: &mut FnState, app: &AppHandle, b: &FnBinding) {
    if b.is_clipboard {
        // Clipboard shortcuts are one-shot regardless of mode.
        spawn_clipboard(app, &b.id);
        return;
    }
    match b.mode {
        DictationTriggerMode::Toggle => spawn_toggle(app, &b.id),
        DictationTriggerMode::PushToTalk => {
            if st.ptt_active.is_none() {
                st.ptt_active = Some((b.id.clone(), b.base_keycode));
                spawn_start(app, &b.id);
            }
        }
    }
}

// --- Async dispatch into the dictation engine -----------------------------

fn is_active(app: &AppHandle) -> bool {
    app.state::<DictationState>().is_active.load(Ordering::Relaxed)
}

fn spawn_toggle(app: &AppHandle, id: &str) {
    let app = app.clone();
    let id = id.to_string();
    tauri::async_runtime::spawn(async move {
        let result = if is_active(&app) {
            dictation::stop_inner(&app).await.map(|_| ())
        } else {
            dictation::start_inner(&app, &id).await
        };
        if let Err(e) = result {
            log::warn!("dictation Fn toggle '{}' failed: {}", id, e);
        }
    });
}

fn spawn_start(app: &AppHandle, id: &str) {
    if is_active(app) {
        return; // already recording — start_inner would just reject
    }
    let app = app.clone();
    let id = id.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = dictation::start_inner(&app, &id).await {
            log::warn!("dictation Fn start '{}' failed: {}", id, e);
        }
    });
}

fn spawn_stop(app: &AppHandle) {
    if !is_active(app) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = dictation::stop_inner(&app).await {
            log::warn!("dictation Fn stop failed: {}", e);
        }
    });
}

fn spawn_clipboard(app: &AppHandle, id: &str) {
    let app = app.clone();
    let id = id.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = dictation::run_clipboard_inner(&app, &id).await {
            log::warn!("dictation Fn clipboard '{}' failed: {}", id, e);
        }
    });
}

// --- Hotkey parsing -------------------------------------------------------

/// Parse an "fn" / "fn+<key>" accelerator.
/// Returns `Some(None)` for Fn solo, `Some(Some(keycode))` for Fn+key, and
/// `None` for anything unsupported.
fn parse_fn_hotkey(hotkey: &str) -> Option<Option<u16>> {
    let lower = hotkey.trim().to_lowercase();
    let parts: Vec<&str> = lower.split('+').map(|p| p.trim()).collect();
    match parts.as_slice() {
        ["fn"] => Some(None),
        ["fn", token] => hotkey_token_to_keycode(token).map(Some),
        _ => None,
    }
}

/// Map a frontend accelerator token (see `codeToHotkeyPart` in shortcuts.js) to
/// a macOS virtual keycode. Returns None for tokens we don't support yet.
fn hotkey_token_to_keycode(token: &str) -> Option<u16> {
    let kc: u16 = match token {
        // Letters (ANSI layout virtual keycodes)
        "a" => 0, "s" => 1, "d" => 2, "f" => 3, "h" => 4, "g" => 5,
        "z" => 6, "x" => 7, "c" => 8, "v" => 9, "b" => 11, "q" => 12,
        "w" => 13, "e" => 14, "r" => 15, "y" => 16, "t" => 17,
        "o" => 31, "u" => 32, "i" => 34, "p" => 35, "l" => 37, "j" => 38,
        "k" => 40, "n" => 45, "m" => 46,
        // Digit row
        "1" => 18, "2" => 19, "3" => 20, "4" => 21, "6" => 22, "5" => 23,
        "9" => 25, "7" => 26, "8" => 28, "0" => 29,
        // Punctuation
        "=" => 24, "-" => 27, "]" => 30, "[" => 33, "'" => 39, ";" => 41,
        "\\" => 42, "," => 43, "/" => 44, "." => 47, "`" => 50,
        // Whitespace / editing
        "space" => 49, "enter" => 36, "tab" => 48, "backspace" => 51,
        "delete" => 117, "home" => 115, "end" => 119, "pageup" => 116,
        "pagedown" => 121,
        // Arrows
        "left" => 123, "right" => 124, "down" => 125, "up" => 126,
        // Function keys
        "f1" => 122, "f2" => 120, "f3" => 99, "f4" => 118, "f5" => 96,
        "f6" => 97, "f7" => 98, "f8" => 100, "f9" => 101, "f10" => 109,
        "f11" => 103, "f12" => 111,
        _ => return None,
    };
    Some(kc)
}

/// Reverse of `hotkey_token_to_keycode`: virtual keycode → frontend token, so
/// capture mode can report "fn+<token>" in the same format Save expects.
fn keycode_to_token(kc: u16) -> Option<String> {
    let tok: &str = match kc {
        0 => "a", 1 => "s", 2 => "d", 3 => "f", 4 => "h", 5 => "g",
        6 => "z", 7 => "x", 8 => "c", 9 => "v", 11 => "b", 12 => "q",
        13 => "w", 14 => "e", 15 => "r", 16 => "y", 17 => "t",
        31 => "o", 32 => "u", 34 => "i", 35 => "p", 37 => "l", 38 => "j",
        40 => "k", 45 => "n", 46 => "m",
        18 => "1", 19 => "2", 20 => "3", 21 => "4", 22 => "6", 23 => "5",
        25 => "9", 26 => "7", 28 => "8", 29 => "0",
        24 => "=", 27 => "-", 30 => "]", 33 => "[", 39 => "'", 41 => ";",
        42 => "\\", 43 => ",", 44 => "/", 47 => ".", 50 => "`",
        49 => "space", 36 => "enter", 48 => "tab", 51 => "backspace",
        117 => "delete", 115 => "home", 119 => "end", 116 => "pageup",
        121 => "pagedown",
        123 => "left", 124 => "right", 125 => "down", 126 => "up",
        122 => "f1", 120 => "f2", 99 => "f3", 118 => "f4", 96 => "f5",
        97 => "f6", 98 => "f7", 100 => "f8", 101 => "f9", 109 => "f10",
        103 => "f11", 111 => "f12",
        _ => return None,
    };
    Some(tok.to_string())
}
