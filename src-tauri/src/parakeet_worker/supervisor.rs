use super::protocol::{
    ControlResult, NdjsonBuffer, PendingReply, ResidentEvent, WorkerCommand, fail_all,
    handle_control, handle_event, handle_transcribe, reject_transcribe,
};
use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tokio::sync::mpsc;

struct SidecarGuard(Option<CommandChild>);

impl Drop for SidecarGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.take() {
            let _ = child.kill();
        }
    }
}

pub(super) async fn supervise(
    app: AppHandle,
    mut commands: mpsc::UnboundedReceiver<WorkerCommand>,
) {
    loop {
        let spawned = app
            .shell()
            .sidecar("fluidaudio-sidecar")
            .and_then(|command| command.args(["--resident-parakeet"]).spawn());
        let (mut events, child) = match spawned {
            Ok(spawned) => spawned,
            Err(error) => {
                log::warn!("resident Parakeet spawn failed: {error}");
                if wait_before_restart(&mut commands).await {
                    return;
                }
                continue;
            }
        };
        let mut child = SidecarGuard(Some(child));
        let mut queued = VecDeque::new();
        let mut buffer = NdjsonBuffer::default();
        let ready = wait_until_ready(&mut events, &mut commands, &mut queued, &mut buffer).await;
        if !ready {
            reject_queued(&mut queued, "resident Parakeet stopped");
        } else {
            let mut pending = HashMap::new();
            let shutdown = serve(
                &mut events,
                &mut commands,
                &mut queued,
                &mut buffer,
                child.0.as_mut().expect("resident child exists"),
                &mut pending,
            )
            .await;
            fail_all(&mut pending, "resident Parakeet stopped");
            if shutdown {
                return;
            }
        }
        drop(child);
        if wait_before_restart(&mut commands).await {
            return;
        }
    }
}

async fn wait_until_ready(
    events: &mut mpsc::Receiver<CommandEvent>,
    commands: &mut mpsc::UnboundedReceiver<WorkerCommand>,
    queued: &mut VecDeque<WorkerCommand>,
    buffer: &mut NdjsonBuffer,
) -> bool {
    loop {
        tokio::select! {
            event = events.recv() => match event {
                Some(CommandEvent::Stdout(chunk)) => for event in buffer.push(&chunk) {
                    match event {
                        Ok(ResidentEvent::Ready) => return true,
                        Ok(ResidentEvent::Fatal { message }) => {
                            log::warn!("resident Parakeet fatal: {message}");
                            return false;
                        }
                        Err(error) => log::warn!("resident Parakeet invalid event: {error}"),
                        _ => {}
                    }
                },
                Some(CommandEvent::Stderr(chunk)) => log_stderr(&chunk),
                Some(CommandEvent::Error(error)) => {
                    log::warn!("resident Parakeet process error: {error}");
                    return false;
                }
                Some(CommandEvent::Terminated(_)) | None => return false,
                _ => {}
            },
            command = commands.recv() => match command {
                Some(command) => match handle_control(command, false) {
                    ControlResult::Handled => {}
                    ControlResult::Shutdown => return false,
                    ControlResult::Forward(command) => queued.push_back(command),
                },
                None => return false,
            }
        }
    }
}

async fn serve(
    events: &mut mpsc::Receiver<CommandEvent>,
    commands: &mut mpsc::UnboundedReceiver<WorkerCommand>,
    queued: &mut VecDeque<WorkerCommand>,
    buffer: &mut NdjsonBuffer,
    child: &mut CommandChild,
    pending: &mut HashMap<String, PendingReply>,
) -> bool {
    loop {
        if let Some(command) = queued.pop_front() {
            if dispatch(command, child, pending) {
                return true;
            }
            continue;
        }
        tokio::select! {
            biased;
            event = events.recv() => match event {
                Some(CommandEvent::Stdout(chunk)) => for event in buffer.push(&chunk) {
                    match event {
                        Ok(event) => if handle_event(event, pending) { return false; },
                        Err(error) => log::warn!("resident Parakeet invalid event: {error}"),
                    }
                },
                Some(CommandEvent::Stderr(chunk)) => log_stderr(&chunk),
                Some(CommandEvent::Error(error)) => {
                    log::warn!("resident Parakeet process error: {error}");
                    return false;
                }
                Some(CommandEvent::Terminated(_)) | None => return false,
                _ => {}
            },
            command = commands.recv() => match command {
                Some(command) => if dispatch(command, child, pending) { return true; },
                None => return true,
            }
        }
    }
}

fn dispatch(
    command: WorkerCommand,
    child: &mut CommandChild,
    pending: &mut HashMap<String, PendingReply>,
) -> bool {
    match handle_control(command, true) {
        ControlResult::Handled => false,
        ControlResult::Shutdown => true,
        ControlResult::Forward(command) => {
            handle_transcribe(command, child, pending);
            false
        }
    }
}

async fn wait_before_restart(commands: &mut mpsc::UnboundedReceiver<WorkerCommand>) -> bool {
    let delay = tokio::time::sleep(Duration::from_secs(30));
    tokio::pin!(delay);
    loop {
        tokio::select! {
            _ = &mut delay => return false,
            command = commands.recv() => match command {
                Some(command) => match handle_control(command, false) {
                    ControlResult::Handled => {}
                    ControlResult::Shutdown => return true,
                    ControlResult::Forward(command) => {
                        reject_transcribe(command, "resident Parakeet is restarting");
                    }
                },
                None => return true,
            }
        }
    }
}

fn reject_queued(queued: &mut VecDeque<WorkerCommand>, message: &str) {
    while let Some(command) = queued.pop_front() {
        reject_transcribe(command, message);
    }
}

fn log_stderr(chunk: &[u8]) {
    log::debug!("resident Parakeet: {}", String::from_utf8_lossy(chunk));
}
