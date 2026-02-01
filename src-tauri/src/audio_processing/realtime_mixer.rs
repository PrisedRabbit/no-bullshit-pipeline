use anyhow::Result;
use std::fs::File;
use std::num::{NonZeroU32, NonZeroU8};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoder};

use super::{MIC_BUFFER, SYSTEM_BUFFER};

/// Real-time mixer that reads from shared buffers and writes mixed output
pub struct RealtimeMixer {
    should_stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl RealtimeMixer {
    pub fn new(output_path: PathBuf) -> Result<Self> {
        // Clear buffers before starting
        MIC_BUFFER.clear();
        SYSTEM_BUFFER.clear();

        let should_stop = Arc::new(AtomicBool::new(false));
        let should_stop_clone = should_stop.clone();

        let handle = thread::spawn(move || {
            if let Err(e) = run_realtime_mixer(output_path, should_stop_clone) {
                eprintln!("Real-time mixer error: {:?}", e);
            }
        });

        Ok(Self {
            should_stop,
            handle: Some(handle),
        })
    }

    pub fn stop(&mut self) {
        self.should_stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run_realtime_mixer(output_path: PathBuf, should_stop: Arc<AtomicBool>) -> Result<()> {
    println!("Real-time mixer: Starting (buffer-based, continuous timeline)");

    // Output format: 48kHz stereo
    let sample_rate = 48000u32;
    let channels = 2u8;

    let output_file = File::create(&output_path)?;
    let mut encoder = VorbisEncoder::new(
        0,
        [("", ""); 0],
        NonZeroU32::new(sample_rate).ok_or(anyhow::anyhow!("Invalid sample rate"))?,
        NonZeroU8::new(channels).ok_or(anyhow::anyhow!("Invalid channels"))?,
        VorbisBitrateManagementStrategy::QualityVbr { target_quality: 0.5 },
        None,
        output_file,
    )?;

    // Continuous timeline tracking (like mic/system recorders)
    let start_time = std::time::Instant::now();
    let mut total_frames_written: u64 = 0;

    // Tick every 10ms to maintain timeline
    let tick_duration = Duration::from_millis(10);

    // Wait a bit for buffers to start filling
    thread::sleep(Duration::from_millis(50));

    while !should_stop.load(Ordering::Relaxed) {
        // Calculate how many frames SHOULD exist by now
        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let expected_frames = (elapsed_secs * sample_rate as f64) as u64;

        // If we're behind, we need to write frames
        if total_frames_written < expected_frames {
            // Write up to 100ms at a time to avoid huge blocks
            let catch_up_limit = (sample_rate as f64 * 0.1) as usize;
            let frames_needed = (expected_frames - total_frames_written).min(catch_up_limit as u64) as usize;

            let mic_avail = MIC_BUFFER.available();
            let sys_avail = SYSTEM_BUFFER.available();

            let mut frames_remaining = frames_needed;

            // 1. Process available audio from buffers
            let audio_frames_available = mic_avail.max(sys_avail);

            if audio_frames_available > 0 {
                let frames_to_process = audio_frames_available.min(frames_remaining);

                // Pop from both buffers
                let (mic_left, mic_right) = MIC_BUFFER.pop(frames_to_process);
                let (sys_left, sys_right) = SYSTEM_BUFFER.pop(frames_to_process);

                // Mix
                let frame_count = mic_left.len().max(sys_left.len());
                if frame_count > 0 {
                    let mut mixed_left = Vec::with_capacity(frame_count);
                    let mut mixed_right = Vec::with_capacity(frame_count);

                    for i in 0..frame_count {
                        let ml = mic_left.get(i).copied().unwrap_or(0.0);
                        let mr = mic_right.get(i).copied().unwrap_or(0.0);
                        let sl = sys_left.get(i).copied().unwrap_or(0.0);
                        let sr = sys_right.get(i).copied().unwrap_or(0.0);

                        // Mix with soft clipping
                        mixed_left.push(soft_clip(ml + sl));
                        mixed_right.push(soft_clip(mr + sr));
                    }

                    let slices: Vec<&[f32]> = vec![&mixed_left, &mixed_right];
                    encoder.encode_audio_block(&slices)?;
                    total_frames_written += frame_count as u64;

                    if frames_remaining >= frame_count {
                        frames_remaining -= frame_count;
                    } else {
                        frames_remaining = 0;
                    }
                }
            }

            // 2. Fill remainder with silence to maintain timeline
            if frames_remaining > 0 {
                let silence = vec![0.0f32; frames_remaining];
                let slices: Vec<&[f32]> = vec![&silence, &silence];
                encoder.encode_audio_block(&slices)?;
                total_frames_written += frames_remaining as u64;
            }

            // Continue immediately if we still need to catch up
            if total_frames_written < expected_frames {
                continue;
            }
        }

        // Sleep until next tick if caught up
        thread::sleep(tick_duration);
    }

    // Drain remaining samples from buffers
    drain_and_encode(&mut encoder, 4096)?;

    // Final padding to match wall-clock duration
    let elapsed = start_time.elapsed().as_secs_f64();
    let expected_frames = (elapsed * sample_rate as f64) as u64;

    if total_frames_written < expected_frames {
        let missing_frames = expected_frames - total_frames_written;
        let silence_chunk = 4096;
        let silence = vec![0.0f32; silence_chunk];

        let mut remaining = missing_frames;
        while remaining > 0 {
            let chunk = remaining.min(silence_chunk as u64) as usize;
            let slices: Vec<&[f32]> = vec![&silence[..chunk], &silence[..chunk]];
            encoder.encode_audio_block(&slices)?;
            remaining -= chunk as u64;
        }
    }

    encoder.finish()?;
    println!("Real-time mixer: Finished. Wrote {} frames ({:.2}s)",
        total_frames_written, total_frames_written as f64 / sample_rate as f64);
    Ok(())
}

/// Drain remaining samples from buffers
fn drain_and_encode(encoder: &mut VorbisEncoder<File>, block_size: usize) -> Result<()> {
    loop {
        let mic_avail = MIC_BUFFER.available();
        let sys_avail = SYSTEM_BUFFER.available();

        if mic_avail == 0 && sys_avail == 0 {
            break;
        }

        let process_count = mic_avail.max(sys_avail).min(block_size);
        let (mic_left, mic_right) = MIC_BUFFER.pop(process_count);
        let (sys_left, sys_right) = SYSTEM_BUFFER.pop(process_count);

        let frame_count = mic_left.len().max(sys_left.len());
        if frame_count == 0 {
            break;
        }

        let mut mixed_left = Vec::with_capacity(frame_count);
        let mut mixed_right = Vec::with_capacity(frame_count);

        for i in 0..frame_count {
            let ml = mic_left.get(i).copied().unwrap_or(0.0);
            let mr = mic_right.get(i).copied().unwrap_or(0.0);
            let sl = sys_left.get(i).copied().unwrap_or(0.0);
            let sr = sys_right.get(i).copied().unwrap_or(0.0);

            mixed_left.push(soft_clip(ml + sl));
            mixed_right.push(soft_clip(mr + sr));
        }

        let slices: Vec<&[f32]> = vec![&mixed_left, &mixed_right];
        encoder.encode_audio_block(&slices)?;
    }
    Ok(())
}

/// Soft clipping to prevent harsh distortion
#[inline]
fn soft_clip(x: f32) -> f32 {
    if x.abs() <= 1.0 {
        x
    } else {
        x.signum() * (1.0 - (-x.abs() + 1.0).exp().min(1.0))
    }
}
