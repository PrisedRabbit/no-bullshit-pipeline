mod audio;
mod storage;
mod system_audio;
use audio::AudioState;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AudioState::new())
        .invoke_handler(tauri::generate_handler![
            greet,
            audio::start_recording,
            audio::stop_recording,
            audio::pause_recording,
            audio::resume_recording,
            storage::list_recordings,
            storage::read_metadata,
            storage::update_tags,
            storage::delete_recording,

        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
