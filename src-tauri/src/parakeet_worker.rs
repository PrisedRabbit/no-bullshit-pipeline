mod protocol;
mod supervisor;

use crate::config::{AppSettings, TranscriptionProvider};
use protocol::{OwnedTempPath, ResidentRequest, WorkerCommand};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc, oneshot, watch};
use uuid::Uuid;

pub struct ParakeetWorkerState {
    runtime: Mutex<RuntimeSlot>,
}

enum RuntimeSlot {
    Stopped,
    Running(WorkerRuntime),
    Stopping(StopCompletion),
    ShuttingDown(Option<StopCompletion>),
}

type StopCompletion = watch::Receiver<bool>;

struct WorkerRuntime {
    tx: mpsc::UnboundedSender<WorkerCommand>,
    task: tauri::async_runtime::JoinHandle<()>,
}

impl ParakeetWorkerState {
    pub fn new() -> Self {
        Self {
            runtime: Mutex::new(RuntimeSlot::Stopped),
        }
    }
}

pub fn configured(settings: &AppSettings) -> bool {
    settings.dictation.enabled
        && settings.transcription.provider == TranscriptionProvider::FluidAudio
        && settings.transcription.keep_model_ready
}

pub fn reconcile(app: &AppHandle) {
    let state = app.state::<ParakeetWorkerState>();
    let mut slot = state
        .runtime
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let enabled = configured(&crate::config::load_settings());
    match (&*slot, enabled) {
        (RuntimeSlot::Stopped, true) => {
            *slot = RuntimeSlot::Running(start_runtime(app));
        }
        (RuntimeSlot::Running(_), false) => {
            let runtime = match std::mem::replace(&mut *slot, RuntimeSlot::Stopped) {
                RuntimeSlot::Running(runtime) => runtime,
                _ => unreachable!(),
            };
            *slot = RuntimeSlot::Stopping(spawn_stop(app.clone(), runtime));
        }
        _ => {}
    }
}

pub async fn transcribe(
    app: &AppHandle,
    samples_16k: &[f32],
    settings: &AppSettings,
) -> Result<String, String> {
    let path = std::env::temp_dir().join(format!("nbp-dict-{}.wav", Uuid::new_v4().simple()));
    let temp_path = OwnedTempPath::new(path);
    write_mono_wav(temp_path.path(), samples_16k)?;
    let vocab_path = if settings.transcription.translit_lang != "off" {
        let path = crate::vocab::vocab_path();
        path.exists().then(|| path.to_string_lossy().into_owned())
    } else {
        None
    };
    let request = ResidentRequest::transcribe(
        Uuid::new_v4().to_string(),
        temp_path.path().to_string_lossy().into_owned(),
        vocab_path,
        settings.transcription.translit_threshold,
        settings.transcription.translit_min_len,
    );
    let tx = running_tx(app).ok_or("resident Parakeet is not running")?;
    let (reply, receiver) = oneshot::channel();
    tx.send(WorkerCommand::Transcribe {
        request,
        temp_path,
        reply,
    })
    .map_err(|_| "resident Parakeet stopped".to_string())?;
    receiver
        .await
        .map_err(|_| "resident Parakeet stopped".to_string())?
}

pub async fn verify_after_wake(app: &AppHandle) {
    let Some(tx) = running_tx(app) else {
        reconcile(app);
        return;
    };
    let (reply, receiver) = oneshot::channel();
    let healthy = tx.send(WorkerCommand::Health { reply }).is_ok()
        && matches!(
            tokio::time::timeout(Duration::from_secs(5), receiver).await,
            Ok(Ok(Ok(())))
        );
    if healthy {
        return;
    }
    let state = app.state::<ParakeetWorkerState>();
    let completion = {
        let mut slot = state
            .runtime
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if matches!(&*slot, RuntimeSlot::Running(runtime) if runtime.tx.same_channel(&tx)) {
            match std::mem::replace(&mut *slot, RuntimeSlot::Stopped) {
                RuntimeSlot::Running(runtime) => {
                    let completion = spawn_stop(app.clone(), runtime);
                    *slot = RuntimeSlot::Stopping(completion.clone());
                    Some(completion)
                }
                _ => unreachable!(),
            }
        } else {
            None
        }
    };
    if let Some(completion) = completion {
        wait_for_stop(completion).await;
    }
}

