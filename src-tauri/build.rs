use std::path::Path;
use std::process::Command;

fn main() {
    let sidecar_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fluidaudio-sidecar");
    let target_binary = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("binaries/fluidaudio-sidecar-aarch64-apple-darwin");

    println!("cargo:rerun-if-changed=../fluidaudio-sidecar/Sources/");
    println!("cargo:rerun-if-changed=../fluidaudio-sidecar/Package.swift");
    println!("cargo:rerun-if-changed=binaries/fluidaudio-sidecar-aarch64-apple-darwin");

    // Only build if sources exist and binary is missing or REBUILD_SIDECAR is set
    if sidecar_dir.exists() && (!target_binary.exists() || std::env::var("REBUILD_SIDECAR").is_ok())
    {
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
    }

    tauri_build::build()
}
