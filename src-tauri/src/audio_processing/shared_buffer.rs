//! Shared audio buffers for real-time mixing
//!
//! Mic and System audio recorders push samples here.
//! Mixer reads from both and writes directly to output.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Max buffer capacity: 10 seconds at 48kHz. If mixer falls behind,
/// oldest samples are dropped to prevent unbounded memory growth.
const MAX_BUFFER_SAMPLES: usize = 48000 * 10;

/// Shared stereo audio buffer.
/// Both channels are protected by a single mutex to guarantee atomic access
/// and prevent L/R channel desynchronization.
pub struct SharedAudioBuffer {
    channels: Mutex<(VecDeque<f32>, VecDeque<f32>)>,
    sample_rate: Mutex<u32>,
}

impl SharedAudioBuffer {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            channels: Mutex::new((
                VecDeque::with_capacity(48000 * 2),
                VecDeque::with_capacity(48000 * 2),
            )),
            sample_rate: Mutex::new(48000),
        })
    }

    /// Trim buffer to max capacity, dropping oldest samples
    fn enforce_capacity(buf: &mut VecDeque<f32>) {
        if buf.len() > MAX_BUFFER_SAMPLES {
            let excess = buf.len() - MAX_BUFFER_SAMPLES;
            buf.drain(..excess);
        }
    }

    pub fn set_sample_rate(&self, rate: u32) {
        if let Ok(mut sr) = self.sample_rate.lock() {
            *sr = rate;
        }
    }

    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate.lock().map(|sr| *sr).unwrap_or(48000)
    }

    /// Push stereo samples (interleaved: L, R, L, R, ...)
    pub fn push_interleaved(&self, samples: &[f32]) {
        let Ok(mut ch) = self.channels.lock() else {
            return; // Poisoned mutex — skip rather than panic in audio thread
        };

        for chunk in samples.chunks(2) {
            if chunk.len() == 2 {
                ch.0.push_back(chunk[0]);
                ch.1.push_back(chunk[1]);
            } else if chunk.len() == 1 {
                ch.0.push_back(chunk[0]);
                ch.1.push_back(chunk[0]);
            }
        }
        Self::enforce_capacity(&mut ch.0);
        Self::enforce_capacity(&mut ch.1);
    }

    /// Push planar samples (separate L and R vectors)
    pub fn push_planar(&self, left_samples: &[f32], right_samples: &[f32]) {
        let Ok(mut ch) = self.channels.lock() else {
            return; // Poisoned mutex — skip rather than panic in audio thread
        };

        ch.0.extend(left_samples.iter().cloned());
        ch.1.extend(right_samples.iter().cloned());
        Self::enforce_capacity(&mut ch.0);
        Self::enforce_capacity(&mut ch.1);
    }

    /// Get available sample count (min of left/right)
    pub fn available(&self) -> usize {
        let Ok(ch) = self.channels.lock() else {
            return 0;
        };
        ch.0.len().min(ch.1.len())
    }

    /// Pop up to `count` stereo frames, returns (left, right) vectors
    pub fn pop(&self, count: usize) -> (Vec<f32>, Vec<f32>) {
        let mut left_out = Vec::with_capacity(count);
        let mut right_out = Vec::with_capacity(count);

        let Ok(mut ch) = self.channels.lock() else {
            return (left_out, right_out);
        };

        for _ in 0..count {
            match (ch.0.pop_front(), ch.1.pop_front()) {
                (Some(l), Some(r)) => {
                    left_out.push(l);
                    right_out.push(r);
                }
                _ => break,
            }
        }

        (left_out, right_out)
    }

    /// Clear all buffered samples
    pub fn clear(&self) {
        if let Ok(mut ch) = self.channels.lock() {
            ch.0.clear();
            ch.1.clear();
        }
    }
}

/// Max buffer capacity for transcription: 10 seconds at 16kHz mono
const MAX_TRANSCRIPTION_SAMPLES: usize = 16000 * 10;

/// Shared mono audio buffer for transcription.
/// Single channel at 16kHz, matching Whisper's expected input format.
pub struct MonoAudioBuffer {
    samples: Mutex<VecDeque<f32>>,
    sample_rate: Mutex<u32>,
}

impl MonoAudioBuffer {
    pub fn new(sample_rate: u32) -> Arc<Self> {
        Arc::new(Self {
            samples: Mutex::new(VecDeque::with_capacity(sample_rate as usize * 2)),
            sample_rate: Mutex::new(sample_rate),
        })
    }

    fn enforce_capacity(buf: &mut VecDeque<f32>) {
        if buf.len() > MAX_TRANSCRIPTION_SAMPLES {
            let excess = buf.len() - MAX_TRANSCRIPTION_SAMPLES;
            buf.drain(..excess);
        }
    }

    pub fn set_sample_rate(&self, rate: u32) {
        if let Ok(mut sr) = self.sample_rate.lock() {
            *sr = rate;
        }
    }

    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate.lock().map(|sr| *sr).unwrap_or(16000)
    }

    /// Push mono samples
    pub fn push(&self, samples: &[f32]) {
        let Ok(mut buf) = self.samples.lock() else {
            return; // Poisoned mutex — skip rather than panic in audio thread
        };
        buf.extend(samples.iter().cloned());
        Self::enforce_capacity(&mut buf);
    }

    /// Get available sample count
    pub fn available(&self) -> usize {
        let Ok(buf) = self.samples.lock() else {
            return 0;
        };
        buf.len()
    }

    /// Pop up to `count` mono samples
    pub fn pop(&self, count: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(count);
        let Ok(mut buf) = self.samples.lock() else {
            return out;
        };
        for _ in 0..count {
            match buf.pop_front() {
                Some(s) => out.push(s),
                None => break,
            }
        }
        out
    }

    /// Clear all buffered samples
    pub fn clear(&self) {
        if let Ok(mut buf) = self.samples.lock() {
            buf.clear();
        }
    }
}

// Global shared buffers
lazy_static::lazy_static! {
    pub static ref MIC_BUFFER: Arc<SharedAudioBuffer> = SharedAudioBuffer::new();
    pub static ref SYSTEM_BUFFER: Arc<SharedAudioBuffer> = SharedAudioBuffer::new();
    pub static ref TRANSCRIPTION_BUFFER: Arc<MonoAudioBuffer> = MonoAudioBuffer::new(16000);
}