pub async fn shutdown(app: &AppHandle) {
    let state = app.state::<ParakeetWorkerState>();
    let completion = {
        let mut slot = state
            .runtime
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        match std::mem::replace(&mut *slot, RuntimeSlot::ShuttingDown(None)) {
            RuntimeSlot::Running(runtime) => {
                let completion = spawn_stop(app.clone(), runtime);
                *slot = RuntimeSlot::ShuttingDown(Some(completion.clone()));
                Some(completion)
            }
            RuntimeSlot::Stopping(completion) => {
                *slot = RuntimeSlot::ShuttingDown(Some(completion.clone()));
                Some(completion)
            }
            RuntimeSlot::ShuttingDown(completion) => {
                let wait = completion.clone();
                *slot = RuntimeSlot::ShuttingDown(completion);
                wait
            }
            RuntimeSlot::Stopped => None,
        }
    };
    if let Some(completion) = completion {
        wait_for_stop(completion).await;
    }
}

fn running_tx(app: &AppHandle) -> Option<mpsc::UnboundedSender<WorkerCommand>> {
    let state = app.state::<ParakeetWorkerState>();
    let slot = state
        .runtime
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    match &*slot {
        RuntimeSlot::Running(runtime) => Some(runtime.tx.clone()),
        _ => None,
    }
}

fn start_runtime(app: &AppHandle) -> WorkerRuntime {
    let (tx, rx) = mpsc::unbounded_channel();
    let handle = app.clone();
    let task = tauri::async_runtime::spawn(async move { supervisor::supervise(handle, rx).await });
    WorkerRuntime { tx, task }
}

fn spawn_stop(app: AppHandle, runtime: WorkerRuntime) -> StopCompletion {
    let (done, completion) = watch::channel(false);
    tauri::async_runtime::spawn(async move {
        stop_runtime(runtime).await;
        finish_stop(&app);
        let _ = done.send(true);
    });
    completion
}

async fn stop_runtime(runtime: WorkerRuntime) {
    let _ = runtime.tx.send(WorkerCommand::Shutdown);
    runtime.task.abort();
    let _ = runtime.task.await;
}

async fn wait_for_stop(mut completion: StopCompletion) {
    while !*completion.borrow() {
        if completion.changed().await.is_err() {
            break;
        }
    }
}

fn finish_stop(app: &AppHandle) {
    let state = app.state::<ParakeetWorkerState>();
    let mut slot = state
        .runtime
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let enabled = configured(&crate::config::load_settings());
    match &*slot {
        RuntimeSlot::Stopping(_) if enabled => {
            *slot = RuntimeSlot::Running(start_runtime(app));
        }
        RuntimeSlot::Stopping(_) => {
            *slot = RuntimeSlot::Stopped;
        }
        _ => {}
    }
}

fn write_mono_wav(path: &Path, samples: &[f32]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|error| error.to_string())?;
    for &sample in samples {
        writer
            .write_sample((sample * 32768.0).clamp(-32768.0, 32767.0) as i16)
            .map_err(|error| error.to_string())?;
    }
    writer.finalize().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_only_for_enabled_parakeet_dictation() {
        let mut settings = AppSettings::default();
        settings.dictation.enabled = true;
        settings.transcription.provider = TranscriptionProvider::FluidAudio;
        settings.transcription.keep_model_ready = true;
        assert!(configured(&settings));
        settings.dictation.enabled = false;
        assert!(!configured(&settings));
        settings.dictation.enabled = true;
        settings.transcription.provider = TranscriptionProvider::Qwen3;
        assert!(!configured(&settings));
        settings.transcription.provider = TranscriptionProvider::AppleSpeech;
        assert!(!configured(&settings));
        settings.transcription.provider = TranscriptionProvider::FluidAudio;
        settings.transcription.keep_model_ready = false;
        assert!(!configured(&settings));
    }
}
