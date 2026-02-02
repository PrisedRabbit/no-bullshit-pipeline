use anyhow::Result;
use cidre::{cat, cf, core_audio as ca, os};
use ringbuf::{
    traits::{Consumer, Producer, Split, Observer},
    HeapProd, HeapRb,
};
use std::fs::File;
use std::num::{NonZeroU32, NonZeroU8};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};
use std::thread;
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoder};

use crate::audio_processing::LoudnessNormalizer;

// System audio level for visualization
lazy_static::lazy_static! {
    static ref SYSTEM_AUDIO_LEVEL: AtomicU32 = AtomicU32::new(0);
}

fn set_system_audio_level(level: f32) {
    let level_u32 = level.clamp(0.0, 1.0).to_bits();
    SYSTEM_AUDIO_LEVEL.store(level_u32, Ordering::Relaxed);
}

pub fn get_system_audio_level() -> f32 {
    f32::from_bits(SYSTEM_AUDIO_LEVEL.load(Ordering::Relaxed))
}

pub fn reset_system_audio_level() {
    SYSTEM_AUDIO_LEVEL.store(0, Ordering::Relaxed);
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

pub struct SystemAudioRecorder {
    should_stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

struct AudioContext {
    producer: HeapProd<f32>,
}

impl SystemAudioRecorder {
    pub fn new(output_path: std::path::PathBuf, skip_file: bool) -> Result<Self> {
        let should_stop = Arc::new(AtomicBool::new(false));
        let should_stop_clone = should_stop.clone();

        let handle = thread::spawn(move || {
            if let Err(e) = run_audio_capture(output_path, should_stop_clone, skip_file) {
                eprintln!("System audio capture error: {:?}", e);
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

pub fn start_system_capture(output_path: std::path::PathBuf, skip_file: bool) -> Result<SystemAudioRecorder> {
    SystemAudioRecorder::new(output_path, skip_file)
}

fn run_audio_capture(mut path: std::path::PathBuf, should_stop: Arc<AtomicBool>, skip_file: bool) -> Result<()> {
    // Switch extension to .ogg
    path.set_extension("ogg");

    // 1. Get Default Output Device
    let output_device = ca::System::default_output_device()?;
    let output_uid = output_device.uid()?;

    // 2. Create Process Tap (Stereo Global Tap)
    // Note: cidre::ns::Array::new() creates an empty array to exclude NO processes (capture all)
    let tap_desc = ca::TapDesc::with_stereo_global_tap_excluding_processes(&cidre::ns::Array::new());
    let tap = tap_desc.create_process_tap()?;

    // 3. Create Aggregate Device Descriptor
    let sub_tap = cf::DictionaryOf::with_keys_values(
        &[ca::sub_device_keys::uid()],
        &[tap.uid().unwrap().as_type_ref()],
    );

    let agg_desc = cf::DictionaryOf::with_keys_values(
        &[
            ca::aggregate_device_keys::is_private(),
            ca::aggregate_device_keys::tap_auto_start(),
            ca::aggregate_device_keys::name(),
            ca::aggregate_device_keys::main_sub_device(),
            ca::aggregate_device_keys::uid(),
            ca::aggregate_device_keys::tap_list(),
        ],
        &[
            cf::Boolean::value_true().as_type_ref(),
            cf::Boolean::value_true(),
            cf::str!(c"nbp-audio-tap").as_type_ref(),
            &output_uid,
            &cf::Uuid::new().to_cf_string(),
            &cf::ArrayOf::from_slice(&[sub_tap.as_ref()]),
        ],
    );

    // 4. Create Aggregate Device
    let agg_device = ca::AggregateDevice::with_desc(&agg_desc)?;

    // 5. Setup Ring Buffer
    // 48kHz Stereo float = 48000 * 2 * 4 bytes/sec = 384KB/sec.
    // 4 seconds buffer = ~1.5MB.
    // HeapRb holds items (f32). 48000 * 2 * 4 = 384000 items.
    let ring_buffer_size = 48000 * 2 * 4; 
    let rb = HeapRb::<f32>::new(ring_buffer_size);
    let (producer, mut consumer) = rb.split();

    let mut ctx = AudioContext { producer };

    // 6. Define IO Proc
    extern "C" fn audio_proc(
        _device: ca::Device,
        _now: &cat::AudioTimeStamp,
        input_data: &cat::AudioBufList<1>,
        _input_time: &cat::AudioTimeStamp,
        _output_data: &mut cat::AudioBufList<1>,
        _output_time: &cat::AudioTimeStamp,
        ctx: Option<&mut AudioContext>,
    ) -> os::Status {
        let ctx = ctx.unwrap();
        
        // Check number of buffers to determine Planar vs Interleaved
        // input_data.number_buffers is u32
        let num_buffers = input_data.number_buffers;
        let buffers = &input_data.buffers;

        if num_buffers == 2 {
            // PLANAR: L in buffer[0], R in buffer[1]
            // Access buffers[0] safely via slice
            let buf_l = &buffers[0];
            
            // Access buffers[1] using unsafe pointer arithmetic because 
            // AudioBufList<1> generic limits slice size to 1.
            let buf_r = unsafe { &*buffers.as_ptr().add(1) };
            
            let ptr_l = buf_l.data as *const f32;
            let ptr_r = buf_r.data as *const f32;
            let len_l = (buf_l.data_bytes_size as usize) / std::mem::size_of::<f32>();
            
            if !ptr_l.is_null() && !ptr_r.is_null() && len_l > 0 {
                let samples_l = unsafe { std::slice::from_raw_parts(ptr_l, len_l) };
                let samples_r = unsafe { std::slice::from_raw_parts(ptr_r, len_l) };
                
                // Interleave manually: L, R, L, R...
                let mut interleaved = Vec::with_capacity(len_l * 2);
                for i in 0..len_l {
                    interleaved.push(samples_l[i]);
                    // Safety check if buf_r len is different? Assume audio block alignment.
                    if i < samples_r.len() {
                        interleaved.push(samples_r[i]);
                    } else {
                        interleaved.push(0.0);
                    }
                }
                ctx.producer.push_slice(&interleaved);
            }
        
        } else if !buffers.is_empty() {
             // INTERLEAVED (or Mono): All channels in buffer[0]
             let buffer = &buffers[0];
             let ptr = buffer.data as *const f32;
             // data_bytes_size tells absolute size.
             let len = (buffer.data_bytes_size as usize) / std::mem::size_of::<f32>();
             if !ptr.is_null() && len > 0 {
                 let samples = unsafe { std::slice::from_raw_parts(ptr, len) };
                 // Push to ring buffer
                 ctx.producer.push_slice(samples);
             }
        }
        
        os::Status::NO_ERR
    }

    // 7. Create IO Proc ID
    let proc_id = agg_device.create_io_proc_id(audio_proc, Some(&mut ctx))?;

    // 8. Start Device
    let _started_device = ca::device_start(agg_device, Some(proc_id))?;

    // 9. Encoder & Normalizer Setup
    let asbd = tap.asbd()?;
    let sample_rate = asbd.sample_rate as u32;
    let channels = asbd.channels_per_frame;

    let mut normalizer = LoudnessNormalizer::new(channels, sample_rate)?;

    // Setup OGG Vorbis Encoder (only if not skipping file output)
    let mut encoder: Option<VorbisEncoder<File>> = if !skip_file {
        let output_file = File::create(&path)?;
        // Quality 0.4 is variable bitrate (VBR) target quality (~128kbps)
        Some(VorbisEncoder::new(
            0, // Stream serial (0 for now)
            [("", ""); 0], // No comments
            NonZeroU32::new(sample_rate).ok_or(anyhow::anyhow!("Invalid sample rate"))?,
            NonZeroU8::new(channels as u8).ok_or(anyhow::anyhow!("Invalid channels"))?,
            VorbisBitrateManagementStrategy::QualityVbr { target_quality: 0.4 },
            None, // Default page size optimization
            output_file,
        )?)
    } else {
        println!("System: Skipping file output (mix-only mode)");
        None
    };
    
    // 10. Continuous Timeline Loop
    // Write audio at exactly 48kHz regardless of whether audio arrives from tap
    let chunk_size = 4096;
    let mut buffer = vec![0.0f32; chunk_size];
    
    let start_time = std::time::Instant::now();
    let mut total_frames_written: u64 = 0;
    
    // Tick every 10ms to maintain timeline
    let tick_duration = std::time::Duration::from_millis(10);

    while !should_stop.load(Ordering::Relaxed) {
        // Calculate how many frames SHOULD exist by now
        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let expected_frames = (elapsed_secs * sample_rate as f64) as u64;
        
        // If we're behind, we need to write frames
        if total_frames_written < expected_frames {
            // Write UP TO 100ms at a time to avoid huge blocks, but loop fast
            let catch_up_limit = (sample_rate as f64 * 0.1) as usize; 
            let frames_needed = (expected_frames - total_frames_written).min(catch_up_limit as u64) as usize;
            
            // Try to get real audio from ring buffer
            let available = consumer.occupied_len();
            let mut frames_remaining = frames_needed;
            
            // 1. Consume available audio first
            let audio_frames_available = available / channels as usize;
            
            if audio_frames_available > 0 {
                // Ensure we don't exceed buffer capacity
                let max_frames_in_buffer = chunk_size / channels as usize;
                // Use as much audio as we can, up to what we need, but limited by buffer
                let frames_to_read = audio_frames_available.min(frames_remaining).min(max_frames_in_buffer);
                let samples_to_read = frames_to_read * channels as usize;
                
                let n = consumer.pop_slice(&mut buffer[..samples_to_read]);
                
                if n > 0 {
                    let raw_samples = &buffer[..n];

                    // Update audio level for visualization (dynamic scaling)
                    let rms = calculate_rms(raw_samples);
                    // Soft compression: sqrt provides natural curve that boosts quiet signals
                    // while preventing loud signals from clipping
                    let scaled = (rms * 4.0).sqrt().min(1.0);
                    set_system_audio_level(scaled);

                    let normalized_samples = normalizer.normalize_loudness(raw_samples);
                    let frames_encoded = normalized_samples.len() / channels as usize;
                    
                    let planar_samples = if channels == 2 {
                        let mut left = Vec::with_capacity(frames_encoded);
                        let mut right = Vec::with_capacity(frames_encoded);
                        for chunk in normalized_samples.chunks(2) {
                            if chunk.len() == 2 {
                                left.push(chunk[0]);
                                right.push(chunk[1]);
                            }
                        }
                        vec![left, right]
                    } else {
                        vec![normalized_samples]
                    };
                    
                    // Push to shared buffer for real-time mixing
                    if planar_samples.len() >= 2 {
                        crate::audio_processing::SYSTEM_BUFFER.push_planar(&planar_samples[0], &planar_samples[1]);
                    } else if planar_samples.len() == 1 {
                        // Mono - push same to both channels
                        crate::audio_processing::SYSTEM_BUFFER.push_planar(&planar_samples[0], &planar_samples[0]);
                    }

                    // Encode to file (if not skipping)
                    if let Some(ref mut enc) = encoder {
                        let slices_ref: Vec<&[f32]> = planar_samples.iter().map(|v| v.as_slice()).collect();
                        enc.encode_audio_block(&slices_ref)?;
                    }
                    total_frames_written += frames_encoded as u64;

                    // Decrease remaining count
                    if frames_remaining >= frames_encoded {
                        frames_remaining -= frames_encoded;
                    } else {
                        frames_remaining = 0;
                    }
                }
            }
            
            // 2. Fill remainder with silence (only if encoding to file)
            if frames_remaining > 0 {
                if let Some(ref mut enc) = encoder {
                    let silence = vec![0.0f32; frames_remaining];
                    let silence_planar = if channels == 2 {
                        vec![silence.clone(), silence]
                    } else {
                        vec![silence]
                    };

                    let slices_ref: Vec<&[f32]> = silence_planar.iter().map(|v| v.as_slice()).collect();
                    enc.encode_audio_block(&slices_ref)?;
                }
                total_frames_written += frames_remaining as u64;
            }
            
            // If we still need to catch up (hit limit), continue loop immediately without sleep
            if total_frames_written < expected_frames {
                continue;
            }
        }
        
        // Sleep until next tick if we are caught up
        thread::sleep(tick_duration);
    }
    
    // STOP THE STREAM NOW so no new data enters buffer during drain
    drop(_started_device);
    
    // Drain any remaining audio from buffer
    loop {
        let available = consumer.occupied_len();
        if available == 0 {
            break;
        }
        
        let mut to_read = chunk_size.min(available);
        to_read -= to_read % (channels as usize);
        
        if to_read == 0 {
            break;
        }
        
        let n = consumer.pop_slice(&mut buffer[..to_read]);
        
        if n > 0 {
            let raw_samples = &buffer[..n];
            let normalized_samples = normalizer.normalize_loudness(raw_samples);
            let frames = normalized_samples.len() / channels as usize;
            
            let planar_samples = if channels == 2 {
                let mut left = Vec::with_capacity(frames);
                let mut right = Vec::with_capacity(frames);
                for chunk in normalized_samples.chunks(2) {
                    if chunk.len() == 2 {
                        left.push(chunk[0]);
                        right.push(chunk[1]);
                    }
                }
                vec![left, right]
            } else {
                vec![normalized_samples]
            };
            
            if let Some(ref mut enc) = encoder {
                let slices_ref: Vec<&[f32]> = planar_samples.iter().map(|v| v.as_slice()).collect();
                if let Err(e) = enc.encode_audio_block(&slices_ref) {
                    eprintln!("Error encoding remaining system audio: {}", e);
                    break;
                }
            }
            total_frames_written += frames as u64;
        }
    }
    
    // FILL SILENCE and finish (only if encoding to file)
    if let Some(mut enc) = encoder {
        let elapsed = start_time.elapsed().as_secs_f64();
        let expected_frames = (elapsed * sample_rate as f64) as u64;

        println!("System Audio Stop: Timer={:.3}s, Rate={}, Written={}, Expected={}, Diff={}",
            elapsed, sample_rate, total_frames_written, expected_frames, expected_frames as i64 - total_frames_written as i64);

        if total_frames_written < expected_frames {
            let missing_frames = expected_frames - total_frames_written;
            println!("System Audio Underrun: Padding with {} frames of silence ({:.2}s)",
                missing_frames, missing_frames as f64 / sample_rate as f64);

            // Create silence block (e.g., 4096 frames at a time)
            let silence_chunk_size = 4096;
            let silence_vec = vec![0.0f32; silence_chunk_size];
            let silence_planar = if channels == 2 {
                vec![silence_vec.clone(), silence_vec]
            } else {
                vec![silence_vec]
            };
            let silence_refs: Vec<&[f32]> = silence_planar.iter().map(|v| v.as_slice()).collect();

            let mut remaining = missing_frames;
            while remaining > 0 {
                let chunk = remaining.min(silence_chunk_size as u64);
                // Slice the silence vectors to match chunk size
                let partial_refs: Vec<&[f32]> = silence_refs.iter().map(|v| &v[..chunk as usize]).collect();

                if let Err(e) = enc.encode_audio_block(&partial_refs) {
                    eprintln!("Error writing silence padding: {}", e);
                    break;
                }
                remaining -= chunk;
            }
        } else {
            println!("System Audio: No padding needed (Written >= Expected)");
        }

        // Finish encoding
        enc.finish()?;
        println!("System Audio: Encoder finished.");
    }

    Ok(())
}
