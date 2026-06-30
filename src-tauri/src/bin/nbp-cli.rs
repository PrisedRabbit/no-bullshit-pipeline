//! Headless lifelog transcriber.
//!
//!   nbp-cli transcribe <file.wav>... [--out <path>] [--pause <sec>] [--sidecar <path>]
//!
//! Drives the FluidAudio sidecar directly — no Tauri runtime, no window. Emits a
//! plain-text transcript with **wall-clock** timecodes derived from the
//! recorder's filename (`YYYYMMDDHHMMSS.WAV`): one timestamp per speech segment,
//! silence dropped, no diarization. Built for the wearable-recorder dump flow:
//! a daemon drops WAVs in a folder, this turns each into a timecoded `.txt` that
//! downstream LLM processing consumes.

use chrono::{Duration, NaiveDateTime};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Pause (seconds) between tokens that starts a new timecoded segment. 3.5s was
/// the sweet spot on real lifelog audio — bigger blocks, minimal punctuation
/// fragments, while genuine multi-second gaps still split (see segment-pause).
const DEFAULT_PAUSE: f64 = 3.5;

/// The single sidecar segment we care about (matches FluidAudioOutputJSON).
#[derive(Deserialize)]
struct Segment {
    #[serde(rename = "startTime")]
    start_time: f64,
    text: String,
}

#[derive(Deserialize)]
struct SidecarOut {
    model: String,
    #[serde(default)]
    segments: Vec<Segment>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args[1] != "transcribe" {
        eprintln!(
            "Usage: nbp-cli transcribe <file.wav>... [--out <path>] [--pause <sec>] [--sidecar <path>]"
        );
        std::process::exit(2);
    }

    let mut wavs: Vec<String> = Vec::new();
    let mut out: Option<String> = None;
    let mut pause = DEFAULT_PAUSE;
    let mut sidecar_override: Option<String> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                if i < args.len() {
                    out = Some(args[i].clone());
                }
            }
            "--pause" => {
                i += 1;
                if i < args.len() {
                    pause = args[i].parse().unwrap_or(DEFAULT_PAUSE);
                }
            }
            "--sidecar" => {
                i += 1;
                if i < args.len() {
                    sidecar_override = Some(args[i].clone());
                }
            }
            other if other.starts_with("--") => {
                eprintln!("Unknown flag: {other}");
                std::process::exit(2);
            }
            other => wavs.push(other.to_string()),
        }
        i += 1;
    }

    if wavs.is_empty() {
        eprintln!("No input WAV given.");
        std::process::exit(2);
    }
    if out.is_some() && wavs.len() > 1 {
        eprintln!("--out takes a single file; for a batch omit it (writes <file>.txt next to each).");
        std::process::exit(2);
    }

    let sidecar = match resolve_sidecar(sidecar_override.as_deref()) {
        Some(p) => p,
        None => {
            eprintln!(
                "Could not locate the fluidaudio-sidecar binary.\n\
                 Pass --sidecar <path> or set NBP_SIDECAR=<path>."
            );
            std::process::exit(1);
        }
    };

    let mut failures = 0;
    for wav in &wavs {
        let out_path = match &out {
            Some(o) => PathBuf::from(o),
            None => PathBuf::from(wav).with_extension("txt"),
        };
        match transcribe_one(&sidecar, wav, pause, &out_path) {
            Ok(n) => eprintln!("\u{2713} {wav} \u{2192} {} ({n} segments)", out_path.display()),
            Err(e) => {
                eprintln!("\u{2717} {wav}: {e}");
                failures += 1;
            }
        }
    }
    if failures > 0 {
        std::process::exit(1);
    }
}

