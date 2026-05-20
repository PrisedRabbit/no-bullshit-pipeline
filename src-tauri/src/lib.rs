//! # NBP (No Bullshit Pipeline)
//!
//! Privacy-first macOS audio capture application built with Tauri 2.
//! Records microphone and system audio simultaneously, processes with
//! AI-powered transcription, and pipes results through configurable pipelines.
//!
//! ## Module Overview
//!
//! ### Core Audio
//! - [`audio`] - Recording lifecycle (start/stop/pause/resume) and state management
//! - [`mic_audio`] - Microphone capture via cpal
//! - [`system_audio`] - System audio capture via Core Audio Process Taps (cidre)
//! - [`audio_processing`] - EBU R128 normalization, real-time mixing, shared buffers
//! - [`playback`] - In-app audio playback via rodio
//! - [`waveform`] - Waveform data extraction for visualization
//! - [`devices`] - Audio input device enumeration
//!
//! ### AI & Transcription
//! - [`transcription`] - FluidAudio (batch via sidecar) and cloud transcription (OpenAI / Google / Anthropic)
//! - [`cloud_ai`] - Cloud AI providers (OpenAI, Google, Anthropic)
//! - [`templates`] - Legacy output templates for structured extraction
//!
//! ### Pipeline System
//! - [`pipelines`] - Pipeline definition model and validation
//! - [`pipeline_engine`] - Sequential step execution engine with progress events
//! - [`connectors`] - Built-in connectors (LLM, Save, Webhook)
//! - [`prompt_templates`] - Reusable prompt template registry
//! - [`transcript_migration`] - Migration from plain text to frontmatter transcript format
//!
//! ### Infrastructure
//! - [`storage`] - Recording metadata, filesystem layout, CRUD operations
//! - [`config`] - App settings, API key management
//! - [`permissions`] - macOS permission checks (microphone, screen recording)

