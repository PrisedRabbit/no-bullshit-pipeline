//! Speaker diarization as a SEPARATE pass over a recording (not part of the
//! transcribe path). Calls the native `--diarize-v2` sidecar (senko port:
//! pyannote VAD + Kaldi fbank + CAM++ + spectral), stores `diarization.json`
//! alongside the raw `transcript.json`.
//!
//! Speakers carry a stable `uid` + 192-d CAM++ centroid (voiceprint). On
//! re-diarize, old named speakers are matched to new clusters by centroid
//! cosine, so a rename survives re-runs instead of sticking to an unstable
//! cluster index. A persistent `diarization_status.json` + the
//! `diarization_progress` event keep the UI honest across view switches.

use crate::storage::get_data_dir;
use crate::transcription::convert_ogg_to_wav;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

/// Cross-recording same-speaker acceptance for centroid matching (cosine).
const MATCH_ACCEPT: f32 = 0.80;
/// Best match must beat the runner-up by this margin to be trusted.
const MATCH_MARGIN: f32 = 0.05;

// ---------------------------------------------------------------------------
// Concurrency guard
// ---------------------------------------------------------------------------

/// Recordings with an in-flight diarization — prevents a second pass (and the
/// temp-wav race) when the button is clicked again before the first finishes.
static ACTIVE_DIAR: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
fn active_set() -> &'static Mutex<HashSet<String>> {
    ACTIVE_DIAR.get_or_init(|| Mutex::new(HashSet::new()))
}
/// RAII: drops the recording id from the active set on any exit path.
struct DiarGuard(String);
impl Drop for DiarGuard {
    fn drop(&mut self) {
        if let Ok(mut s) = active_set().lock() {
            s.remove(&self.0);
        }
    }
}

/// Kills the sidecar if the command future is dropped (abort, error path).
struct SidecarGuard(Option<tauri_plugin_shell::process::CommandChild>);
impl Drop for SidecarGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.take() {
            let _ = child.kill();
        }
    }
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DiarSegment {
    pub start: f64,
    pub end: f64,
    pub speaker: i64,
    #[serde(default)]
    pub text: String,
}

/// A detected speaker: stable uid + voiceprint centroid + optional user name.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpeakerEntry {
    pub uid: String,
    /// Cluster index used by `DiarSegment.speaker` within THIS recording.
    pub local_id: i64,
    #[serde(default)]
    pub name: String,
    /// 192-d CAM++ centroid (L2-normalized by the sidecar).
    pub centroid: Vec<f32>,
    /// Split mode: this speaker is the local mic channel ("You").
    #[serde(default)]
    pub is_me: bool,
}

/// Stored `diarization.json`.
#[derive(Serialize, Deserialize)]
pub struct DiarizationJson {
    pub speaker_count: usize,
    pub segments: Vec<DiarSegment>,
    #[serde(default)]
    pub speakers: Vec<SpeakerEntry>,
    pub created_at: String,
}

/// Sidecar `--diarize-v2` / `--diarize-v2-split` stdout shape.
#[derive(Deserialize)]
struct SidecarDiarOut {
    #[serde(rename = "speakerCount")]
    speaker_count: usize,
    segments: Vec<DiarSegment>,
    centroids: Vec<Vec<f32>>,
    #[serde(rename = "meSpeaker", default)]
    me_speaker: Option<i64>,
}

/// Persistent job status (`diarization_status.json`) so the UI can rehydrate
/// after navigating away, and stale `running` is repaired on startup.
#[derive(Serialize, Deserialize, Clone)]
pub struct DiarizationStatus {
    pub status: String, // running | done | failed
    #[serde(default)]
    pub stage: String,
    #[serde(default)]
    pub percent: u32,
    #[serde(default)]
    pub error: String,
    pub updated_at: String,
}

#[derive(Clone, Serialize)]
struct DiarProgressEvent {
    recording_id: String,
    status: String,
    stage: String,
    percent: u32,
}

// ---------------------------------------------------------------------------
// Paths + IO helpers
// ---------------------------------------------------------------------------

fn diar_path(recording_id: &str) -> PathBuf {
    get_data_dir().join(recording_id).join("diarization.json")
}

