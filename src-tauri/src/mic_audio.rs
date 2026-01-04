use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{
    traits::{Consumer, Producer, Split, Observer},
    HeapRb,
};
use std::fs::File;
use std::num::{NonZeroU32, NonZeroU8};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoder};

use crate::audio_processing::LoudnessNormalizer;

pub struct MicAudioRecorder {
    should_stop: Arc<AtomicBool>,
    processing_thread: Option<thread::JoinHandle<()>>,
    // Stream must be kept alive to keep recording
    stream: Option<cpal::Stream>,
}

impl MicAudioRecorder {
    pub fn new(output_path: std::path::PathBuf) -> Result<Self> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or(anyhow::anyhow!("No input device available"))?;
        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        
        println!("Mic Config: Rate={}, Channels={}", sample_rate, channels);

        // Ring Buffer (1 second capacity is usually enough for processing thread to catch up)
        // 48k * 2ch * 4bytes = ~384KB.
        let ring_buffer_size = (sample_rate as usize) * (channels as usize) * 8; // generous buffer
        let rb = HeapRb::<f32>::new(ring_buffer_size);
        let (mut producer, consumer) = rb.split();

        // Error callback
        let err_fn = move |err| {
            eprintln!("an error occurred on stream: {}", err);
        };

        // Build stream with appropriate format handling
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    let _ = producer.push_slice(data);
                },
                err_fn,
                None
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &_| {
                    for &sample in data {
                        let s = (sample as f32) / (i16::MAX as f32);
                        let _ = producer.push_slice(&[s]);
                    }
                },
                err_fn,
                None
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &_| {
                    for &sample in data {
                        let s = (sample as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0);
                        let _ = producer.push_slice(&[s]);
                    }
                },
                err_fn,
                None
            )?,
            _ => return Err(anyhow::anyhow!("Unsupported sample format")),
        };

        stream.play()?;

        // Processing Thread
        let should_stop = Arc::new(AtomicBool::new(false));
         let should_stop_clone = should_stop.clone();
        
        let path = output_path.clone();

        let handle = thread::spawn(move || {
            if let Err(e) = run_audio_processing(
                path, 
                should_stop_clone, 
                consumer, 
                channels as u32, 
                sample_rate
            ) {
                eprintln!("Mic audio processing error: {:?}", e);
            }
        });

        Ok(Self {
            should_stop,
            processing_thread: Some(handle),
            stream: Some(stream),
        })
    }

    pub fn stop(&mut self) {
        // Stop the stream FIRST to prevent more audio from being captured
        drop(self.stream.take());
        
        // Then signal encoder to stop and wait for it to finish
        self.should_stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.processing_thread.take() {
            let _ = handle.join();
        }
    }
}

pub fn start_mic_capture(output_path: std::path::PathBuf) -> Result<MicAudioRecorder> {
    MicAudioRecorder::new(output_path)
}