/// Run the sidecar on one file and write the timecoded transcript.
fn transcribe_one(sidecar: &Path, wav: &str, pause: f64, out_path: &Path) -> Result<usize, String> {
    if !Path::new(wav).exists() {
        return Err("file not found".into());
    }
    eprintln!("Transcribing {wav} \u{2026}");

    // stderr inherited so the sidecar's PROGRESS:/TIMING: lines stay visible.
    let child = Command::new(sidecar)
        .arg(wav)
        .arg("--no-diarize")
        .arg("--segment-pause")
        .arg(format!("{pause}"))
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("spawn sidecar: {e}"))?;

    let output = child
        .wait_with_output()
        .map_err(|e| format!("sidecar wait: {e}"))?;
    if !output.status.success() {
        return Err(format!("sidecar exited {}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_line = stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .ok_or("no JSON in sidecar output")?;
    let parsed: SidecarOut =
        serde_json::from_str(json_line).map_err(|e| format!("parse sidecar JSON: {e}"))?;

    let start = parse_recorder_start(wav);
    let body = render(&parsed, start, wav);
    std::fs::write(out_path, &body).map_err(|e| format!("write {}: {e}", out_path.display()))?;

    Ok(parsed.segments.iter().filter(|s| has_text(&s.text)).count())
}

/// Recorder filenames are `YYYYMMDDHHMMSS.WAV` — parse the stem to a start time.
fn parse_recorder_start(wav: &str) -> Option<NaiveDateTime> {
    let stem = Path::new(wav).file_stem()?.to_str()?;
    NaiveDateTime::parse_from_str(stem, "%Y%m%d%H%M%S").ok()
}

/// A segment is worth keeping only if it carries an actual word — pure
/// punctuation / whitespace fragments are dropped (they'd be timecode noise).
fn has_text(t: &str) -> bool {
    t.trim().chars().any(|c| c.is_alphanumeric())
}

/// Build the transcript body: a small header + one `[HH:MM:SS] text` line per
/// kept segment. Timecodes are wall-clock when the filename parsed, else an
/// offset from zero (clearly flagged in the header).
fn render(out: &SidecarOut, start: Option<NaiveDateTime>, wav: &str) -> String {
    let fname = Path::new(wav)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(wav);
    let kept = out.segments.iter().filter(|s| has_text(&s.text)).count();

    let mut s = String::new();
    match start {
        Some(dt) => s.push_str(&format!(
            "# {fname}  {}  (timecodes = wall-clock)\n",
            dt.format("%Y-%m-%d, start %H:%M:%S")
        )),
        None => s.push_str(&format!(
            "# {fname}  (filename not YYYYMMDDHHMMSS \u{2192} timecodes are offset from 0)\n"
        )),
    }
    s.push_str(&format!("# model: {} | segments: {kept}\n\n", out.model));

    for seg in &out.segments {
        if !has_text(&seg.text) {
            continue;
        }
        let stamp = match start {
            Some(dt) => (dt + Duration::milliseconds((seg.start_time * 1000.0) as i64))
                .format("%H:%M:%S")
                .to_string(),
            None => fmt_offset(seg.start_time),
        };
        s.push_str(&format!("[{stamp}] {}\n", seg.text.trim()));
    }
    s
}

fn fmt_offset(t: f64) -> String {
    let t = t as i64;
    format!("{:02}:{:02}:{:02}", t / 3600, (t % 3600) / 60, t % 60)
}

/// Locate the sidecar: explicit override → env → next to our own exe → the dev
/// `binaries/` dir → the Swift `.build/release/` output.
fn resolve_sidecar(override_path: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = override_path {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    if let Ok(p) = std::env::var("NBP_SIDECAR") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }

    let bundled = "fluidaudio-sidecar-aarch64-apple-darwin";
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            cands.push(dir.join(bundled)); // prod: sidecar next to the cli
            // dev: src-tauri/target/{debug,release}/nbp-cli → src-tauri/
            if let Some(src_tauri) = dir.parent().and_then(|p| p.parent()) {
                cands.push(src_tauri.join("binaries").join(bundled));
                if let Some(repo) = src_tauri.parent() {
                    cands.push(
                        repo.join("fluidaudio-sidecar/.build/release/fluidaudio-sidecar"),
                    );
                }
            }
        }
    }
    cands.into_iter().find(|p| p.exists())
}
