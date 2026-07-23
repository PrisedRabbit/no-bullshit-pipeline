// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Headless CLI mode: `nbp transcribe <file>...` drives the FluidAudio sidecar
    // directly and exits, without spinning up the Tauri runtime or a window.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("transcribe") {
        std::process::exit(nbp_lib::transcribe_cli::run(&args));
    }

    nbp_lib::run()
}
