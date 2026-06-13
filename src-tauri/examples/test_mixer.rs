use std::path::PathBuf;

fn main() {
    let recording_id = "2c2ab041-17c0-4094-93c0-dc22aec41965";
    let base = PathBuf::from(format!("/Users/skopanev/nbp-data/{}", recording_id));

    let mic_path = base.join("raw_mic.ogg");
    let system_path = base.join("raw_system.ogg");
    let mix_path = base.join("audio_mix_test2.ogg");

    println!("Test Mixer on 10-second recording...\n");

    // Call the mixer
    match nbp_lib::audio_processing::mix_audio_files(&mic_path, &system_path, &mix_path) {
        Ok(_) => println!("\n✅ Mix completed!"),
        Err(e) => println!("\n❌ Mix failed: {}", e),
    }
}