fn status_path(recording_id: &str) -> PathBuf {
    get_data_dir().join(recording_id).join("diarization_status.json")
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let body = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body).map_err(|e| format!("write {}: {e}", path.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("finalize {}: {e}", path.display()))?;
    Ok(())
}

fn read_diarization(recording_id: &str) -> Option<DiarizationJson> {
    let content = std::fs::read_to_string(diar_path(recording_id)).ok()?;
    serde_json::from_str(&content).ok()
}

/// Write status + notify the UI. Best-effort: status must never fail the job.
fn set_status(app: &tauri::AppHandle, recording_id: &str, status: &str, stage: &str, percent: u32, error: &str) {
    let s = DiarizationStatus {
        status: status.to_string(),
        stage: stage.to_string(),
        percent,
        error: error.to_string(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let _ = write_json_atomic(&status_path(recording_id), &s);
    let _ = app.emit(
        "diarization_progress",
        DiarProgressEvent {
            recording_id: recording_id.to_string(),
            status: status.to_string(),
            stage: stage.to_string(),
            percent,
        },
    );
}

/// Repair statuses left `running` by a crash/quit — call once on startup.
pub fn cleanup_interrupted() {
    let Ok(entries) = std::fs::read_dir(get_data_dir()) else { return };
    for entry in entries.flatten() {
        let sp = entry.path().join("diarization_status.json");
        let Ok(content) = std::fs::read_to_string(&sp) else { continue };
        let Ok(mut st) = serde_json::from_str::<DiarizationStatus>(&content) else { continue };
        if st.status == "running" {
            st.status = "failed".to_string();
            st.error = "Interrupted (app restarted)".to_string();
            st.updated_at = chrono::Utc::now().to_rfc3339();
            let _ = write_json_atomic(&sp, &st);
        }
    }
}

// ---------------------------------------------------------------------------
// Model resolution (the app owns it — panel P0: clean install must work)
// ---------------------------------------------------------------------------

/// Locate a bundled diarization model: packaged resources first, then the dev
/// tree (compile-time manifest path — correct on the machine that built it).
fn find_model(app: &tauri::AppHandle, name: &str) -> Option<PathBuf> {
    if let Ok(res) = app.path().resource_dir() {
        let p = res.join("Models").join(name);
        if p.exists() {
            return Some(p);
        }
    }
    let dev = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fluidaudio-sidecar/Models")
        .join(name);
    if dev.exists() {
        return Some(dev);
    }
    None
}

// ---------------------------------------------------------------------------
// Speaker identity: carry uids/names across re-diarize via centroid cosine
// ---------------------------------------------------------------------------

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (na * nb + 1e-9)
}

/// Build speaker entries for fresh centroids, inheriting uid+name from the
/// previous run when a centroid clearly matches (accept ≥0.80, margin ≥0.05).
/// `me_local` marks the mic-channel speaker in split mode.
fn build_speakers(
    centroids: &[Vec<f32>],
    previous: &[SpeakerEntry],
    me_local: Option<i64>,
) -> Vec<SpeakerEntry> {
    let mut used_prev: HashSet<usize> = HashSet::new();
    centroids
        .iter()
        .enumerate()
        .map(|(local, c)| {
            let is_me = me_local == Some(local as i64);
            let mut best: Option<(usize, f32)> = None;
            let mut second = -1.0f32;
            for (pi, prev) in previous.iter().enumerate() {
                if used_prev.contains(&pi) || prev.centroid.is_empty() {
                    continue;
                }
                let s = cosine(c, &prev.centroid);
                match best {
                    Some((_, bs)) if s > bs => {
                        second = bs;
                        best = Some((pi, s));
                    }
                    Some((_, bs)) => second = second.max(s).min(bs),
                    None => best = Some((pi, s)),
                }
            }
            if let Some((pi, s)) = best {
                if s >= MATCH_ACCEPT && s - second.max(0.0) >= MATCH_MARGIN {
                    used_prev.insert(pi);
                    return SpeakerEntry {
                        uid: previous[pi].uid.clone(),
                        local_id: local as i64,
                        name: previous[pi].name.clone(),
                        centroid: c.clone(),
                        is_me,
                    };
                }
            }
            SpeakerEntry {
                uid: uuid::Uuid::new_v4().to_string(),
                local_id: local as i64,
                name: String::new(),
                centroid: c.clone(),
                is_me,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Run diarization on a recording's audio (separate pass). Stores
/// diarization.json + keeps diarization_status.json / events updated.
#[tauri::command]
pub async fn diarize_recording(
    app_handle: tauri::AppHandle,
    recording_id: String,
) -> Result<usize, String> {
    // Refuse a concurrent diarize on the same recording (dedup + no temp race).
    {
        let mut s = active_set().lock().map_err(|_| "diarization lock poisoned")?;
        if s.contains(&recording_id) {
            return Err("Diarization already running for this recording".to_string());
        }
        s.insert(recording_id.clone());
    }
    let _guard = DiarGuard(recording_id.clone());

    let result = diarize_inner(&app_handle, &recording_id).await;
    match &result {
        Ok(_) => set_status(&app_handle, &recording_id, "done", "Complete", 100, ""),
        Err(e) => set_status(&app_handle, &recording_id, "failed", "", 0, e),
    }
    result
}

async fn diarize_inner(app_handle: &tauri::AppHandle, recording_id: &str) -> Result<usize, String> {
    let dir = get_data_dir().join(recording_id);

    // Prefer source-split: clean stems beat the echo/overlap-contaminated mix
    // (remote voices from raw_system, "me" from raw_mic by channel identity).
    let sys_ogg = dir.join("raw_system.ogg");
    let mic_ogg = dir.join("raw_mic.ogg");
    let split = sys_ogg.exists() && mic_ogg.exists();

    let mut audio = dir.join("audio_mix.ogg");
    if !audio.exists() {
        audio = dir.join("raw_mic.ogg");
    }
    if !split && !audio.exists() {
        return Err("Audio file not found".to_string());
    }

    set_status(app_handle, recording_id, "running", "Converting audio", 0, "");

    // Per-job temp wavs — no collision even if guards are ever bypassed.
    struct TempWav(PathBuf);
    impl Drop for TempWav {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let job = uuid::Uuid::new_v4();

    let mut cmd = app_handle
        .shell()
        .sidecar("fluidaudio-sidecar")
        .map_err(|e| format!("Failed to create sidecar command: {e}"))?;
    let mut _tmps: Vec<TempWav> = Vec::new();
    if split {
        let sys_wav = dir.join(format!("temp_diar_sys_{job}.wav"));
        let mic_wav = dir.join(format!("temp_diar_mic_{job}.wav"));
        convert_ogg_to_wav(&sys_ogg, &sys_wav)?;
        convert_ogg_to_wav(&mic_ogg, &mic_wav)?;
        cmd = cmd
            .arg("--diarize-v2-split")
            .arg(sys_wav.to_str().ok_or("Invalid WAV path")?)
            .arg(mic_wav.to_str().ok_or("Invalid WAV path")?);
        _tmps.push(TempWav(sys_wav));
        _tmps.push(TempWav(mic_wav));
    } else {
        let wav = dir.join(format!("temp_diar_{job}.wav"));
        convert_ogg_to_wav(&audio, &wav)?;
        cmd = cmd.arg("--diarize-v2").arg(wav.to_str().ok_or("Invalid WAV path")?);
        _tmps.push(TempWav(wav));
    }
    if let Some(p) = find_model(app_handle, "camplusplus_batch16.mlmodelc") {
        cmd = cmd.env("NBP_CAMPP", p.to_string_lossy().to_string());
    }
    if let Some(p) = find_model(app_handle, "pyannote_segmentation.mlmodelc") {
        cmd = cmd.env("NBP_PYANNOTE", p.to_string_lossy().to_string());
    }

    let (mut rx, child) = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn diarization sidecar: {e}"))?;
    let mut sidecar_guard = SidecarGuard(Some(child));

    let mut stdout_buf = Vec::new();
    let mut stderr_buf = String::new();
    let mut exit_code: Option<i32> = None;
    let mut last_pct: u32 = 0;

    let timeout_duration = std::time::Duration::from_secs(600);
    let start = std::time::Instant::now();

    while let Some(event) = rx.recv().await {
        if start.elapsed() > timeout_duration {
            return Err("Diarization timed out after 10 minutes".to_string());
        }
        match event {
            CommandEvent::Stdout(data) => stdout_buf.extend_from_slice(&data),
            CommandEvent::Stderr(data) => {
                let line = String::from_utf8_lossy(&data);
                stderr_buf.push_str(&line);
                for l in line.lines() {
                    if let Some(rest) = l.strip_prefix("PROGRESS:") {
                        let parts: Vec<&str> = rest.splitn(2, ':').collect();
                        if parts.len() == 2
                            && let Ok(pct) = parts[1].parse::<u32>()
                            && pct != last_pct
                        {
                            last_pct = pct;
                            set_status(app_handle, recording_id, "running", parts[0], pct, "");
                        }
                    }
                }
            }
            CommandEvent::Terminated(payload) => {
                exit_code = payload.code;
                break;
            }
            _ => {}
        }
    }
    // Sidecar exited on its own — nothing left to kill.
    sidecar_guard.0.take();

    if exit_code != Some(0) {
        let detail: String = stderr_buf
            .lines()
            .filter(|l| !l.starts_with("PROGRESS:") && !l.starts_with("TIMING:") && !l.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        return Err(format!(
            "Diarization failed: {}",
            if detail.is_empty() { format!("sidecar exited with {exit_code:?}") } else { detail }
        ));
    }

    let stdout = String::from_utf8_lossy(&stdout_buf);
    let json_line = stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .ok_or("No JSON in diarization output")?;
    let out: SidecarDiarOut =
        serde_json::from_str(json_line).map_err(|e| format!("Parse diarization JSON: {e}"))?;

    // Inherit uids/names from the previous run by centroid similarity.
    let previous = read_diarization(recording_id).map(|d| d.speakers).unwrap_or_default();
    let speakers = build_speakers(&out.centroids, &previous, out.me_speaker);

    let diar = DiarizationJson {
        speaker_count: out.speaker_count,
        segments: out.segments,
        speakers,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    write_json_atomic(&diar_path(recording_id), &diar)?;
    Ok(diar.speaker_count)
}

/// Store diarization produced inside the transcribe pass (single ASR run).
/// Same uid/name inheritance as a standalone diarize.
pub(crate) fn store_from_transcribe(
    app: &tauri::AppHandle,
    recording_id: &str,
    speaker_count: usize,
    segments: Vec<DiarSegment>,
    centroids: &[Vec<f32>],
) {
    let previous = read_diarization(recording_id).map(|d| d.speakers).unwrap_or_default();
    let speakers = build_speakers(centroids, &previous, None);
    let diar = DiarizationJson {
        speaker_count,
        segments,
        speakers,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    if let Err(e) = write_json_atomic(&diar_path(recording_id), &diar) {
        eprintln!("diarization: store_from_transcribe failed: {e}");
        return;
    }
    set_status(app, recording_id, "done", "Complete", 100, "");
}

/// Read stored diarization for a recording (None if not diarized yet).
#[tauri::command]
pub async fn get_diarization(recording_id: String) -> Result<Option<DiarizationJson>, String> {
    Ok(read_diarization(&recording_id))
}

/// Read the persistent job status (None = never started).
#[tauri::command]
pub async fn get_diarization_status(
    recording_id: String,
) -> Result<Option<DiarizationStatus>, String> {
    let path = status_path(&recording_id);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(serde_json::from_str(&content).ok())
}

/// Rename a speaker (by local cluster id) — persists on the uid-keyed entry.
#[tauri::command]
pub async fn rename_speaker(
    recording_id: String,
    speaker: i64,
    name: String,
) -> Result<(), String> {
    if active_set().lock().map_err(|_| "lock poisoned")?.contains(&recording_id) {
        return Err("Diarization is running — try again when it finishes".to_string());
    }
    let mut d = read_diarization(&recording_id).ok_or("No diarization for this recording")?;
    if let Some(entry) = d.speakers.iter_mut().find(|s| s.local_id == speaker) {
        entry.name = name.trim().to_string();
    } else {
        return Err(format!("Unknown speaker {speaker}"));
    }
    write_json_atomic(&diar_path(&recording_id), &d)?;
    Ok(())
}
