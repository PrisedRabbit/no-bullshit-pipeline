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

/// Serializes every read-mutate-write over speaker_profiles.json AND the
/// retro-apply fan-out — two concurrent diarizations (or a rename during one)
/// must not lose each other's updates. Held only around fs mutation, never
/// across awaits or sidecar runs.
static DIAR_STATE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
fn state_lock() -> &'static Mutex<()> {
    DIAR_STATE_LOCK.get_or_init(|| Mutex::new(()))
}

/// Reject ids that could escape the data dir (path traversal): recording ids
/// are uuid-like tokens, never path fragments.
fn validate_recording_id(id: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok { Ok(()) } else { Err("Invalid recording id".to_string()) }
}

fn diar_path(recording_id: &str) -> PathBuf {
    get_data_dir().join(recording_id).join("diarization.json")
}

fn status_path(recording_id: &str) -> PathBuf {
    get_data_dir().join(recording_id).join("diarization_status.json")
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let body = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    // Unique temp name: concurrent writers must never rename each other's temp.
    let tmp = path.with_extension(format!("json.tmp.{}", uuid::Uuid::new_v4()));
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

// ---------------------------------------------------------------------------
// Global speaker profiles — cross-recording identity. Renaming a speaker once
// creates/extends a profile (multi-vector voiceprint); every later diarization
// matches its centroids against the profiles and auto-fills the name.
// ---------------------------------------------------------------------------

/// Voiceprints kept per person; more vectors (from different calls/mics) make
/// recognition sturdier.
const PROFILE_MAX_VECTORS: usize = 8;

/// One recording a voice appeared in (drives the People editor: listen/quote).
#[derive(Serialize, Deserialize, Clone)]
pub struct Appearance {
    pub recording_id: String,
    pub local_id: i64,
    pub seconds: f64,
    #[serde(default)]
    pub preview: String,
}

/// A person (named) or a candidate voice (name empty). Candidates accumulate
/// reactively with every diarization — the People editor lists them for
/// naming; assigning a name back-fills every recording they appeared in.
#[derive(Serialize, Deserialize, Clone)]
pub struct SpeakerProfile {
    pub uid: String,
    #[serde(default)]
    pub name: String,
    pub centroids: Vec<Vec<f32>>,
    #[serde(default)]
    pub appearances: Vec<Appearance>,
    pub updated_at: String,
}

fn profiles_path() -> PathBuf {
    get_data_dir().join("speaker_profiles.json")
}

fn load_profiles() -> Vec<SpeakerProfile> {
    std::fs::read_to_string(profiles_path())
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

fn save_profiles(profiles: &[SpeakerProfile]) -> Result<(), String> {
    write_json_atomic(&profiles_path(), &profiles.to_vec())
}

/// Reactive profile growth: after every diarization, every speaker (named OR
/// anonymous) upserts into the global profiles — centroids (cap 8) + an
/// appearance record. Candidates thus accumulate across recordings with no
/// user action; the People editor just reads this file.
fn upsert_profiles_after_diarize(recording_id: &str, d: &DiarizationJson) {
    let _g = state_lock().lock();
    let mut profiles = load_profiles();
    let now = chrono::Utc::now().to_rfc3339();
    for sp in &d.speakers {
        if sp.centroid.is_empty() {
            continue;
        }
        let seconds: f64 = d
            .segments
            .iter()
            .filter(|s| s.speaker == sp.local_id)
            .map(|s| s.end - s.start)
            .sum();
        let preview = d
            .segments
            .iter()
            .filter(|s| s.speaker == sp.local_id && !s.text.trim().is_empty())
            .max_by(|a, b| (a.end - a.start).partial_cmp(&(b.end - b.start)).unwrap_or(std::cmp::Ordering::Equal))
            .map(|s| s.text.trim().chars().take(120).collect::<String>())
            .unwrap_or_default();
        let appearance = Appearance {
            recording_id: recording_id.to_string(),
            local_id: sp.local_id,
            seconds,
            preview,
        };
        match profiles.iter_mut().find(|p| p.uid == sp.uid) {
            Some(p) => {
                if !sp.name.is_empty() {
                    p.name = sp.name.clone();
                }
                p.centroids.push(sp.centroid.clone());
                if p.centroids.len() > PROFILE_MAX_VECTORS {
                    let excess = p.centroids.len() - PROFILE_MAX_VECTORS;
                    p.centroids.drain(0..excess);
                }
                p.appearances.retain(|a| a.recording_id != recording_id);
                p.appearances.push(appearance);
                p.updated_at = now.clone();
            }
            None => profiles.push(SpeakerProfile {
                uid: sp.uid.clone(),
                name: sp.name.clone(),
                centroids: vec![sp.centroid.clone()],
                appearances: vec![appearance],
                updated_at: now.clone(),
            }),
        }
    }
    if let Err(e) = save_profiles(&profiles) {
        eprintln!("diarization: profile upsert failed: {e}");
    }
}

/// Cut a short voice sample per speaker (longest text segment, ≤6s) from the
/// already-converted 16k mono wav(s) — written next to the recording as
/// `voice_<local_id>.wav` for instant playback in the UI / People editor.
fn cut_voice_samples(
    dir: &Path,
    d: &DiarizationJson,
    wav_for_speaker: impl Fn(i64) -> PathBuf,
) {
    use std::collections::HashMap as Map;
    // Pick the longest texted segment per speaker.
    let mut best: Map<i64, &DiarSegment> = Map::new();
    for s in &d.segments {
        if s.text.trim().is_empty() {
            continue;
        }
        let cur = best.get(&s.speaker);
        if cur.map_or(true, |c| (s.end - s.start) > (c.end - c.start)) {
            best.insert(s.speaker, s);
        }
    }
    // Group by source wav so each file is read once.
    let mut by_wav: Map<PathBuf, Vec<(i64, f64, f64)>> = Map::new();
    for (spk, seg) in &best {
        let dur = (seg.end - seg.start).min(6.0);
        if dur < 1.0 {
            continue;
        }
        by_wav
            .entry(wav_for_speaker(*spk))
            .or_default()
            .push((*spk, seg.start, dur));
    }
    for (wav, cuts) in by_wav {
        let Ok(mut reader) = hound::WavReader::open(&wav) else { continue };
        let spec = reader.spec();
        let sr = spec.sample_rate as f64;
        let samples: Vec<i16> = reader.samples::<i16>().filter_map(Result::ok).collect();
        for (spk, start, dur) in cuts {
            let a = ((start * sr) as usize).min(samples.len());
            let b = (((start + dur) * sr) as usize).min(samples.len());
            if b <= a {
                continue;
            }
            let out = dir.join(format!("voice_{spk}.wav"));
            if let Ok(mut w) = hound::WavWriter::create(&out, spec) {
                for s in &samples[a..b] {
                    let _ = w.write_sample(*s);
                }
                let _ = w.finalize();
            }
        }
    }
}

/// Best profile for a centroid: max cosine over each profile's vectors, with
/// the same accept/margin bar as within-recording matching.
fn match_profile<'a>(centroid: &[f32], profiles: &'a [SpeakerProfile]) -> Option<&'a SpeakerProfile> {
    let mut best: Option<(&SpeakerProfile, f32)> = None;
    let mut second = -1.0f32;
    for p in profiles {
        let score = p
            .centroids
            .iter()
            .map(|c| cosine(centroid, c))
            .fold(-1.0f32, f32::max);
        match best {
            Some((_, bs)) if score > bs => {
                second = bs;
                best = Some((p, score));
            }
            Some((_, bs)) => second = second.max(score).min(bs),
            None => best = Some((p, score)),
        }
    }
    match best {
        Some((p, s)) if s >= MATCH_ACCEPT && s - second.max(0.0) >= MATCH_MARGIN => Some(p),
        _ => None,
    }
}

