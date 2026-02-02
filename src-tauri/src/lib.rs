mod audio;
pub mod audio_processing;
mod storage;
mod system_audio;
mod mic_audio;
mod permissions;
mod config;
pub mod transcription;
mod cloud_ai;
mod templates;
mod playback;
mod waveform;
mod devices;
use audio::AudioState;
use tauri::menu::{AboutMetadataBuilder, MenuBuilder, SubmenuBuilder};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AudioState::new())
        .manage(permissions::PermissionsStateCache(std::sync::Arc::new(std::sync::Mutex::new(
            permissions::PermissionsState::default()
        ))))
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
