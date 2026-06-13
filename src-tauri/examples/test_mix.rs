// Test the audio mixer with mismatched sample rates

use nbp_lib::audio_processing::mix_audio_files;
use std::path::PathBuf;

fn main() {
    let base = PathBuf::from("/Users/skopanev/nbp-data/43a80165-9166-412e-b7c9-5d2f7956623f");
    let mic = base.join("raw_mic.ogg");
    let sys = base.join("raw_system.ogg");
    let output = base.join("audio_mix.ogg");

    println!("Testing mixer with:");
    println!("  Mic: {:?}", mic);
    println!("  System: {:?}", sys);
    println!("  Output: {:?}", output);

    match mix_audio_files(mic, sys, output) {
        Ok(_) => println!("\n✅ Mix successful!"),
        Err(e) => eprintln!("\n❌ Mix failed: {}", e),
    }
}
