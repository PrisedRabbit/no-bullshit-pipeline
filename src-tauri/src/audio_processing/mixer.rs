use anyhow::Result;
use std::fs::File;
use std::path::Path;
use lewton::inside_ogg::OggStreamReader;
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};
use std::num::{NonZeroU32, NonZeroU8};
use std::collections::VecDeque;
// Note: rubato available for high-quality resampling if needed


/// Mix two OGG Vorbis files (mic + system) into a single output OGG file.
/// Automatically resamples if sample rates don't match.
/// Uses buffered streaming to keep memory usage low even for large files.
pub fn mix_audio_files<P: AsRef<Path>>(
    mic_path: P,
    system_path: P,
    output_path: P,
) -> Result<()> {
    // Open both OGG files
    let mic_file = File::open(mic_path.as_ref())?;
    let system_file = File::open(system_path.as_ref())?;
    
    let mut mic_reader = OggStreamReader::new(mic_file)?;
    let mut system_reader = OggStreamReader::new(system_file)?;
    
    // Get specs
    let mic_channels = mic_reader.ident_hdr.audio_channels;
    let mic_sample_rate = mic_reader.ident_hdr.audio_sample_rate;
    let sys_channels = system_reader.ident_hdr.audio_channels;
    let sys_sample_rate = system_reader.ident_hdr.audio_sample_rate;
    
    // Validate channels match (we don't handle channel conversion yet)
    if mic_channels != sys_channels {
        return Err(anyhow::anyhow!("Channel count mismatch. Mic: {}, Sys: {}", 
            mic_channels, sys_channels));
    }
    
    let channels = mic_channels;
    
    // Always output at 48kHz (standard sample rate)
    let output_sample_rate = 48000;
    
    // Check if resampling needed
    let mic_needs_resample = mic_sample_rate != output_sample_rate;
    let sys_needs_resample = sys_sample_rate != output_sample_rate;
    
    #[cfg(debug_assertions)]
    if mic_needs_resample {
        eprintln!("Will resample mic on-the-fly: {} Hz -> {} Hz", mic_sample_rate, output_sample_rate);
    }
    #[cfg(debug_assertions)]
    if sys_needs_resample {
        eprintln!("Will resample system on-the-fly: {} Hz -> {} Hz", sys_sample_rate, output_sample_rate);
    }
    
    let output_file = File::create(output_path.as_ref())?;
    let mut encoder = VorbisEncoderBuilder::new_with_serial(
        NonZeroU32::new(output_sample_rate).ok_or(anyhow::anyhow!("Invalid sample rate"))?,
        NonZeroU8::new(channels).ok_or(anyhow::anyhow!("Invalid channels"))?,
        output_file,
        0,
    )
    .bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr { target_quality: 0.5 })
    .build()?;

    // Buffers for mixing
    let mut mic_buffers: Vec<VecDeque<f32>> = vec![VecDeque::new(); channels as usize];
    let mut sys_buffers: Vec<VecDeque<f32>> = vec![VecDeque::new(); channels as usize];
    
    let mut mic_done = false;
    let mut sys_done = false;
    
    let block_size = 4096;
    
    #[cfg(debug_assertions)]
    eprintln!("Mixing audio...");
    
    loop {
        // 1. REFILL MIC BUFFERS
        while !mic_done && mic_buffers[0].len() < block_size {
            match mic_reader.read_dec_packet_generic::<Vec<Vec<f32>>>() {
                Ok(Some(packet)) => {
                    if packet.is_empty() { continue; }
                    
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
                Ok(None) => mic_done = true,
                Err(_) => mic_done = true,
            }
        }
        
        // 2. REFILL SYSTEM BUFFERS
        while !sys_done && sys_buffers[0].len() < block_size {
            match system_reader.read_dec_packet_generic::<Vec<Vec<f32>>>() {
                Ok(Some(packet)) => {
                    if packet.is_empty() { continue; }
                    
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
                Ok(None) => sys_done = true,
                Err(_) => sys_done = true,
            }
        }
        
        // 3. DETERMINE PROCESSING AMOUNT
        let mic_avail = mic_buffers[0].len();
        let sys_avail = sys_buffers[0].len();
        
        if mic_avail == 0 && sys_avail == 0 && mic_done && sys_done {
            break; // All done
        }
        
        // Limit based on what is available
        let max_avail = mic_avail.max(sys_avail);
        let process_count = block_size.min(max_avail);

        if process_count == 0 { break; }

        // 4. MIX
        let mut mixed_channels: Vec<Vec<f32>> = vec![Vec::with_capacity(process_count); channels as usize];
        
        for ch in 0..channels as usize {
            for _ in 0..process_count {
                let m = mic_buffers[ch].pop_front().unwrap_or(0.0);
                let s = sys_buffers[ch].pop_front().unwrap_or(0.0);
                
                let sum = m + s;
                // Soft clip using tanh - smooth, continuous, no discontinuity
                let clipped = (sum as f64).tanh() as f32;
                mixed_channels[ch].push(clipped);
            }
        }
        
        // 5. ENCODE
        let slices: Vec<&[f32]> = mixed_channels.iter().map(|v| v.as_slice()).collect();
        encoder.encode_audio_block(&slices)?;
    }
    
    encoder.finish()?;
    Ok(())
}

/// Get actual duration of an OGG file by counting samples
pub fn get_ogg_duration<P: AsRef<Path>>(path: P) -> Result<f64> {
    let file = File::open(path.as_ref())?;
    let mut reader = OggStreamReader::new(file)?;
    
    let sample_rate = reader.ident_hdr.audio_sample_rate;
    let mut total_samples = 0;
    
    loop {
        match reader.read_dec_packet_generic::<Vec<Vec<f32>>>() {
            Ok(Some(samples)) => {
                if !samples.is_empty() && !samples[0].is_empty() {
                    total_samples += samples[0].len();
                }
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }
    
    Ok(total_samples as f64 / sample_rate as f64)
}

/// Simple linear interpolation resampler - fast and good enough for voice
/// Converts input at input_rate to output_rate
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
            // Map output position to input position
            let src_pos = i as f64 / ratio;
            let src_idx = src_pos.floor() as usize;
            let frac = src_pos - src_idx as f64;
            
            let sample = if src_idx + 1 < input_len {
                // Linear interpolation between two samples
                let a = input[ch][src_idx];
                let b = input[ch][src_idx + 1];
                a + (b - a) * frac as f32
            } else if src_idx < input_len {
                // Last sample, no interpolation
                input[ch][src_idx]
            } else {
                0.0
            };
            
            output[ch].push(sample);
        }
    }
    
    output
}