mod audio;
pub mod audio_processing;
pub mod storage;
mod system_audio;
mod mic_audio;
mod permissions;
pub mod config;
pub mod transcription;
mod cloud_ai;
mod templates;
pub mod playback;
mod waveform;
mod devices;
pub mod pipelines;
mod prompt_templates;
mod connectors;
mod pipeline_engine;
mod transcript_migration;
mod integrations;
pub mod local_llm;
pub mod realtime_transcription;
mod call_detector;
mod call_session;
mod audio_process_detector;
mod app_icons;
mod dictation;
mod dictation_streaming;
use audio::AudioState;
use transcription::TranscriptionState;
use tauri::menu::{AboutMetadataBuilder, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn log_from_js(msg: String) {
    log::info!("[js] {}", msg);
}

/// True on macOS 26.0+ (Tahoe) — the OS version that ships the SpeechAnalyzer
/// framework powering the Apple Speech provider. Used by the frontend to hide
/// the dropdown option on older systems.
#[tauri::command]
fn has_apple_speech() -> bool {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output();
        if let Ok(o) = out {
            if let Ok(s) = String::from_utf8(o.stdout) {
                if let Some(major) = s.trim().split('.').next() {
                    if let Ok(n) = major.parse::<u32>() {
                        return n >= 26;
                    }
                }
            }
        }
        false
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[tauri::command]
fn get_audio_level() -> f32 {
    // Return max of mic and system audio levels
    let mic_level = mic_audio::get_current_audio_level();
    let system_level = system_audio::get_system_audio_level();
    mic_level.max(system_level)
}

#[derive(serde::Serialize)]
struct AudioLevels {
    mic: f32,
    system: f32,
}

#[tauri::command]
fn get_audio_levels() -> AudioLevels {
    AudioLevels {
        mic: mic_audio::get_current_audio_level(),
        system: system_audio::get_system_audio_level(),
    }
}

pub fn run() {
    // Init logging so log::info!/error! prints to stderr
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("nbp=debug"))
        .init();

    // Disable App Nap. Without this macOS throttles backgrounded tray apps
    // and Carbon HotKey events get queued for tens of seconds (or minutes)
    // before delivery — the user presses cmd+shift+b and the HUD only shows
    // up much later. `NSAppSleepDisabled` in Info.plist works for signed
    // bundles but not dev binaries, so we ask runtime-style. Token must
    // outlive the process; leak it.
    #[cfg(target_os = "macos")]
    {
        use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};
        let info = NSProcessInfo::processInfo();
        let reason = NSString::from_str("Global hotkey listener — Quick Dictate");
        let token = info.beginActivityWithOptions_reason(
            NSActivityOptions::UserInitiated,
            &reason,
        );
        std::mem::forget(token);
    }

    // Load settings for managed state
    let settings = std::sync::Arc::new(std::sync::Mutex::new(config::load_settings()));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus the existing window when a second instance is launched
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        // tauri-plugin-window-state removed: on fractionally-scaled displays
        // it loses the scale factor every save→restore cycle, shrinking the
        // window ~20% each launch. The window now always opens at the size
        // configured in tauri.conf.json — stable and predictable.
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .manage(AudioState::new())
        .manage(TranscriptionState::new())
        .manage(dictation::DictationState::new())
        .manage(permissions::PermissionsStateCache(std::sync::Arc::new(std::sync::Mutex::new(
            permissions::PermissionsState::default()
        ))))
        .manage(settings)
        .setup(|app| {
            // Ensure the app appears in Dock and Cmd+Tab
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);

            // Create custom menu with only NBP submenu (no File, Edit, etc.)
            let about_metadata = AboutMetadataBuilder::new()
                .name(Some("NBP"))
                .version(Some(env!("CARGO_PKG_VERSION")))
                .copyright(Some("© 2024-2026"))
                .comments(Some("No Bullshit Pipeline - Audio recording and transcription"))
                .build();

            let app_submenu = SubmenuBuilder::new(app, "NBP")
                .about(Some(about_metadata))
                .separator()
                .services()
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?;

            let edit_submenu = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let window_submenu = SubmenuBuilder::new(app, "Window")
                .minimize()
                .close_window()
                .build()?;

            let menu = MenuBuilder::new(app)
                .item(&app_submenu)
                .item(&edit_submenu)
                .item(&window_submenu)
                .build()?;

            app.set_menu(menu)?;

            // Run transcript migration on startup
            transcript_migration::run_migration_if_needed();

            // Auto-check model freshness if >24h since last check
            let settings = config::load_settings();
            let now = chrono::Utc::now().timestamp();
            let needs_check = match settings.last_model_freshness_check {
                Some(last) => now - last > 86400,
                None => true,
            };
            if needs_check {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    local_llm::auto_check_model_freshness(handle).await;
                });
            }

            // Request notification permission if auto-record is enabled
            if settings.auto_record_meetings {
                use tauri_plugin_notification::NotificationExt;
                use tauri::plugin::PermissionState;
                if app.notification().permission_state()
                    .unwrap_or(PermissionState::Prompt) != PermissionState::Granted
                {
                    let _ = app.notification().request_permission();
                }
            }

            // Set up UNUserNotificationCenter delegate for handling notification clicks
            call_detector::init_notification_delegate(app.handle());

            // Initialize call detector state (always managed, started conditionally)
            let detector = if settings.auto_record_meetings {
                Some(call_detector::CallDetector::start(app.handle().clone()))
            } else {
                None
            };
            app.manage(call_detector::CallDetectorState(std::sync::Mutex::new(detector)));
            app.manage(call_session::CallSessionState::new());
            call_session::install(app.handle());

            // Heal any recordings stuck in `status="processing"` from a
            // previous crash / force-quit during finalization. Without this
            // they would display as "Processing..." forever.
            let repaired = storage::cleanup_stuck_processing_recordings();
            if repaired > 0 {
                log::warn!(
                    "startup: repaired {} stuck-in-processing recording(s)",
                    repaired
                );
            }

            // HAL per-process audio detector (macOS 14.4+). Runs alongside
            // the legacy device-level call_detector — both emit `call-event`
            // and call_session dedups via the existing overwrite policy.
            // Required for FaceTime detection (avconferenced daemon path
            // doesn't flip device-level DeviceIsRunningSomewhere).
            let process_detector = audio_process_detector::AudioProcessDetectorState::new();
            audio_process_detector::sync_detector(
                &process_detector,
                settings.auto_record_meetings,
                app.handle(),
            );
            app.manage(process_detector);

            // Unify the macOS title bar with our in-app top bar: extend the
            // webview content under the title bar (fullSizeContentView) and make
            // the native title bar transparent + title hidden. The `titleBarStyle`
            // config alone didn't reliably enable fullSizeContentView on this
            // wry/tao version, so force it here on the live NSWindow.
            #[cfg(target_os = "macos")]
            apply_unified_titlebar(app.handle());

            // Build system tray menu
            build_tray(app)?;

            // Quick Dictate floating HUD: small always-on-top overlay window
            // shown during dictation. Repositioned to the cursor's monitor
            // on each session start (see dictation::start_inner).
            build_dictation_hud(app.handle())?;

            // Call detector debug popup: tiny overlay that flashes on each
            // call start/end transition. Auto-hides from JS. See call_detector.
            build_call_popup(app.handle())?;

            // Quick Dictate: init plugin (always, even if disabled — so reload works
            // at runtime without restart). Shortcuts are then registered if enabled.
            #[cfg(desktop)]
            {
                let init = app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new().build(),
                );
                if let Err(e) = init {
                    log::warn!("global-shortcut plugin init failed: {}", e);
                } else if settings.dictation.enabled {
                    match reload_dictation_shortcuts(app.handle()) {
                        Ok(results) => {
                            for r in &results {
                                if r.status != "registered" {
                                    log::warn!(
                                        "dictation startup: {} ({}) → {} {}",
                                        r.id,
                                        r.hotkey,
                                        r.status,
                                        r.error.clone().unwrap_or_default()
                                    );
                                }
                            }
                        }
                        Err(e) => log::warn!("dictation: initial registration failed: {}", e),
                    }
                }
            }

            // Streaming pre-warm is disabled while streaming itself is off.
            // Instead, prewarm the batch sidecar models so the user's first
            // dictation doesn't pay the CoreML compile + cold-disk penalty.
            dictation::prewarm_models(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            log_from_js,
            app_icons::get_app_icon,
            has_apple_speech,
            audio::start_recording,
            audio::stop_recording,
            audio::pause_recording,
            audio::resume_recording,
            storage::list_recordings,
            storage::read_metadata,
            storage::update_tags,
            storage::update_title,
            storage::delete_recording,
            storage::list_projects,
            storage::save_projects,
            permissions::check_permissions,
            permissions::request_mic_permission,
            permissions::request_system_audio_permission,
            permissions::open_privacy_settings,
            config::load_settings,
            config::save_settings,
            transcription::transcribe_recording,
            transcription::is_transcribing,
            transcription::get_transcript,
            transcription::export_transcript_md,
            transcription::summarize_recording,
            transcription::process_with_template,
            templates::list_templates,
            templates::get_template,
            playback::play_audio,
            playback::pause_audio,
            playback::resume_audio,
            playback::stop_audio,
            playback::seek_audio,
            playback::get_playback_state,
            waveform::get_waveform_data,
            devices::get_input_devices,
            get_audio_level,
            get_audio_levels,
            // Pipeline system
            pipelines::list_pipelines,
            pipelines::get_pipeline,
            pipelines::save_pipeline,
            pipelines::delete_pipeline,
            // Prompt templates
            prompt_templates::list_prompt_templates,
            prompt_templates::get_prompt_template,
            prompt_templates::save_prompt_template,
            prompt_templates::delete_prompt_template,
            // Pipeline execution
            pipeline_engine::execute_pipeline,
            pipeline_engine::get_pipeline_status,
            pipeline_engine::get_all_pipeline_states,
            pipeline_engine::get_step_outputs,
            pipeline_engine::assign_pipeline,
            pipeline_engine::remove_pipeline_run,
            connectors::cli_agent::check_cli_availability,
            // Integrations
            integrations::list_slack_integrations,
            integrations::add_slack_integration,
            integrations::remove_slack_integration,
            integrations::test_slack_integration,
            integrations::list_slack_channels,
            integrations::list_slack_members,
            // Notion integrations
            integrations::notion::add_notion_integration,
            integrations::notion::list_notion_databases,
            integrations::notion::sync_notion_schema,
            integrations::notion::update_notion_people_mappings,
            integrations::notion::test_notion_integration,
            integrations::notion::remove_notion_integration,
            integrations::notion::list_notion_profiles,
            // Save path integrations
            integrations::save_path::add_save_path_integration,
            integrations::save_path::list_save_path_integrations,
            integrations::save_path::update_save_path_integration,
            integrations::save_path::remove_save_path_integration,
            // Linear integrations
            integrations::linear::add_linear_integration,
            integrations::linear::list_linear_teams,
            integrations::linear::sync_linear_schema,
            integrations::linear::test_linear_integration,
            integrations::linear::remove_linear_integration,
            integrations::linear::list_linear_profiles,
            integrations::linear::update_linear_member_aliases,
            // Webhook integrations
            integrations::webhook::add_webhook_integration,
            integrations::webhook::list_webhook_profiles,
            integrations::webhook::remove_webhook_integration,
            integrations::webhook::test_webhook_integration,
            // Local LLM
            local_llm::get_llm_models_info,
            local_llm::refresh_llm_model_sizes,
            local_llm::download_llm_model,
            local_llm::cancel_llm_download,
            local_llm::delete_llm_model,
            local_llm::check_model_freshness,
            local_llm::check_all_llm_freshness,
            local_llm::cancel_llm_freshness,
            local_llm::get_cached_freshness_results,
            // Provider models
            cloud_ai::models::fetch_provider_models,
            // Real-time transcription
            audio::start_realtime_transcription,
            audio::stop_realtime_transcription,
            // Call detection
            call_detector::test_call_notification,
            // Quick Dictate
            dictation::dictation_start,
            dictation::dictation_stop,
            dictation::dictation_toggle,
            dictation::dictation_is_active,
            dictation::dictation_cancel,
            dictation::dictation_reload_shortcuts,
            dictation::dictation_get_registration_status,
            dictation::dictation_release_esc,
            open_accessibility_settings,
            is_accessibility_granted,
            request_accessibility_permission,
            restart_app,
            set_hud_clickthrough,
            set_call_popup_clickthrough,
            call_session::ignore_call_recording,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Hide window instead of closing — tray keeps the app alive
                api.prevent_close();
                let _ = window.hide();
                // Hide from dock too
                #[cfg(target_os = "macos")]
                {
                    let app = window.app_handle();
                    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Per-shortcut registration result returned to the frontend so it can show
/// inline status badges (✓ / ⚠ conflict with another app / disabled).
#[derive(serde::Serialize, Clone, Debug)]
pub struct ShortcutRegistration {
    pub id: String,
    pub hotkey: String,
    /// "registered" — bound OK; "disabled" — master toggle off; "error" — OS rejected
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Unregister all current dictation hotkeys and re-register from the live settings.
/// Returns a per-shortcut status array so the UI can render conflicts.
#[cfg(desktop)]
pub fn reload_dictation_shortcuts(
    app: &tauri::AppHandle,
) -> Result<Vec<ShortcutRegistration>, String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let settings = config::load_settings();
    let mut results: Vec<ShortcutRegistration> = Vec::with_capacity(settings.dictation.shortcuts.len());
    // Cache the result so the frontend can poll status without re-registering
    let cache_arc = app.state::<dictation::DictationState>();

    if !settings.dictation.enabled {
        log::info!("dictation: disabled — no shortcuts registered");
        for sc in &settings.dictation.shortcuts {
            results.push(ShortcutRegistration {
                id: sc.id.clone(),
                hotkey: sc.hotkey.clone(),
                status: "disabled".into(),
                error: None,
            });
        }
        if let Ok(mut cache) = cache_arc.last_registration.lock() {
            *cache = results.clone();
        }
        return Ok(results);
    }

    for sc in &settings.dictation.shortcuts {
        if sc.hotkey.trim().is_empty() {
            results.push(ShortcutRegistration {
                id: sc.id.clone(),
                hotkey: sc.hotkey.clone(),
                status: "error".into(),
                error: Some("Empty hotkey".into()),
            });
            continue;
        }
        let sid = sc.id.clone();
        let name = sc.name.clone();
        let hotkey = sc.hotkey.clone();
        let is_clipboard_source =
            matches!(sc.input_source, config::DictationInputSource::Clipboard);
        let result = gs.on_shortcut(hotkey.as_str(), move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let app = app.clone();
                let sid = sid.clone();
                tauri::async_runtime::spawn(async move {
                    if is_clipboard_source {
                        // One-shot: snapshot clipboard → pipeline → paste. No toggle.
                        if let Err(e) = dictation::run_clipboard_inner(&app, &sid).await {
                            log::warn!("dictation clipboard '{}' failed: {}", sid, e);
                        }
                        return;
                    }
                    let active = app
                        .state::<dictation::DictationState>()
                        .is_active
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let result = if active {
                        dictation::stop_inner(&app).await.map(|_| ())
                    } else {
                        dictation::start_inner(&app, &sid).await
                    };
                    if let Err(e) = result {
                        log::warn!("dictation toggle for '{}' failed: {}", sid, e);
                    }
                });
            }
        });
        match result {
            Ok(()) => {
                log::info!("dictation: registered '{}' ({} → {})", name, hotkey, sc.id);
                results.push(ShortcutRegistration {
                    id: sc.id.clone(),
                    hotkey: hotkey.clone(),
                    status: "registered".into(),
                    error: None,
                });
            }
            Err(e) => {
                let msg = e.to_string();
                log::warn!("dictation: failed to register '{}' ({}): {}", name, hotkey, msg);
                results.push(ShortcutRegistration {
                    id: sc.id.clone(),
                    hotkey: hotkey.clone(),
                    status: "error".into(),
                    error: Some(msg),
                });
            }
        }
    }
    if let Ok(mut cache) = cache_arc.last_registration.lock() {
        *cache = results.clone();
    }
    Ok(results)
}

