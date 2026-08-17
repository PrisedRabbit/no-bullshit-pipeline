use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri_plugin_shell::process::CommandChild;
use tokio::sync::oneshot;

pub(super) enum WorkerCommand {
    Transcribe {
        request: ResidentRequest,
        temp_path: OwnedTempPath,
        reply: oneshot::Sender<Result<String, String>>,
    },
    Health {
        reply: oneshot::Sender<Result<(), String>>,
    },
    Shutdown,
}

pub(super) enum ControlResult {
    Handled,
    Shutdown,
    Forward(WorkerCommand),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ResidentRequest {
    id: String,
    operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    wav_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vocab_path: Option<String>,
    translit_threshold: f32,
    translit_min_len: u32,
}

impl ResidentRequest {
    pub(super) fn transcribe(
        id: String,
        wav_path: String,
        vocab_path: Option<String>,
        translit_threshold: f32,
        translit_min_len: u32,
    ) -> Self {
        Self {
            id,
            operation: "transcribe".into(),
            wav_path: Some(wav_path),
            vocab_path,
            translit_threshold,
            translit_min_len,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(super) enum ResidentEvent {
    Ready,
    Pong {
        id: String,
    },
    Result {
        id: String,
        text: String,
        model: String,
    },
    Error {
        id: Option<String>,
        message: String,
    },
    Fatal {
        message: String,
    },
}

pub(super) enum PendingReply {
    Transcribe {
        _temp_path: OwnedTempPath,
        reply: oneshot::Sender<Result<String, String>>,
    },
}

pub(super) struct OwnedTempPath(PathBuf);

impl OwnedTempPath {
    pub(super) fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for OwnedTempPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[derive(Default)]
pub(super) struct NdjsonBuffer(Vec<u8>);

impl NdjsonBuffer {
    pub(super) fn push(&mut self, chunk: &[u8]) -> Vec<Result<ResidentEvent, String>> {
        self.0.extend_from_slice(chunk);
        let mut events = Vec::new();
        while let Some(index) = self.0.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.0.drain(..=index).collect();
            let line = &line[..line.len().saturating_sub(1)];
            if !line.is_empty() {
                events.push(serde_json::from_slice(line).map_err(|error| error.to_string()));
            }
        }
        events
    }
}

pub(super) fn handle_control(command: WorkerCommand, ready: bool) -> ControlResult {
    match command {
        WorkerCommand::Health { reply } => {
            let result = if ready {
                Ok(())
            } else {
                Err("resident Parakeet is not ready".into())
            };
            let _ = reply.send(result);
            ControlResult::Handled
        }
        WorkerCommand::Shutdown => ControlResult::Shutdown,
        command => ControlResult::Forward(command),
    }
}

pub(super) fn handle_transcribe(
    command: WorkerCommand,
    child: &mut CommandChild,
    pending: &mut HashMap<String, PendingReply>,
) {
    let WorkerCommand::Transcribe {
        request,
        temp_path,
        reply,
    } = command
    else {
        return;
    };
    let id = request.id.clone();
    pending.insert(
        id.clone(),
        PendingReply::Transcribe {
            _temp_path: temp_path,
            reply,
        },
    );
    if let Err(error) = write_request(child, &request)
        && let Some(reply) = pending.remove(&id)
    {
        fail_reply(reply, error);
    }
}

pub(super) fn handle_event(
    event: ResidentEvent,
    pending: &mut HashMap<String, PendingReply>,
) -> bool {
    match event {
        ResidentEvent::Result { id, text, model } => {
            let _ = model;
            if let Some(PendingReply::Transcribe { reply, .. }) = pending.remove(&id) {
                let _ = reply.send(Ok(text));
            }
        }
        ResidentEvent::Error {
            id: Some(id),
            message,
        } => {
            if let Some(reply) = pending.remove(&id) {
                fail_reply(reply, message);
            }
        }
        ResidentEvent::Error { id: None, message } => {
            log::warn!("resident Parakeet error: {message}");
        }
        ResidentEvent::Fatal { message } => {
            log::warn!("resident Parakeet fatal: {message}");
            return true;
        }
        ResidentEvent::Pong { id } => {
            log::debug!("resident Parakeet unsolicited pong: {id}");
        }
        ResidentEvent::Ready => {}
    }
    false
}

pub(super) fn fail_all(pending: &mut HashMap<String, PendingReply>, message: &str) {
    for (_, reply) in pending.drain() {
        fail_reply(reply, message.to_string());
    }
}

pub(super) fn reject_transcribe(command: WorkerCommand, message: &str) {
    if let WorkerCommand::Transcribe { reply, .. } = command {
        let _ = reply.send(Err(message.into()));
    }
}

fn write_request(child: &mut CommandChild, request: &ResidentRequest) -> Result<(), String> {
    let mut line = serde_json::to_vec(request).map_err(|error| error.to_string())?;
    line.push(b'\n');
    child.write(&line).map_err(|error| error.to_string())
}

fn fail_reply(reply: PendingReply, message: String) {
    let PendingReply::Transcribe { reply, .. } = reply;
    let _ = reply.send(Err(message));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_health_is_ready_during_serve() {
        let (reply, mut receiver) = oneshot::channel();
        assert!(matches!(
            handle_control(WorkerCommand::Health { reply }, true),
            ControlResult::Handled
        ));
        assert!(matches!(receiver.try_recv(), Ok(Ok(()))));
    }

    #[test]
    fn local_health_is_not_ready_without_a_ready_child() {
        let (reply, mut receiver) = oneshot::channel();
        assert!(matches!(
            handle_control(WorkerCommand::Health { reply }, false),
            ControlResult::Handled
        ));
        assert!(matches!(receiver.try_recv(), Ok(Err(_))));
    }

    #[test]
    fn ndjson_buffer_preserves_fragmented_event() {
        let mut buffer = NdjsonBuffer::default();
        assert!(buffer.push(b"{\"type\":\"res").is_empty());
        let events =
            buffer.push(b"ult\",\"id\":\"a\",\"text\":\"hello\",\"model\":\"parakeet\"}\n");
        assert_eq!(events.len(), 1);
        assert!(matches!(events.into_iter().next().unwrap().unwrap(),
            ResidentEvent::Result { id, text, .. } if id == "a" && text == "hello"));
    }

    #[test]
    fn ndjson_buffer_returns_multiple_events_from_one_chunk() {
        let mut buffer = NdjsonBuffer::default();
        let events = buffer.push(b"{\"type\":\"ready\"}\n{\"type\":\"pong\",\"id\":\"b\"}\n");
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], Ok(ResidentEvent::Ready)));
        assert!(matches!(&events[1], Ok(ResidentEvent::Pong { id }) if id == "b"));
    }
}
