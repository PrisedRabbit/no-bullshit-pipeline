//! Quick Dictate HUD window — the small always-on-top overlay that shows
//! recording / transcribing / paste status. This module owns the window's
//! construction and on-screen placement; all in-session visuals (theme, level
//! meter, fade-out) live in the frontend `dictation-hud.{html,js}`.

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// Fixed HUD window size in logical points — single source of truth for both the
/// window builder and the centring math. The window is transparent and larger
/// than the visible card on purpose: the extra margin is room for the card's CSS
/// drop shadow (the native window shadow is off), so it isn't clipped.
const HUD_W: f64 = 320.0;
const HUD_H: f64 = 140.0;

/// True if a physical (pixel) global point lies inside a monitor's bounds.
fn monitor_contains(m: &tauri::Monitor, x: f64, y: f64) -> bool {
    let p = m.position();
    let s = m.size();
    let (left, top) = (p.x as f64, p.y as f64);
    x >= left && x < left + s.width as f64 && y >= top && y < top + s.height as f64
}

/// Reposition the HUD over the monitor that currently hosts the mouse cursor,
/// near the top centre. Multi-display users expect dictation overlays on the
/// screen they're working on, not the primary monitor. Called from dictation.rs
/// just before emitting the first `recording` / `reading_clipboard` event.
pub fn reposition(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("dictation-hud") else {
        return;
    };
    let Ok(monitors) = window.available_monitors() else {
        return;
    };
    if monitors.is_empty() {
        return;
    }

    let cursor = window.cursor_position().ok();
    let monitor = cursor
        .and_then(|c| monitors.iter().find(|m| monitor_contains(m, c.x, c.y)))
        .or_else(|| monitors.first());
    let Some(monitor) = monitor else {
        return;
    };

    let scale = monitor.scale_factor();
    let mx = monitor.position().x as f64 / scale;
    let my = monitor.position().y as f64 / scale;
    let mw = monitor.size().width as f64 / scale;
    let mh = monitor.size().height as f64 / scale;
    let x = mx + (mw - HUD_W) / 2.0;
    let y = my + mh * 0.12;
    let _ = window.set_position(tauri::LogicalPosition::new(x, y));
    // Respect the "Show HUD" dictation setting. When off, the overlay stays
    // hidden — dictation still records / transcribes / pastes, it just runs
    // silently with no on-screen window. When on (default), the first call
    // after launch reveals the window (built hidden off-screen to dodge the
    // transparent-WebView startup flash); later calls no-op.
    if crate::config::load_settings().dictation.show_hud {
        let _ = window.show();
    } else {
        let _ = window.hide();
    }
}

/// Build the HUD window once. Born hidden + click-through; the frontend and
/// `reposition` reveal and place it on the first dictation start.
pub(crate) fn build(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if app.get_webview_window("dictation-hud").is_some() {
        log::info!("dictation-hud: window already exists, skip");
        return Ok(());
    }
    // HUD lifecycle: built hidden (visible=false) so its transparent WebView
    // doesn't paint a black rectangle over whatever's on screen during the
    // initial paint pass. `reposition` positions and shows it on first dictation
    // start; in-session show/hide is the CSS `body.hidden` class toggle from JS.
    let mut builder = WebviewWindowBuilder::new(
        app,
        "dictation-hud",
        WebviewUrl::App("dictation-hud.html".into()),
    )
    .title("")
    .inner_size(HUD_W, HUD_H)
    .position(0.0, 0.0)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .accept_first_mouse(true)
    .visible(false)
    .resizable(false)
    .shadow(false);

    #[cfg(debug_assertions)]
    {
        builder = builder.devtools(true);
    }

    match builder.build() {
        Ok(_) => log::info!("dictation-hud: window created"),
        Err(e) => {
            log::error!("dictation-hud: window build failed: {}", e);
            return Err(Box::new(e));
        }
    }

    if let Some(window) = app.get_webview_window("dictation-hud") {
        // Float across all macOS Spaces so the HUD stays visible when the user
        // switches desktops / Mission Control during a dictation session.
        if let Err(e) = window.set_visible_on_all_workspaces(true) {
            log::warn!("dictation-hud: set_visible_on_all_workspaces failed: {}", e);
        }

        // Borderless NSWindows default to isMovable = false on macOS; without
        // these flags neither CSS `-webkit-app-region: drag` nor JS
        // startDragging() actually moves the window. Flip them on directly via
        // the NSWindow handle.
        //
        // Also pin ignoresMouseEvents=true at build time so the HUD is born
        // click-through. JS toggles it false only when entering active states
        // (recording / error with an action button). This is the OS-level belt
        // that backstops the CSS body.hidden / window.hide() path — if either
        // layer races, the NSWindow itself still refuses clicks.
        #[cfg(target_os = "macos")]
        unsafe {
            if let Ok(ns_window_ptr) = window.ns_window() {
                let ns_window: *mut objc2::runtime::AnyObject =
                    ns_window_ptr as *mut objc2::runtime::AnyObject;
                if !ns_window.is_null() {
                    let _: () = objc2::msg_send![ns_window, setMovable: true];
                    let _: () = objc2::msg_send![ns_window, setMovableByWindowBackground: true];
                    let _: () = objc2::msg_send![ns_window, setIgnoresMouseEvents: true];
                }
            }
        }

        // Initial position: cursor monitor, top-center.
        reposition(app);
    }
    Ok(())
}
