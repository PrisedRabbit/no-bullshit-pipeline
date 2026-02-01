//! Shared audio buffers for real-time mixing
//!
//! Mic and System audio recorders push samples here.
//! Mixer reads from both and writes directly to output.

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// Shared stereo audio buffer
pub struct SharedAudioBuffer {
    left: Mutex<VecDeque<f32>>,
    right: Mutex<VecDeque<f32>>,
    sample_rate: Mutex<u32>,
}

impl SharedAudioBuffer {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            left: Mutex::new(VecDeque::with_capacity(48000 * 2)), // 2 seconds buffer
            right: Mutex::new(VecDeque::with_capacity(48000 * 2)),
            sample_rate: Mutex::new(48000),
        })
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
        let mut left = self.left.lock().unwrap();
        let mut right = self.right.lock().unwrap();

        for chunk in samples.chunks(2) {
            if chunk.len() == 2 {
                left.push_back(chunk[0]);
                right.push_back(chunk[1]);
            } else if chunk.len() == 1 {
                // Mono - duplicate to both channels
                left.push_back(chunk[0]);
                right.push_back(chunk[0]);
            }
        }
    }

    /// Push planar samples (separate L and R vectors)
    pub fn push_planar(&self, left_samples: &[f32], right_samples: &[f32]) {
        if let Ok(mut left) = self.left.lock() {
            left.extend(left_samples.iter().cloned());
        }
        if let Ok(mut right) = self.right.lock() {
            right.extend(right_samples.iter().cloned());
        }
    }

    /// Get available sample count (min of left/right)
    pub fn available(&self) -> usize {
        let left_len = self.left.lock().map(|l| l.len()).unwrap_or(0);
        let right_len = self.right.lock().map(|r| r.len()).unwrap_or(0);
        left_len.min(right_len)
    }

    /// Pop up to `count` stereo frames, returns (left, right) vectors
    pub fn pop(&self, count: usize) -> (Vec<f32>, Vec<f32>) {
        let mut left_out = Vec::with_capacity(count);
        let mut right_out = Vec::with_capacity(count);

        if let Ok(mut left) = self.left.lock() {
            for _ in 0..count {
                if let Some(s) = left.pop_front() {
                    left_out.push(s);
                } else {
                    break;
                }
            }
        }

        if let Ok(mut right) = self.right.lock() {
            for _ in 0..count {
                if let Some(s) = right.pop_front() {
                    right_out.push(s);
                } else {
                    break;
                }
            }
        }

        (left_out, right_out)
    }

    /// Clear all buffered samples
    pub fn clear(&self) {
        if let Ok(mut left) = self.left.lock() {
            left.clear();
        }
        if let Ok(mut right) = self.right.lock() {
            right.clear();
        }
    }
}

// Global shared buffers
lazy_static::lazy_static! {
    pub static ref MIC_BUFFER: Arc<SharedAudioBuffer> = SharedAudioBuffer::new();
    pub static ref SYSTEM_BUFFER: Arc<SharedAudioBuffer> = SharedAudioBuffer::new();
}
