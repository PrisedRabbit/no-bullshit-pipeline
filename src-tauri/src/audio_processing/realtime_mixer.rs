use anyhow::Result;
use lewton::inside_ogg::OggStreamReader;
use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::num::{NonZeroU32, NonZeroU8};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoder};

/// Real-time mixer that reads from growing raw files during recording
pub struct RealtimeMixer {
    should_stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl RealtimeMixer {
    pub fn new(
        mic_path: PathBuf,
        sys_path: PathBuf,
        output_path: PathBuf,
    ) -> Result<Self> {
        let should_stop = Arc::new(AtomicBool::new(false));
        let should_stop_clone = should_stop.clone();
        
        let handle = thread::spawn(move || {
            if let Err(e) = run_realtime_mixer(
                mic_path,
                sys_path,
                output_path,
                should_stop_clone,
            ) {
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

fn run_realtime_mixer(
    mic_path: PathBuf,
    sys_path: PathBuf,
    output_path: PathBuf,
    should_stop: Arc<AtomicBool>,
) -> Result<()> {
    // Wait a bit for files to be created and have some data
    thread::sleep(Duration::from_millis(500));
    
    println!("Real-time mixer: Starting");
    
    // Open files
    let mic_file = File::open(&mic_path)?;
    let sys_file = File::open(&sys_path)?;
    
    let mut mic_reader = OggStreamReader::new(BufReader::new(mic_file))?;
    let mut sys_reader = OggStreamReader::new(BufReader::new(sys_file))?;
    
    let mic_sample_rate = mic_reader.ident_hdr.audio_sample_rate;
    let sys_sample_rate = sys_reader.ident_hdr.audio_sample_rate;
    let channels = mic_reader.ident_hdr.audio_channels;
    
    // Always output at 48kHz (standard sample rate)
    let output_sample_rate = 48000;
    
    println!("Real-time mixer: Mic {}Hz, Sys {}Hz -> Output {}Hz", 
        mic_sample_rate, sys_sample_rate, output_sample_rate);
    
    let output_file = File::create(&output_path)?;
    let mut encoder = VorbisEncoder::new(
        0,
        [("", ""); 0],
        NonZeroU32::new(output_sample_rate).ok_or(anyhow::anyhow!("Invalid sample rate"))?,
        NonZeroU8::new(channels).ok_or(anyhow::anyhow!("Invalid channels"))?,
        VorbisBitrateManagementStrategy::QualityVbr { target_quality: 0.5 },
        None,
       output_file,
    )?;
    
    let mic_needs_resample = mic_sample_rate != output_sample_rate;
    let sys_needs_resample = sys_sample_rate != output_sample_rate;
    
    let mut mic_buffers: Vec<VecDeque<f32>> = vec![VecDeque::new(); channels as usize];
    let mut sys_buffers: Vec<VecDeque<f32>> = vec![VecDeque::new(); channels as usize];
    
    let block_size = 4096;
    let mut encoded_blocks = 0;
    
    loop {
        if should_stop.load(Ordering::Relaxed) {
            break;
        }
        
        // Read mic packets
        while mic_buffers[0].len() < block_size {
            match mic_reader.read_dec_packet_generic::<Vec<Vec<f32>>>() {
                Ok(Some(packet)) => {
                    if packet.is_empty() { break; }
                    
                    let processed = if mic_needs_resample {
                        resample_linear(&packet, mic_sample_rate as f64, output_sample_rate as f64)
                    } else {
                        packet
                    };
                    
                    for (ch_idx, samples) in processed.iter().enumerate() {
                        if ch_idx < mic_buffers.len() {
                            mic_buffers[ch_idx].extend(samples.iter().cloned());
                        }
                    }
                },
                Ok(None) | Err(_) => break,
            }
        }
        
        // Read system packets
        while sys_buffers[0].len() < block_size {
            match sys_reader.read_dec_packet_generic::<Vec<Vec<f32>>>() {
                Ok(Some(packet)) => {
                    if packet.is_empty() { break; }
                    
                    let processed = if sys_needs_resample {
                        resample_linear(&packet, sys_sample_rate as f64, output_sample_rate as f64)
                    } else {
                        packet
                    };
                    
                    for (ch_idx, samples) in processed.iter().enumerate() {
                        if ch_idx < sys_buffers.len() {
                            sys_buffers[ch_idx].extend(samples.iter().cloned());
                        }
                    }
                },
                Ok(None) | Err(_) => break,
            }
        }
        
        // Mix available samples
        let mic_avail = mic_buffers[0].len();
        let sys_avail = sys_buffers[0].len();
        
        if mic_avail == 0 && sys_avail == 0 {
            // No data yet, wait a bit
            thread::sleep(Duration::from_millis(100));
            continue;
        }
        
        let max_avail = mic_avail.max(sys_avail);
        let process_count = block_size.min(max_avail);
        
        if process_count == 0 {
            thread::sleep(Duration::from_millis(50));
            continue;
        }
        
        // Mix
        let mut mixed_channels: Vec<Vec<f32>> = vec![Vec::with_capacity(process_count); channels as usize];
        
        for ch in 0..channels as usize {
            for _ in 0..process_count {
                let m = mic_buffers[ch].pop_front().unwrap_or(0.0);
                let s = sys_buffers[ch].pop_front().unwrap_or(0.0);
                
                let sum = m + s;
                let clipped = if sum.abs() > 1.0 {
                    sum.signum() * sum.abs().tanh()
                } else {
                    sum
                };
                mixed_channels[ch].push(clipped);
            }
        }
        
        // Encode
        let slices: Vec<&[f32]> = mixed_channels.iter().map(|v| v.as_slice()).collect();
        encoder.encode_audio_block(&slices)?;
        encoded_blocks += 1;
    }
    
    encoder.finish()?;
    println!("Real-time mixer: Finished. Encoded {} blocks", encoded_blocks);
    Ok(())
}

/// Simple linear interpolation resampler
fn resample_linear(input: &[Vec<f32>], input_rate: f64, output_rate: f64) -> Vec<Vec<f32>> {
    if input.is_empty() || input[0].is_empty() {
        return input.to_vec();
    }
    
    let ratio = output_rate / input_rate;
    let input_len = input[0].len();
    let output_len = (input_len as f64 * ratio).ceil() as usize;
    let channels = input.len();
    
    let mut output = vec![Vec::with_capacity(output_len); channels];
    
    for ch in 0..channels {
        for i in 0..output_len {
            let src_pos = i as f64 / ratio;
            let src_idx = src_pos.floor() as usize;
            let frac = src_pos - src_idx as f64;
            
            let sample = if src_idx + 1 < input_len {
                let a = input[ch][src_idx];
                let b = input[ch][src_idx + 1];
                a + (b - a) * frac as f32
            } else if src_idx < input_len {
                input[ch][src_idx]
            } else {
                0.0
            };
            
            output[ch].push(sample);
        }
    }
    
    output
}