/// Build speaker entries for fresh centroids, inheriting uid+name from the
/// previous run when a centroid clearly matches (accept ≥0.80, margin ≥0.05).
/// `me_local` marks the mic-channel speaker in split mode.
fn build_speakers(
    centroids: &[Vec<f32>],
    previous: &[SpeakerEntry],
    me_local: Option<i64>,
) -> Vec<SpeakerEntry> {
    let profiles = load_profiles();
    let mut used_prev: HashSet<usize> = HashSet::new();
    let mut used_profiles: HashSet<String> = HashSet::new();
    centroids
        .iter()
        .enumerate()
        .map(|(local, c)| {
            let is_me = me_local == Some(local as i64);
            // 1) Same-recording previous run — the surest match (same audio).
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
            // 2) Global speaker profiles — cross-recording auto-naming.
            if let Some(p) = match_profile(c, &profiles) {
                if !used_profiles.contains(&p.uid) {
                    used_profiles.insert(p.uid.clone());
                    return SpeakerEntry {
                        uid: p.uid.clone(),
                        local_id: local as i64,
                        name: p.name.clone(),
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
    validate_recording_id(&recording_id)?;
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
    // Kept for post-run voice-sample cutting (files are removed on drop).
    let mut sys_wav_path: Option<PathBuf> = None;
    let mut mic_wav_path: Option<PathBuf> = None;
    let mut mix_wav_path: Option<PathBuf> = None;
    if split {
        let sys_wav = dir.join(format!("temp_diar_sys_{job}.wav"));
        let mic_wav = dir.join(format!("temp_diar_mic_{job}.wav"));
        convert_ogg_to_wav(&sys_ogg, &sys_wav)?;
        convert_ogg_to_wav(&mic_ogg, &mic_wav)?;
        cmd = cmd
            .arg("--diarize-v2-split")
            .arg(sys_wav.to_str().ok_or("Invalid WAV path")?)
            .arg(mic_wav.to_str().ok_or("Invalid WAV path")?);
        sys_wav_path = Some(sys_wav.clone());
        mic_wav_path = Some(mic_wav.clone());
        _tmps.push(TempWav(sys_wav));
        _tmps.push(TempWav(mic_wav));
    } else {
        let wav = dir.join(format!("temp_diar_{job}.wav"));
        convert_ogg_to_wav(&audio, &wav)?;
        cmd = cmd.arg("--diarize-v2").arg(wav.to_str().ok_or("Invalid WAV path")?);
        mix_wav_path = Some(wav.clone());
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

    // Real wall-clock timeout: a silently hung sidecar produces NO events, so
    // the deadline must wrap `recv()` itself — a post-event elapsed check would
    // never fire. Split mode runs 2 diar pipelines + 2 ASR passes → more room.
    let timeout_duration = std::time::Duration::from_secs(if split { 1200 } else { 600 });
    let start = std::time::Instant::now();

    loop {
        let remaining = timeout_duration.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            // SidecarGuard kills the child on drop.
            return Err("Diarization timed out".to_string());
        }
        let event = match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(ev)) => ev,
            Ok(None) => break, // channel closed without Terminated — guard cleans up
            Err(_) => return Err("Diarization timed out".to_string()),
        };
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
                // Clean exit — nothing left to kill. (On any other path the
                // guard's kill-on-drop stays armed.)
                sidecar_guard.0.take();
                break;
            }
            _ => {}
        }
    }

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

    // Reactive layers: voice samples (while the temp wavs still exist) and the
    // global candidate profiles for the People editor.
    let me_local = out.me_speaker;
    cut_voice_samples(&dir, &diar, |spk| {
        if split {
            if Some(spk) == me_local {
                mic_wav_path.clone().unwrap_or_default()
            } else {
                sys_wav_path.clone().unwrap_or_default()
            }
        } else {
            mix_wav_path.clone().unwrap_or_default()
        }
    });
    upsert_profiles_after_diarize(recording_id, &diar);

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
    source_wav: Option<&Path>,
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
    // Cut voice samples while the transcribe temp wav still exists — otherwise
    // People-editor candidates from this path would have nothing to play.
    if let Some(wav) = source_wav {
        let dir = get_data_dir().join(recording_id);
        cut_voice_samples(&dir, &diar, |_| wav.to_path_buf());
    }
    upsert_profiles_after_diarize(recording_id, &diar);
    set_status(app, recording_id, "done", "Complete", 100, "");
}

/// Read stored diarization for a recording (None if not diarized yet).
#[tauri::command]
pub async fn get_diarization(recording_id: String) -> Result<Option<DiarizationJson>, String> {
    validate_recording_id(&recording_id)?;
    Ok(read_diarization(&recording_id))
}

/// Read the persistent job status (None = never started).
#[tauri::command]
pub async fn get_diarization_status(
    recording_id: String,
) -> Result<Option<DiarizationStatus>, String> {
    validate_recording_id(&recording_id)?;
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
    validate_recording_id(&recording_id)?;
    if active_set().lock().map_err(|_| "lock poisoned")?.contains(&recording_id) {
        return Err("Diarization is running — try again when it finishes".to_string());
    }
    let mut d = read_diarization(&recording_id).ok_or("No diarization for this recording")?;
    let trimmed = name.trim().to_string();
    let entry = d
        .speakers
        .iter_mut()
        .find(|s| s.local_id == speaker)
        .ok_or_else(|| format!("Unknown speaker {speaker}"))?;
    entry.name = trimmed.clone();
    let uid = entry.uid.clone();
    let centroid = entry.centroid.clone();

    // The name is a LABEL on this voice's uid — never a join key. Two people
    // may share a display name; merging identities is an explicit future
    // operation, not a side effect of typing the same string.
    let _g = state_lock().lock();
    if !trimmed.is_empty() && !centroid.is_empty() {
        let mut profiles = load_profiles();
        let now = chrono::Utc::now().to_rfc3339();
        match profiles.iter_mut().find(|p| p.uid == uid) {
            Some(p) => {
                p.name = trimmed.clone();
                p.centroids.push(centroid.clone());
                if p.centroids.len() > PROFILE_MAX_VECTORS {
                    let excess = p.centroids.len() - PROFILE_MAX_VECTORS;
                    p.centroids.drain(0..excess);
                }
                p.updated_at = now;
            }
            None => profiles.push(SpeakerProfile {
                uid: uid.clone(),
                name: trimmed.clone(),
                centroids: vec![centroid.clone()],
                appearances: Vec::new(),
                updated_at: now,
            }),
        }
        save_profiles(&profiles)?;
    }

    write_json_atomic(&diar_path(&recording_id), &d)?;

    // Retro-apply: the same voice (same uid) in OTHER recordings gets the name
    // too — one rename labels the whole history. Recordings mid-diarize are
    // skipped (their fresh write would clobber ours); the next rename or
    // re-diarize reconciles them via the profile.
    if !trimmed.is_empty() {
        let appearances: Vec<Appearance> = load_profiles()
            .into_iter()
            .find(|p| p.uid == uid)
            .map(|p| p.appearances)
            .unwrap_or_default();
        for a in appearances {
            if a.recording_id == recording_id {
                continue;
            }
            let busy = active_set()
                .lock()
                .map(|s| s.contains(&a.recording_id))
                .unwrap_or(true);
            if busy {
                continue;
            }
            if let Some(mut other) = read_diarization(&a.recording_id) {
                let mut changed = false;
                for s in other.speakers.iter_mut() {
                    if s.uid == uid && s.name != trimmed {
                        s.name = trimmed.clone();
                        changed = true;
                    }
                }
                if changed {
                    let _ = write_json_atomic(&diar_path(&a.recording_id), &other);
                }
            }
        }
    }
    Ok(())
}

/// Purge a deleted recording from the identity store: drop its appearances
/// and any anonymous profile left with no appearances (deleting a recording
/// must also delete the biometric traces it produced).
pub fn purge_recording(recording_id: &str) {
    let _g = state_lock().lock();
    let mut profiles = load_profiles();
    let before = profiles.len();
    for p in profiles.iter_mut() {
        p.appearances.retain(|a| a.recording_id != recording_id);
    }
    profiles.retain(|p| !p.name.is_empty() || !p.appearances.is_empty());
    if let Err(e) = save_profiles(&profiles) {
        eprintln!("diarization: purge failed: {e}");
    } else if profiles.len() != before {
        log::info!("diarization: pruned {} orphan voice profile(s)", before - profiles.len());
    }
}

/// Lightweight profile projection for the People editor (no centroid payload).
#[derive(Serialize)]
pub struct ProfileView {
    pub uid: String,
    pub name: String,
    pub total_seconds: f64,
    pub recordings: usize,
    pub preview: String,
    /// Best appearance to play a voice sample from.
    pub sample_recording_id: String,
    pub sample_local_id: i64,
    /// Whether a voice sample file actually exists for that appearance.
    pub has_sample: bool,
}

/// All profiles (named + candidates), best-appearance first. The UI derives
/// the "N voices waiting for names" badge from entries with an empty name.
#[tauri::command]
pub async fn list_speaker_profiles() -> Result<Vec<ProfileView>, String> {
    let mut views: Vec<ProfileView> = load_profiles()
        .into_iter()
        .map(|p| {
            let total: f64 = p.appearances.iter().map(|a| a.seconds).sum();
            let has_file = |a: &&Appearance| {
                get_data_dir()
                    .join(&a.recording_id)
                    .join(format!("voice_{}.wav", a.local_id))
                    .exists()
            };
            let by_secs = |a: &&Appearance, b: &&Appearance| {
                a.seconds.partial_cmp(&b.seconds).unwrap_or(std::cmp::Ordering::Equal)
            };
            // Prefer the longest appearance that actually HAS a sample file;
            // fall back to the longest overall (quote still useful, ▶ hidden).
            let best = p
                .appearances
                .iter()
                .filter(has_file)
                .max_by(by_secs)
                .or_else(|| p.appearances.iter().max_by(by_secs));
            let has_sample = best.map(|a| has_file(&a)).unwrap_or(false);
            ProfileView {
                uid: p.uid,
                name: p.name,
                total_seconds: total,
                recordings: p.appearances.len(),
                preview: best.map(|a| a.preview.clone()).unwrap_or_default(),
                sample_recording_id: best.map(|a| a.recording_id.clone()).unwrap_or_default(),
                sample_local_id: best.map(|a| a.local_id).unwrap_or(0),
                has_sample,
            }
        })
        .collect();
    views.sort_by(|a, b| b.total_seconds.partial_cmp(&a.total_seconds).unwrap_or(std::cmp::Ordering::Equal));
    Ok(views)
}

/// Voice sample (wav) for a speaker in a recording, base64 for an <audio> tag.
#[tauri::command]
pub async fn get_voice_sample(recording_id: String, speaker: i64) -> Result<String, String> {
    validate_recording_id(&recording_id)?;
    if speaker < 0 {
        return Err("Invalid speaker".to_string());
    }
    let path = get_data_dir()
        .join(&recording_id)
        .join(format!("voice_{speaker}.wav"));
    let bytes = std::fs::read(&path).map_err(|_| "No voice sample for this speaker".to_string())?;
    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}
