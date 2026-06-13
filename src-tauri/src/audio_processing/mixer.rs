use anyhow::Result;
use lewton::inside_ogg::OggStreamReader;
use std::collections::VecDeque;
use std::fs::File;
use std::num::{NonZeroU8, NonZeroU32};
use std::path::Path;
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};
// Note: rubato available for high-quality resampling if needed

/// Mix two OGG Vorbis files (mic + system) into a single output OGG file.
/// Automatically resamples if sample rates don't match.
/// Uses buffered streaming to keep memory usage low even for large files.
pub fn mix_audio_files<P: AsRef<Path>>(mic_path: P, system_path: P, output_path: P) -> Result<()> {
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
        return Err(anyhow::anyhow!(
            "Channel count mismatch. Mic: {}, Sys: {}",
            mic_channels,
            sys_channels
        ));
    }

    let channels = mic_channels;

    // Always output at 48kHz (standard sample rate)
    let output_sample_rate = 48000;

    // Check if resampling needed
    let mic_needs_resample = mic_sample_rate != output_sample_rate;
    let sys_needs_resample = sys_sample_rate != output_sample_rate;

    #[cfg(debug_assertions)]
    if mic_needs_resample {
        eprintln!(
            "Will resample mic on-the-fly: {} Hz -> {} Hz",
            mic_sample_rate, output_sample_rate
        );
    }
    #[cfg(debug_assertions)]
    if sys_needs_resample {
        eprintln!(
            "Will resample system on-the-fly: {} Hz -> {} Hz",
            sys_sample_rate, output_sample_rate
        );
    }

    let output_file = File::create(output_path.as_ref())?;
    let mut encoder = VorbisEncoderBuilder::new_with_serial(
        NonZeroU32::new(output_sample_rate).ok_or(anyhow::anyhow!("Invalid sample rate"))?,
        NonZeroU8::new(channels).ok_or(anyhow::anyhow!("Invalid channels"))?,
        output_file,
        0,
    )
    .bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
        target_quality: 0.5,
    })
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
                    if packet.is_empty() {
                        continue;
                    }

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
                }
                Ok(None) => mic_done = true,
                Err(_) => mic_done = true,
            }
        }

        // 2. REFILL SYSTEM BUFFERS
        while !sys_done && sys_buffers[0].len() < block_size {
            match system_reader.read_dec_packet_generic::<Vec<Vec<f32>>>() {
                Ok(Some(packet)) => {
                    if packet.is_empty() {
                        continue;
                    }

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
                }
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

        if process_count == 0 {
            break;
        }

        // 4. MIX
        let mut mixed_channels: Vec<Vec<f32>> =
            vec![Vec::with_capacity(process_count); channels as usize];

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

/// Mix two OGG files with loudness normalization.
/// Normalizes both sources to equal RMS before mixing for balanced output.
/// This ensures quieter sources (like system audio) are properly audible.
pub fn mix_audio_files_normalized<P: AsRef<Path>>(
    mic_path: P,
    system_path: P,
    output_path: P,
) -> Result<()> {
    const TARGET_RMS_DB: f32 = -20.0; // Target RMS level in dB
    const OUTPUT_SAMPLE_RATE: u32 = 48000;

    // Helper: read entire OGG file into memory as stereo f32
    fn read_ogg_to_samples(path: &Path) -> Result<(Vec<Vec<f32>>, u32, u8)> {
        let file = File::open(path)?;
        let mut reader = OggStreamReader::new(file)?;
        let sample_rate = reader.ident_hdr.audio_sample_rate;
        let channels = reader.ident_hdr.audio_channels;

        let mut all_samples: Vec<Vec<f32>> = vec![Vec::new(); channels as usize];

        while let Some(packet) = reader.read_dec_packet_generic::<Vec<Vec<f32>>>()? {
            if packet.is_empty() {
                continue;
            }
            for (ch, samples) in packet.iter().enumerate() {
                if ch < all_samples.len() {
                    all_samples[ch].extend(samples);
                }
            }
        }

        Ok((all_samples, sample_rate, channels))
    }

    // Helper: calculate RMS of audio samples
    fn calculate_rms(samples: &[Vec<f32>]) -> f32 {
        if samples.is_empty() || samples[0].is_empty() {
            return 0.0;
        }

        let mut sum_sq = 0.0_f64;
        let mut count = 0_usize;

        for channel in samples {
            for &sample in channel {
                sum_sq += (sample as f64) * (sample as f64);
                count += 1;
            }
        }

        if count == 0 {
            return 0.0;
        }
        (sum_sq / count as f64).sqrt() as f32
    }

    // Helper: convert RMS to dB
    fn rms_to_db(rms: f32) -> f32 {
        if rms <= 0.0 {
            return -100.0;
        }
        20.0 * rms.log10()
    }

    // Helper: calculate gain to reach target RMS
    fn gain_for_target_rms(current_rms: f32, target_db: f32) -> f32 {
        if current_rms <= 0.0 {
            return 1.0;
        }
        let current_db = rms_to_db(current_rms);
        let gain_db = target_db - current_db;
        // Limit gain to reasonable range (-20 to +20 dB)
        let clamped_gain_db = gain_db.clamp(-20.0, 20.0);
        10_f32.powf(clamped_gain_db / 20.0)
    }

    // Read both files
    let (mic_samples, mic_rate, mic_channels) = read_ogg_to_samples(mic_path.as_ref())?;
    let (sys_samples, sys_rate, sys_channels) = read_ogg_to_samples(system_path.as_ref())?;

    if mic_channels != sys_channels {
        return Err(anyhow::anyhow!("Channel count mismatch"));
    }
    let channels = mic_channels;

    // Calculate RMS for each source
    let mic_rms = calculate_rms(&mic_samples);
    let sys_rms = calculate_rms(&sys_samples);

    #[cfg(debug_assertions)]
    eprintln!(
        "Normalizing mix: mic RMS={:.1} dB, system RMS={:.1} dB, target={:.1} dB",
        rms_to_db(mic_rms),
        rms_to_db(sys_rms),
        TARGET_RMS_DB
    );

    // Calculate gains
    let mic_gain = gain_for_target_rms(mic_rms, TARGET_RMS_DB);
    let sys_gain = gain_for_target_rms(sys_rms, TARGET_RMS_DB);

    #[cfg(debug_assertions)]
    eprintln!(
        "Applying gains: mic={:.2}x ({:+.1} dB), system={:.2}x ({:+.1} dB)",
        mic_gain,
        20.0 * mic_gain.log10(),
        sys_gain,
        20.0 * sys_gain.log10()
    );

    // Resample if needed
    let mic_resampled = if mic_rate != OUTPUT_SAMPLE_RATE {
        resample_linear(&mic_samples, mic_rate as f64, OUTPUT_SAMPLE_RATE as f64)
    } else {
        mic_samples
    };

    let sys_resampled = if sys_rate != OUTPUT_SAMPLE_RATE {
        resample_linear(&sys_samples, sys_rate as f64, OUTPUT_SAMPLE_RATE as f64)
    } else {
        sys_samples
    };

    // Determine output length (max of both)
    let mic_len = mic_resampled.first().map(|v| v.len()).unwrap_or(0);
    let sys_len = sys_resampled.first().map(|v| v.len()).unwrap_or(0);
    let output_len = mic_len.max(sys_len);

    // Create encoder
    let output_file = File::create(output_path.as_ref())?;
    let mut encoder = VorbisEncoderBuilder::new_with_serial(
        NonZeroU32::new(OUTPUT_SAMPLE_RATE).ok_or(anyhow::anyhow!("Invalid sample rate"))?,
        NonZeroU8::new(channels).ok_or(anyhow::anyhow!("Invalid channels"))?,
        output_file,
        0,
    )
    .bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
        target_quality: 0.5,
    })
    .build()?;

    // Mix with normalization in blocks
    let block_size = 4096;
    let mut pos = 0;

    while pos < output_len {
        let end = (pos + block_size).min(output_len);
        let mut mixed_channels: Vec<Vec<f32>> =
            vec![Vec::with_capacity(end - pos); channels as usize];

        for (ch, channel) in mixed_channels.iter_mut().enumerate() {
            for i in pos..end {
                let m = mic_resampled
                    .get(ch)
                    .and_then(|v| v.get(i))
                    .copied()
                    .unwrap_or(0.0)
                    * mic_gain;
                let s = sys_resampled
                    .get(ch)
                    .and_then(|v| v.get(i))
                    .copied()
                    .unwrap_or(0.0)
                    * sys_gain;

                // Mix and soft clip
                let sum = m + s;
                let clipped = (sum as f64).tanh() as f32;
                channel.push(clipped);
            }
        }

        let slices: Vec<&[f32]> = mixed_channels.iter().map(|v| v.as_slice()).collect();
        encoder.encode_audio_block(&slices)?;
        pos = end;
    }

    encoder.finish()?;

    #[cfg(debug_assertions)]
    eprintln!(
        "Normalized mix created: {} samples @ {} Hz",
        output_len, OUTPUT_SAMPLE_RATE
    );

    Ok(())
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
