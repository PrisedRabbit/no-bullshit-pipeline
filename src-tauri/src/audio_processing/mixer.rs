use anyhow::Result;
use std::fs::File;
use std::path::Path;
use lewton::inside_ogg::OggStreamReader;
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoder};
use std::num::{NonZeroU32, NonZeroU8};

/// Mix two OGG Vorbis files (mic + system) into a single output OGG file
/// Files must have matching sample rates and channels
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
    let system_channels = system_reader.ident_hdr.audio_channels;
    let mic_rate = mic_reader.ident_hdr.audio_sample_rate;
    let system_rate = system_reader.ident_hdr.audio_sample_rate;
    
    println!("Mixer: mic={} ch @ {} Hz, system={} ch @ {} Hz", mic_channels, mic_rate, system_channels, system_rate);
    
    // Verify matching specs
    if mic_channels != system_channels || mic_rate != system_rate {
        return Err(anyhow::anyhow!(
            "Audio files must have matching specs for mixing. mic={} ch @ {} Hz, system={} ch @ {} Hz",
            mic_channels, mic_rate, system_channels, system_rate
        ));
    }
    
    println!("Mixer: Decoding streams to memory...");
    
    // Helper to decode entire stream
    fn decode_stream<T: std::io::Read + std::io::Seek>(reader: &mut OggStreamReader<T>, channels: u8) -> Result<Vec<Vec<f32>>> {
        let mut streams = vec![Vec::new(); channels as usize];
        
        while let Some(packet) = reader.read_dec_packet_generic::<Vec<Vec<f32>>>()? {
            for (ch, samples) in packet.iter().enumerate() {
                if ch < streams.len() {
                    streams[ch].extend_from_slice(samples);
                }
            }
        }
        Ok(streams)
    }
    
    let mic_pcm = decode_stream(&mut mic_reader, mic_channels)?;
    let system_pcm = decode_stream(&mut system_reader, system_channels)?;
    
    // Create output encoder
    let output_file = File::create(output_path.as_ref())?;
    let mut encoder = VorbisEncoder::new(
        0,
        [("", ""); 0],
        NonZeroU32::new(mic_rate).ok_or(anyhow::anyhow!("Invalid sample rate"))?,
        NonZeroU8::new(mic_channels).ok_or(anyhow::anyhow!("Invalid channels"))?,
        VorbisBitrateManagementStrategy::QualityVbr { target_quality: 0.5 },
        None,
        output_file,
    )?;
    
    // Mix entire PCM buffers
    let mixed_pcm = mix_samples(&mic_pcm, &system_pcm);
    let total_samples = if !mixed_pcm.is_empty() { mixed_pcm[0].len() } else { 0 };
    
    // Encode in chunks to keep memory usage reasonable during encoding (though we already loaded all to memory)
    // chunk size 4096 is standard
    let chunk_size = 4096;
    let mut offset = 0;
    
    while offset < total_samples {
        let end = (offset + chunk_size).min(total_samples);
        let mut slice_block = Vec::with_capacity(mic_channels as usize);
        
        for ch_data in &mixed_pcm {
            slice_block.push(&ch_data[offset..end]);
        }
        
        encoder.encode_audio_block(&slice_block)?;
        offset = end;
    }
    
    encoder.finish()?;
    
    println!("Mixer: Output file created successfully");
    Ok(())
}

/// Mix two sample sets (simple addition with soft clipping)
fn mix_samples(a: &[Vec<f32>], b: &[Vec<f32>]) -> Vec<Vec<f32>> {
    if a.is_empty() || b.is_empty() {
        return if !a.is_empty() { a.to_vec() } else { b.to_vec() };
    }
    
    // Check if channels are empty to avoid panic
    if a[0].is_empty() { return b.to_vec(); }
    if b[0].is_empty() { return a.to_vec(); }

    let channels = a.len().min(b.len());
    let mut mixed = Vec::with_capacity(channels);

    for ch in 0..channels {
        // Use MAX length to preserve all audio
        let len = a[ch].len().max(b[ch].len());
        let mut channel_mix = Vec::with_capacity(len);

        for i in 0..len {
            // Get sample or 0.0 if out of bounds
            let val_a = if i < a[ch].len() { a[ch][i] } else { 0.0 };
            let val_b = if i < b[ch].len() { b[ch][i] } else { 0.0 };
            
            // Simple addition with soft clipping (tanh)
            let sum = val_a + val_b;
            let clipped = if sum.abs() > 1.0 {
                sum.signum() * sum.abs().tanh()
            } else {
                sum
            };
            channel_mix.push(clipped);
        }
        mixed.push(channel_mix);
    }

    mixed
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
