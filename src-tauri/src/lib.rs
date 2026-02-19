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
//! - [`transcription`] - Local Whisper and cloud transcription
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
use audio::AudioState;
use tauri::menu::{AboutMetadataBuilder, MenuBuilder, SubmenuBuilder};
use tauri::Manager;

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_audio_level() -> f32 {
    // Return max of mic and system audio levels
    let mic_level = mic_audio::get_current_audio_level();
    let system_level = system_audio::get_system_audio_level();
    mic_level.max(system_level)
}

pub fn run() {
    // Load settings for managed state
    let settings = std::sync::Arc::new(std::sync::Mutex::new(config::load_settings()));
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())

        .manage(AudioState::new())
        .manage(permissions::PermissionsStateCache(std::sync::Arc::new(std::sync::Mutex::new(
            permissions::PermissionsState::default()
        ))))
        .manage(settings)
        .setup(|app| {
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

            let window_submenu = SubmenuBuilder::new(app, "Window")
                .minimize()
                .close_window()
                .build()?;

            let menu = MenuBuilder::new(app)
                .item(&app_submenu)
                .item(&window_submenu)
                .build()?;

            app.set_menu(menu)?;

            // Run transcript migration on startup
            transcript_migration::run_migration_if_needed();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version,
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
            transcription::get_whisper_models_info,
            transcription::download_whisper_model,
            transcription::delete_whisper_model,
            transcription::transcribe_recording,
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
            // Integrations
            integrations::list_slack_integrations,
            integrations::add_slack_integration,
            integrations::remove_slack_integration,
            integrations::test_slack_integration,
            integrations::list_slack_channels,
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
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Graceful shutdown: stop active recording and wait for finalization
                let state = window.state::<AudioState>();
                {
                    let is_recording = state.is_recording.lock().unwrap_or_else(|e| e.into_inner());
                    if *is_recording {
                        drop(is_recording);
                        // Best-effort stop — we're shutting down
                        let mut mic = state.mic_recorder.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(mut r) = mic.take() { r.stop(); }
                        let mut sys = state.system_recorder.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(mut r) = sys.take() { r.stop(); }
                        let mut mix = state.realtime_mixer.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(mut m) = mix.take() { m.stop(); }
                    }
                }
                state.wait_for_finalization();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