/// Open System Settings → Privacy & Security → Accessibility so the user can
/// grant the permission required for the ⌘V auto-paste keystroke.
#[tauri::command]
fn open_accessibility_settings() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .status()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Check whether NBP is in the macOS Accessibility allow-list. Lightweight,
/// no UI prompt. Backed by ApplicationServices' AXIsProcessTrusted.
#[tauri::command]
fn is_accessibility_granted() -> bool {
    dictation::is_ax_trusted()
}

/// Trigger the system Accessibility-permission prompt for NBP. Returns the
/// current trusted state. macOS only shows the dialog once; afterwards this
/// just returns the cached status.
#[tauri::command]
fn request_accessibility_permission() -> bool {
    dictation::request_ax_prompt()
}

/// Cleanly restart NBP. Useful after granting Accessibility permission since
/// macOS caches the `AXIsProcessTrusted` result per-process — without a
/// restart the running app keeps thinking it's untrusted even after the user
/// flipped the toggle in System Settings.
#[tauri::command]
fn restart_app(app: tauri::AppHandle) {
    app.restart();
}

/// Extend the webview under the macOS title bar (fullSizeContentView) and make
/// the native title bar transparent with a hidden title, so the traffic-light
/// strip shares our in-app top bar's color — one continuous bar, no seam.
/// Forced on the live NSWindow because the `titleBarStyle` config alone didn't
/// enable fullSizeContentView on this wry/tao version.
#[cfg(target_os = "macos")]
fn apply_unified_titlebar(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        log::warn!("apply_unified_titlebar: main window not found");
        return;
    };
    unsafe {
        if let Ok(ns_window_ptr) = window.ns_window() {
            let ns_window: *mut objc2::runtime::AnyObject =
                ns_window_ptr as *mut objc2::runtime::AnyObject;
            if ns_window.is_null() {
                return;
            }
            // NSWindowStyleMaskFullSizeContentView = 1 << 15
            const FULL_SIZE_CONTENT_VIEW: usize = 1 << 15;
            let mask: usize = objc2::msg_send![ns_window, styleMask];
            let _: () = objc2::msg_send![ns_window, setStyleMask: mask | FULL_SIZE_CONTENT_VIEW];
            let _: () = objc2::msg_send![ns_window, setTitlebarAppearsTransparent: true];
            // NSWindowTitleVisibility::Hidden = 1
            let _: () = objc2::msg_send![ns_window, setTitleVisibility: 1isize];

            // Attach an empty unified toolbar so the title bar grows taller and
            // the traffic lights drop to vertically center within our taller top
            // bar. Persistent across resize/focus — unlike repositioning the
            // standard buttons by hand, which AppKit resets on relayout.
            if let Some(toolbar_cls) = objc2::runtime::AnyClass::get(c"NSToolbar") {
                let toolbar: *mut objc2::runtime::AnyObject = objc2::msg_send![toolbar_cls, alloc];
                let toolbar: *mut objc2::runtime::AnyObject = objc2::msg_send![toolbar, init];
                if !toolbar.is_null() {
                    let _: () = objc2::msg_send![toolbar, setShowsBaselineSeparator: false];
                    let _: () = objc2::msg_send![ns_window, setToolbar: toolbar];
                    // NSWindowToolbarStyle::Unified = 3
                    let _: () = objc2::msg_send![ns_window, setToolbarStyle: 3isize];
                }
            }
        }
    }
}