fn run_audio_processing(
    path: std::path::PathBuf,
    should_stop: Arc<AtomicBool>,
    mut consumer: ringbuf::HeapCons<f32>,
    input_channels: u32,
    sample_rate: u32,
) -> Result<()> {
    // We always output Stereo 48kHz (or input sample rate)
    // If input is Mono, we duplicate.
    // If input is Stereo, we pass through.
    
    let output_channels = 2; // Target Stereo
    
    // Setup Normalizer
    let mut normalizer = LoudnessNormalizer::new(input_channels, sample_rate)?;

    // Setup Encoder
    // Quality 0.4 (~128kbps)
    let output_file = File::create(&path)?;
    let mut encoder = VorbisEncoder::new(
        0,
        [("", ""); 0],
        NonZeroU32::new(sample_rate).ok_or(anyhow::anyhow!("Invalid sample rate"))?,
        NonZeroU8::new(output_channels).ok_or(anyhow::anyhow!("Invalid channels"))?,
        VorbisBitrateManagementStrategy::QualityVbr { target_quality: 0.4 },
        None,
        output_file,
    )?;

    let chunk_size = 4096;
    let mut buffer = vec![0.0f32; chunk_size];
    
    // Continuous Timeline tracking
    let start_time = std::time::Instant::now();
    let mut total_frames_written: u64 = 0;
    
    // Tick every 10ms
    let tick_duration = std::time::Duration::from_millis(10);

    while !should_stop.load(Ordering::Relaxed) {
        // Calculate expected frames
        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let expected_frames = (elapsed_secs * sample_rate as f64) as u64;
        
        if total_frames_written < expected_frames {
            // Write UP TO 100ms to catch up
            let catch_up_limit = (sample_rate as f64 * 0.1) as usize; 
            let frames_needed = (expected_frames - total_frames_written).min(catch_up_limit as u64) as usize;
            
            let available = consumer.occupied_len();
            let mut frames_remaining = frames_needed;
            
            // 1. Consume available audio
            let audio_frames_available = available / input_channels as usize;
            
            if audio_frames_available > 0 {
                let max_frames_in_buffer = chunk_size / input_channels as usize;
                let frames_to_read = audio_frames_available.min(frames_remaining).min(max_frames_in_buffer);
                let samples_to_read = frames_to_read * input_channels as usize;
                
                let n = consumer.pop_slice(&mut buffer[..samples_to_read]);
                
                if n > 0 {
                    let raw_samples = &buffer[..n];
                    let normalized_samples = normalizer.normalize_loudness(raw_samples);
                    
                    // Logic to handle Mono/Stereo/Multi -> Stereo Output
                    let frames_encoded = normalized_samples.len() / input_channels as usize;

                    let planar_slices: Vec<Vec<f32>> = if input_channels == 1 {
                        // MONO -> STEREO
                        let ch_data = normalized_samples.clone();
                        vec![ch_data.clone(), ch_data]
                    } else if input_channels == 2 {
                        // STEREO -> STEREO (De-interleave)
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
                        // Multi-channel -> Stereo (Take first 2)
                         let mut left = Vec::with_capacity(frames_encoded);
                         let mut right = Vec::with_capacity(frames_encoded);
                         for chunk in normalized_samples.chunks(input_channels as usize) {
                              if chunk.len() >= 2 {
                                 left.push(chunk[0]);
                                 right.push(chunk[1]);
                             }
                         }
                         vec![left, right]
                    };

                    let slices_ref: Vec<&[f32]> = planar_slices.iter().map(|v| v.as_slice()).collect();
                    encoder.encode_audio_block(&slices_ref)?;
                    total_frames_written += frames_encoded as u64;
                    
                    if frames_remaining >= frames_encoded {
                        frames_remaining -= frames_encoded;
                    } else {
                        frames_remaining = 0;
                    }
                }
            }
            
            // 2. Silence Padding if still behind
            if frames_remaining > 0 {
                // Generate silence for remaining frames
                let silence_vec = vec![0.0f32; frames_remaining];
                let silence_planar = vec![silence_vec.clone(), silence_vec]; // Output is always Stereo
                
                let slices_ref: Vec<&[f32]> = silence_planar.iter().map(|v| v.as_slice()).collect();
                encoder.encode_audio_block(&slices_ref)?;
                total_frames_written += frames_remaining as u64;
            }

            // Loop immediately if we still need to catch up
            if total_frames_written < expected_frames {
                continue;
            }
        } else {
             thread::sleep(tick_duration);
        }
    }
    
    // Drain remaining samples in the buffer (only what is currently there)
    loop {
        let available = consumer.occupied_len();
        if available == 0 {
            break;
        }
        
        // Read aligned to input channels
        let max_frames_in_buffer = chunk_size / input_channels as usize;
        let audio_frames_available = available / input_channels as usize;
        
        let frames_to_read = audio_frames_available.min(max_frames_in_buffer);
        let samples_to_read = frames_to_read * input_channels as usize;

        if samples_to_read == 0 {
            break; 
        }
        
        let n = consumer.pop_slice(&mut buffer[..samples_to_read]);
        
        if n > 0 {
            let raw_samples = &buffer[..n];
            let normalized_samples = normalizer.normalize_loudness(raw_samples);
            let frames_encoded = normalized_samples.len() / input_channels as usize;
            
            let planar_slices: Vec<Vec<f32>> = if input_channels == 1 {
                let ch_data = normalized_samples.clone();
                vec![ch_data.clone(), ch_data]
            } else if input_channels == 2 {
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
                let mut left = Vec::with_capacity(frames_encoded);
                let mut right = Vec::with_capacity(frames_encoded);
                for chunk in normalized_samples.chunks(input_channels as usize) {
                    if chunk.len() >= 2 {
                        left.push(chunk[0]);
                        right.push(chunk[1]);
                    }
                }
                vec![left, right]
            };
            
            let slices_ref: Vec<&[f32]> = planar_slices.iter().map(|v| v.as_slice()).collect();
            if let Err(e) = encoder.encode_audio_block(&slices_ref) {
                eprintln!("Error encoding remaining mic audio: {}", e);
                break;
            }
            total_frames_written += frames_encoded as u64;
        }
    }

    // FINAL PADDING: Ensure duration matches exactly the final wall time
    let elapsed = start_time.elapsed().as_secs_f64();
    let expected_frames = (elapsed * sample_rate as f64) as u64;
    
    if total_frames_written < expected_frames {
        let missing_frames = expected_frames - total_frames_written;
        println!("Mic Audio Underrun: Padding with {} frames of silence ({:.2}s)", 
             missing_frames, missing_frames as f64 / sample_rate as f64);
             
         // Create silence block
         let silence_limit_chunk = 4096;
         let silence_vec = vec![0.0f32; silence_limit_chunk];
         let silence_planar = vec![silence_vec.clone(), silence_vec];
         let silence_refs: Vec<&[f32]> = silence_planar.iter().map(|v| v.as_slice()).collect(); 
         
         let mut remaining = missing_frames;
         while remaining > 0 {
             let chunk = remaining.min(silence_limit_chunk as u64);
             let partial_refs: Vec<&[f32]> = silence_refs.iter().map(|v| &v[..chunk as usize]).collect();
             if let Err(e) = encoder.encode_audio_block(&partial_refs) {
                 eprintln!("Error writing final mic silence: {}", e);
                 break;
             }
             remaining -= chunk;
         }
    }
    
    encoder.finish()?;
    Ok(())
}
