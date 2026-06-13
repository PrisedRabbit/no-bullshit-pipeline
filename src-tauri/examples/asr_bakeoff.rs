//! ASR bake-off harness for the RU+EN code-switching evaluation.
//!
//! Runs one audio file through every on-device candidate engine and prints the
//! transcripts side by side so you can judge which handles mixed Russian+English
//! best on YOUR voice. Reuses the exact same OGG→WAV conversion the app uses, so
//! the comparison is fair (identical preprocessing).
//!
//! Usage:
//!   cargo run --example asr_bakeoff -- <audio.ogg|audio.wav> [vocab.txt]
//!
//! - `audio` may be an app recording (`~/nbp-data/<uuid>/audio_mix.ogg`) or a WAV.
//! - `vocab.txt` (optional): one term per line — enables the Parakeet+vocab run.
//!   See `scripts/asr-vocab.sample.txt`.
//!
//! Build the sidecar first: `bun run build:sidecar`.
//! Results are also written to `bakeoff-results.md` next to the audio file.

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

#[derive(serde::Deserialize)]
struct SidecarOut {
    text: String,
    model: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --example asr_bakeoff -- <audio.ogg|audio.wav> [vocab.txt]");
        std::process::exit(2);
    }
    let audio = PathBuf::from(&args[1]);
    if !audio.exists() {
        eprintln!("audio not found: {}", audio.display());
        std::process::exit(1);
    }
    let vocab = args.get(2).map(PathBuf::from);
    if let Some(v) = &vocab
        && !v.exists()
    {
        eprintln!("vocab file not found: {}", v.display());
        std::process::exit(1);
    }

    let sidecar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries/fluidaudio-sidecar-aarch64-apple-darwin");
    if !sidecar.exists() {
        eprintln!(
            "sidecar binary not found at {}\nRun `bun run build:sidecar` first.",
            sidecar.display()
        );
        std::process::exit(1);
    }

    // The sidecar reads audio via AVFoundation (WAV / M4A / AIFF / CAF / MP3 all
    // work), so most formats pass straight through. Only OGG needs pre-conversion
    // — AVFoundation can't decode Vorbis — done with the same routine the app uses.
    let is_ogg = audio
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("ogg"))
        .unwrap_or(false);
    let input = if is_ogg {
        let tmp = std::env::temp_dir().join("asr_bakeoff_input.wav");
        eprintln!("Converting {} → WAV …", audio.display());
        nbp_lib::transcription::convert_ogg_to_wav(&audio, &tmp)
            .expect("OGG→WAV conversion failed");
        tmp
    } else {
        audio.clone()
    };
    let wav_str = input.to_string_lossy().to_string();

    // Candidate engines. `--no-diarize` everywhere: the bake-off judges the
    // transcript text, not speaker turns (and Qwen3 has no diarization anyway).
    let mut runs: Vec<(String, Vec<String>)> = vec![(
        "parakeet-v3 (baseline)".to_string(),
        vec![wav_str.clone(), "--no-diarize".to_string()],
    )];
    if let Some(v) = &vocab {
        runs.push((
            "parakeet-v3 + vocab".to_string(),
            vec![
                wav_str.clone(),
                "--no-diarize".to_string(),
                "--vocab".to_string(),
                v.to_string_lossy().to_string(),
            ],
        ));
    }
    runs.push((
        "qwen3-asr (f32)".to_string(),
        vec![
            wav_str.clone(),
            "--engine".to_string(),
            "qwen3".to_string(),
            "--no-diarize".to_string(),
        ],
    ));

    let mut report = String::from("# ASR bake-off\n\n");
    report.push_str(&format!("audio: `{}`\n\n", audio.display()));

    for (label, a) in &runs {
        eprintln!("\n=== {label} === (first run downloads the model)");
        let t = Instant::now();
        let out = Command::new(&sidecar)
            .args(a)
            .output()
            .expect("failed to spawn sidecar");
        let secs = t.elapsed().as_secs_f64();

        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            let tail: Vec<&str> = err.lines().filter(|l| !l.is_empty()).collect();
            let tail = tail
                .iter()
                .rev()
                .take(6)
                .rev()
                .copied()
                .collect::<Vec<_>>()
                .join("\n");
            eprintln!("FAILED ({secs:.1}s):\n{tail}");
            report.push_str(&format!(
                "## {label}\n\n_FAILED_ ({secs:.1}s)\n\n```\n{tail}\n```\n\n"
            ));
            continue;
        }

        let stdout = String::from_utf8_lossy(&out.stdout);
        match serde_json::from_str::<SidecarOut>(stdout.trim()) {
            Ok(o) => {
                println!(
                    "\n----- {label}  [{}]  {secs:.1}s -----\n{}\n",
                    o.model, o.text
                );
                report.push_str(&format!(
                    "## {label}\n\nmodel: `{}` · {secs:.1}s\n\n{}\n\n",
                    o.model, o.text
                ));
            }
            Err(e) => {
                eprintln!("parse error: {e}\nraw: {}", stdout.trim());
                report.push_str(&format!("## {label}\n\n_parse error_: {e}\n\n"));
            }
        }
    }

    let report_path = audio.with_file_name("bakeoff-results.md");
    if let Err(e) = std::fs::write(&report_path, &report) {
        eprintln!("could not write report: {e}");
    } else {
        eprintln!("\nReport written to {}", report_path.display());
    }
}
