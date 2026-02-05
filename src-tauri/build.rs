use std::path::Path;
use std::process::Command;

fn main() {
    // Build fluidaudio-sidecar from Swift package
    let sidecar_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fluidaudio-sidecar");
    let target_binary = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("binaries/fluidaudio-sidecar-aarch64-apple-darwin");

    // Rebuild when sidecar source changes
    println!("cargo:rerun-if-changed=../fluidaudio-sidecar/Sources/");
    println!("cargo:rerun-if-changed=../fluidaudio-sidecar/Package.swift");

    let status = Command::new("swift")
        .args(["build", "-c", "release"])
        .current_dir(&sidecar_dir)
        .status()
        .expect("Failed to run swift build — is Swift installed?");

    if !status.success() {
        panic!("swift build failed for fluidaudio-sidecar");
    }

    let built = sidecar_dir.join(".build/release/fluidaudio-sidecar");
    std::fs::copy(&built, &target_binary).unwrap_or_else(|e| {
        panic!(
            "Failed to copy sidecar binary from {} to {}: {}",
            built.display(),
            target_binary.display(),
            e
        )
    });

    tauri_build::build()
}
