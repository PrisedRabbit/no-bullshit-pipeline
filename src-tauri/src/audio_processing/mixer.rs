use anyhow::Result;
use std::fs::File;
use std::path::Path;
use lewton::inside_ogg::OggStreamReader;
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoder};
use std::num::{NonZeroU32, NonZeroU8};

/// Mix two OGG Vorbis files (mic + system) into a single output OGG file
/// Files must have matching sample rates and channels
use std::collections::VecDeque;

/// Mix two OGG Vorbis files (mic + system) into a single output OGG file
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
    let channels = mic_reader.ident_hdr.audio_channels;
    let sample_rate = mic_reader.ident_hdr.audio_sample_rate;
    
    // Validate minimal compatibility (we assume 2 channels 48000 usually)
    // If system differs, we error for now (or could implement resampling later)
    if system_reader.ident_hdr.audio_channels != channels || 
       system_reader.ident_hdr.audio_sample_rate != sample_rate {
        return Err(anyhow::anyhow!("Sample rate/channel mismatch. Mic: {}/{}, Sys: {}/{}", 
            channels, sample_rate, 
            system_reader.ident_hdr.audio_channels, system_reader.ident_hdr.audio_sample_rate));
    }
    
    let output_file = File::create(output_path.as_ref())?;
    let mut encoder = VorbisEncoder::new(
        0,
        [("", ""); 0],
        NonZeroU32::new(sample_rate).ok_or(anyhow::anyhow!("Invalid sample rate"))?,
        NonZeroU8::new(channels).ok_or(anyhow::anyhow!("Invalid channels"))?,
        VorbisBitrateManagementStrategy::QualityVbr { target_quality: 0.5 },
        None,
        output_file,
    )?;

    // Buffers for each channel (Assuming Stereo max, but using Vec<VecDeque<f32>> for flexibility)
    let mut mic_buffers: Vec<VecDeque<f32>> = vec![VecDeque::new(); channels as usize];
    let mut sys_buffers: Vec<VecDeque<f32>> = vec![VecDeque::new(); channels as usize];
    
    let mut mic_done = false;
    let mut sys_done = false;
    
    let block_size = 4096; // Process 4096 samples at a time
    
    loop {
        // 1. REFILL MIC BUFFERS
        while !mic_done && mic_buffers[0].len() < block_size {
            match mic_reader.read_dec_packet_generic::<Vec<Vec<f32>>>() {
                Ok(Some(packet)) => {
                    if packet.is_empty() { continue; }
                    for (ch_idx, samples) in packet.iter().enumerate() {
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
                    for (ch_idx, samples) in packet.iter().enumerate() {
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
        // We go until both are fully completely empty
        let mic_avail = mic_buffers[0].len();
        let sys_avail = sys_buffers[0].len();
        
        if mic_avail == 0 && sys_avail == 0 && mic_done && sys_done {
            break; // All done
        }
        
        // Process up to block_size, but limited by available data if streams are done
        // If streams are NOT done, we want to wait for full block_size, 
        // but if one is done we just drain the other.
        
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
                let clipped = if sum.abs() > 1.0 {
                    sum.signum() * sum.abs().tanh()
                } else {
                    sum
                };
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