/// Toggle whether the dictation HUD NSWindow swallows mouse events.
/// Called from dictation-hud.js: false when entering an interactive state
/// (recording / accessibility-error with a button), true when idling out.
/// This is the OS-level guard — even if the webview happens to be visible,
/// NSWindow.ignoresMouseEvents=true means clicks pass straight through to
/// whatever's behind, so no focus stealing.
#[tauri::command]
fn set_hud_clickthrough(app: tauri::AppHandle, clickthrough: bool) {
    let Some(window) = app.get_webview_window("dictation-hud") else {
        return;
    };
    #[cfg(target_os = "macos")]
    unsafe {
        if let Ok(ns_window_ptr) = window.ns_window() {
            let ns_window: *mut objc2::runtime::AnyObject =
                ns_window_ptr as *mut objc2::runtime::AnyObject;
            if !ns_window.is_null() {
                let _: () = objc2::msg_send![ns_window, setIgnoresMouseEvents: clickthrough];
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.set_ignore_cursor_events(clickthrough);
    }
}

/// Mirror of `set_hud_clickthrough` for the call-popup window. JS calls this
/// when the popup becomes visible (clickthrough=false → user can drag) and
/// again when it auto-hides (clickthrough=true → idle window doesn't eat
/// clicks meant for whatever app is below).
#[tauri::command]
fn set_call_popup_clickthrough(app: tauri::AppHandle, clickthrough: bool) {
    let Some(window) = app.get_webview_window("call-popup") else {
        return;
    };
    #[cfg(target_os = "macos")]
    unsafe {
        if let Ok(ns_window_ptr) = window.ns_window() {
            let ns_window: *mut objc2::runtime::AnyObject =
                ns_window_ptr as *mut objc2::runtime::AnyObject;
            if !ns_window.is_null() {
                let _: () = objc2::msg_send![ns_window, setIgnoresMouseEvents: clickthrough];
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.set_ignore_cursor_events(clickthrough);
    }
}

/// Reposition the HUD over the monitor that currently hosts the mouse cursor.
/// macOS users with multiple displays expect dictation overlays to appear on
/// the screen they're working on, not on the primary monitor by default.
/// Called by dictation.rs just before emitting the first `recording` /
/// `reading_clipboard` event each session.
pub fn reposition_dictation_hud(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("dictation-hud") else {
        return;
    };
    let Ok(monitors) = window.available_monitors() else {
        return;
    };
    if monitors.is_empty() {
        return;
    }
    // Cursor returns a PhysicalPosition in global (multi-monitor) coords.
    let cursor = window.cursor_position().ok();

    let active = cursor.and_then(|c| {
        monitors.iter().find(|m| {
            let pos = m.position();
            let size = m.size();
            let x0 = pos.x as f64;
            let y0 = pos.y as f64;
            let x1 = x0 + size.width as f64;
            let y1 = y0 + size.height as f64;
            c.x >= x0 && c.x < x1 && c.y >= y0 && c.y < y1
        })
    });

    let monitor = match active {
        Some(m) => m,
        None => match monitors.first() {
            Some(m) => m,
            None => return,
        },
    };

    let scale = monitor.scale_factor();
    let m_pos = monitor.position();
    let m_size = monitor.size();
    let hud_w = 340.0_f64;
    // Convert physical monitor origin + size into logical coords so
    // set_position(LogicalPosition) lands consistently across HiDPI displays.
    let mx = m_pos.x as f64 / scale;
    let my = m_pos.y as f64 / scale;
    let mw = m_size.width as f64 / scale;
    let mh = m_size.height as f64 / scale;
    let x = mx + (mw - hud_w) / 2.0;
    let y = my + mh * 0.12;
    let _ = window.set_position(tauri::LogicalPosition::new(x, y));
    // First call after app launch — the window was built off-screen + hidden
    // to dodge the transparent-WebView-paints-black startup flash. Reveal it
    // now that we have a real on-screen target. Subsequent calls hit show()
    // again as a no-op.
    let _ = window.show();
}

fn build_dictation_hud(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    if app.get_webview_window("dictation-hud").is_some() {
        log::info!("dictation-hud: window already exists, skip");
        return Ok(());
    }
    // HUD lifecycle: built hidden (visible=false) so its transparent WebView
    // doesn't paint a black rectangle over whatever's on screen during the
    // initial paint pass. `reposition_dictation_hud` is what positions and
    // shows it on first dictation start; in-session show/hide is handled
    // by the CSS `body.hidden` class toggle from JS.
    let mut builder = WebviewWindowBuilder::new(
        app,
        "dictation-hud",
        WebviewUrl::App("dictation-hud.html".into()),
    )
    .title("")
    .inner_size(340.0, 96.0)
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

    match builder.build()
    {
        Ok(_) => log::info!("dictation-hud: window created"),
        Err(e) => {
            log::error!("dictation-hud: window build failed: {}", e);
            return Err(Box::new(e));
        }
    }

    if let Some(window) = app.get_webview_window("dictation-hud") {
        // Float across all macOS Spaces so the HUD stays visible when the
        // user switches desktops / Mission Control during a dictation session.
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
        // (recording / error with an action button). This is the OS-level
        // belt that backstops the CSS body.hidden / window.hide() path —
        // if either layer races, the NSWindow itself still refuses clicks.
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
        reposition_dictation_hud(app);
    }
    Ok(())
}

/// Build the call-detector debug popup window. Stays hidden until the
/// detector emits a `call-event`; JS in `call-popup.js` flips it visible for
/// ~3s and then hides it again. Will be replaced by a real popup later.
fn build_call_popup(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    if app.get_webview_window("call-popup").is_some() {
        return Ok(());
    }

    let mut builder = WebviewWindowBuilder::new(
        app,
        "call-popup",
        WebviewUrl::App("call-popup.html".into()),
    )
    .title("")
    // 120px tall to fit info row + [Record this meeting] [Ignore] buttons
    // on prompt; the "saved" mode shows only the info row and pads vertically.
    // accept_first_mouse(true) so the first click after the popup appears
    // (when it's not the focused window) still hits Record/Ignore — without
    // it, that first click only activates the window and is consumed.
    .inner_size(340.0, 120.0)
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

    builder.build()?;

    if let Some(window) = app.get_webview_window("call-popup") {
        // Float across all macOS Spaces. tao's set_visible_on_all_workspaces
        // only flips `NSWindowCollectionBehaviorCanJoinAllSpaces` — which
        // doesn't cover *fullscreen* Spaces (Zoom in screen-share/full video
        // sits in its own fullscreen Space). We OR in FullScreenAuxiliary
        // below via NSWindow.collectionBehavior so the popup follows there
        // too.
        let _ = window.set_visible_on_all_workspaces(true);

        // Configure NSWindow directly:
        //   1. setIgnoresMouseEvents:true — initial state is click-through
        //      (popup is invisible). JS flips this off on `call-event` so the
        //      user can drag the visible card, then back on after hide.
        //   2. setLevel:1001 — above kCGScreenSaverWindowLevel (1000) so we
        //      float over Zoom/Teams call panels. Tauri's always_on_top(true)
        //      only sets NSFloatingWindowLevel (3), which call apps cheerfully
        //      sit on top of. Set BEFORE show() so the level is in effect on
        //      first appearance. CAVEAT: calling tauri set_always_on_top(...)
        //      later would reset to floating — don't do that on this window.
        //      Does NOT cover Zoom fullscreen screen-share (CGShieldingWindowLevel
        //      ≈ INT_MAX); going there is user-hostile and not attempted.
        //   3. collectionBehavior |= FullScreenAuxiliary (1<<8 = 256) — needed
        //      so the popup appears even when the foreground app is in a
        //      fullscreen Space (Zoom call in fullscreen, FaceTime fullscreen,
        //      etc). Without this the popup is trapped on Desktop 1.
        //   4. setMovable + setMovableByWindowBackground — borderless windows
        //      default to non-movable; JS startDragging() needs both to be on.
        #[cfg(target_os = "macos")]
        unsafe {
            if let Ok(ns_window_ptr) = window.ns_window() {
                let ns_window: *mut objc2::runtime::AnyObject =
                    ns_window_ptr as *mut objc2::runtime::AnyObject;
                if !ns_window.is_null() {
                    let _: () = objc2::msg_send![ns_window, setIgnoresMouseEvents: true];
                    let _: () = objc2::msg_send![ns_window, setLevel: 1001_i64];
                    let _: () = objc2::msg_send![ns_window, setMovable: true];
                    let _: () = objc2::msg_send![ns_window, setMovableByWindowBackground: true];

                    let current: u64 = objc2::msg_send![ns_window, collectionBehavior];
                    // NSWindowCollectionBehaviorFullScreenAuxiliary = 1 << 8
                    let updated = current | (1u64 << 8);
                    let _: () = objc2::msg_send![ns_window, setCollectionBehavior: updated];
                }
            }
        }

        // Position once at build; call_session re-runs `reposition_call_popup`
        // on each call-event so the popup follows the cursor monitor.
        reposition_call_popup(app);

        // Reveal the window once content is positioned and NSWindow attrs
        // are in place. From here on the NSWindow stays shown — visibility
        // is driven purely by the CSS opacity transition in call-popup.html.
        // With transparent:true + ignoresMouseEvents:true + body opacity 0
        // by default, the shown window is functionally invisible and
        // click-through until JS toggles the `visible` class on body.
        let _ = window.show();
    }
    Ok(())
}

/// Position the call-popup at top-right of the monitor that currently hosts
/// the mouse cursor (falls back to the primary monitor). Called once at
/// build and again from `call_session::handle_started` per call event so a
/// multi-monitor user gets the popup on the screen they're actually looking
/// at. No persistence — drag survives the session but a fresh call event
/// snaps back to this default.
pub fn reposition_call_popup(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("call-popup") else {
        return;
    };
    let Ok(monitors) = window.available_monitors() else {
        return;
    };
    if monitors.is_empty() {
        return;
    }
    let cursor = window.cursor_position().ok();
    let active = cursor.and_then(|c| {
        monitors.iter().find(|m| {
            let pos = m.position();
            let size = m.size();
            let x0 = pos.x as f64;
            let y0 = pos.y as f64;
            let x1 = x0 + size.width as f64;
            let y1 = y0 + size.height as f64;
            c.x >= x0 && c.x < x1 && c.y >= y0 && c.y < y1
        })
    });
    let monitor = match active.or_else(|| monitors.first()) {
        Some(m) => m,
        None => return,
    };
    let scale = monitor.scale_factor();
    let m_pos = monitor.position();
    let m_size = monitor.size();
    let popup_w = 340.0_f64;
    let right_inset = 16.0_f64;          // matches macOS Notification Center
    let top_inset = 40.0_f64;            // menubar (~24) + cushion (16)
    let mx = m_pos.x as f64 / scale;
    let my = m_pos.y as f64 / scale;
    let mw = m_size.width as f64 / scale;
    let x = mx + mw - popup_w - right_inset;
    let y = my + top_inset;
    let _ = window.set_position(tauri::LogicalPosition::new(x, y));
}

/// Show the main window and restore dock presence
fn show_main_window(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Build the system tray icon with menu
/// Build the tray menu from current pipelines + settings. Extracted so it
/// can be re-invoked when pipelines change (see `refresh_tray_menu`).
fn build_tray_menu(
    app: &tauri::AppHandle,
) -> Result<tauri::menu::Menu<tauri::Wry>, tauri::Error> {
    use tauri::menu::{Menu, PredefinedMenuItem};

    // Record submenu — "New Record" first (no pipeline), then a separator,
    // then a flat list of available pipelines (last-used + default surfaced
    // first, capped at 5 to keep the menu readable).
    let new_record_item =
        MenuItemBuilder::with_id("tray-record-new", "New Record").build(app)?;
    let mut record_submenu = SubmenuBuilder::new(app, "Record")
        .item(&new_record_item)
        .separator();

    let pipeline_items: Vec<tauri::menu::MenuItem<tauri::Wry>> = match pipelines::load_pipelines() {
        Ok(pipeline_list) => {
            let settings = config::load_settings();
            let mut names: Vec<String> = Vec::new();
            if let Some(ref last) = settings.last_used_pipeline {
                if pipeline_list.contains_key(last) {
                    names.push(last.clone());
                }
            }
            if let Some(ref default) = settings.default_pipeline {
                if pipeline_list.contains_key(default) && !names.contains(default) {
                    names.push(default.clone());
                }
            }
            for name in pipeline_list.keys() {
                if !names.contains(name) {
                    names.push(name.clone());
                }
            }
            names.truncate(5);
            names
                .iter()
                .filter_map(|name| {
                    MenuItemBuilder::with_id(
                        format!("pipeline:{}", name),
                        format!("▶ {}", name),
                    )
                    .build(app)
                    .ok()
                })
                .collect()
        }
        Err(_) => Vec::new(),
    };
    for item in &pipeline_items {
        record_submenu = record_submenu.item(item);
    }
    let record_submenu = record_submenu.build()?;

    // Top-level items: Record submenu → Home → separator → Quit.
    let home_item = MenuItemBuilder::with_id("show", "Home").build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    Menu::with_items(
        app,
        &[
            &record_submenu,
            &home_item,
            &separator,
            &quit_item,
        ],
    )
}

/// Re-fetch pipelines and swap the tray menu live. Called by
/// `pipelines::save_pipeline` and `pipelines::delete_pipeline` so the
/// Record submenu reflects edits without restarting the app.
pub(crate) fn refresh_tray_menu(app: &tauri::AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        log::warn!("refresh_tray_menu: tray '{}' not found", TRAY_ID);
        return;
    };
    match build_tray_menu(app) {
        Ok(menu) => {
            if let Err(e) = tray.set_menu(Some(menu)) {
                log::warn!("refresh_tray_menu: set_menu failed: {}", e);
            }
        }
        Err(e) => {
            log::warn!("refresh_tray_menu: build_tray_menu failed: {}", e);
        }
    }
}

/// Stable id we use to look the tray up at runtime for menu refresh.
const TRAY_ID: &str = "nbp-main";

fn build_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::Manager;
    let app_handle = app.app_handle().clone();
    let menu = build_tray_menu(&app_handle)?;

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("NBP")
        .on_menu_event(|app: &tauri::AppHandle, event| {
            let id = event.id.as_ref();
            if id.starts_with("pipeline:") {
                let pipeline_name = id.strip_prefix("pipeline:").unwrap().to_string();
                show_main_window(app);
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("tray-start-pipeline", pipeline_name);
                }
            } else {
                match id {
                    "tray-record-new" => {
                        // No pipeline — start a clean recording. Frontend
                        // listener calls startRecording() (vs.
                        // startRecordingWithPipeline) so the chip bar stays
                        // untouched and pendingAutoExec stays empty.
                        show_main_window(app);
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("tray-record-new", ());
                        }
                    }
                    "show" => {
                        show_main_window(app);
                    }
                    "quit" => {
                        // Graceful shutdown
                        if let Some(window) = app.get_webview_window("main") {
                            let state: tauri::State<AudioState> = window.state();
                            {
                                let is_recording = state.is_recording.lock().unwrap_or_else(|e| e.into_inner());
                                if *is_recording {
                                    drop(is_recording);
                                    let mut mic = state.mic_recorder.lock().unwrap_or_else(|e| e.into_inner());
                                    if let Some(mut r) = mic.take() { r.stop(); }
                                    let mut sys = state.system_recorder.lock().unwrap_or_else(|e| e.into_inner());
                                    if let Some(mut r) = sys.take() { r.stop(); }
                                    let mut rt = state.realtime_transcriber.lock().unwrap_or_else(|e| e.into_inner());
                                    if let Some(mut h) = rt.take() { h.stop(); }
                                    let mut mix = state.realtime_mixer.lock().unwrap_or_else(|e| e.into_inner());
                                    if let Some(mut m) = mix.take() { m.stop(); }
                                }
                            }
                            state.wait_for_finalization();
                        }
                        app.exit(0);
                    }
                    _ => {}
                }
            }
        })
        .build(app)?;

    Ok(())
}
