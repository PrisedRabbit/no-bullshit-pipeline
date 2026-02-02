# Audio Module

## Capabilities

- Microphone capture
- System audio capture (macOS only via Process Taps)
- Audio mixing (real-time)
- Audio playback
- Waveform generation

## Mic Capture (cpal)

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn capture_mic() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or("No input device")?;

    let config = device.default_input_config()?;

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Process audio samples
        },
        |err| eprintln!("Stream error: {}", err),
        None,
    )?;

    stream.play()?;
    Ok(())
}
```

## System Audio (macOS Process Taps)

```rust
// Requires cidre crate and macOS 13+
// Uses Core Audio Process Taps API
// Requires Screen Recording permission

use cidre::at::audio;

// Process tap setup - captures system audio output
// See cidre documentation for full implementation
```

## Audio Mixing

```rust
// Mix two audio streams (mic + system)
fn mix_samples(mic: &[f32], system: &[f32], output: &mut [f32]) {
    for i in 0..output.len() {
        let m = mic.get(i).copied().unwrap_or(0.0);
        let s = system.get(i).copied().unwrap_or(0.0);
        output[i] = (m + s).clamp(-1.0, 1.0);
    }
}
```

## Audio Normalization (EBU R128)

```rust
// Target: -23 LUFS integrated loudness
// True peak limit: -1 dBTP

fn normalize_audio(samples: &mut [f32], target_lufs: f32) {
    let current_lufs = measure_loudness(samples);
    let gain_db = target_lufs - current_lufs;
    let gain_linear = 10.0_f32.powf(gain_db / 20.0);

    for sample in samples.iter_mut() {
        *sample *= gain_linear;
    }
}
```

## Audio Playback (rodio)

```rust
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;

fn play_audio(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (_stream, handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&handle)?;

    let file = File::open(path)?;
    let source = Decoder::new(BufReader::new(file))?;

    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}
```

## Waveform Generation

```rust
fn generate_waveform(samples: &[f32], num_points: usize) -> Vec<f32> {
    let chunk_size = samples.len() / num_points;
    let mut waveform = Vec::with_capacity(num_points);

    for chunk in samples.chunks(chunk_size) {
        // Get peak amplitude for this chunk
        let peak = chunk.iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);
        waveform.push(peak);
    }

    waveform
}
```

## File Formats

| Format | Use Case | Crate |
|--------|----------|-------|
| WAV | Intermediate/lossless | hound |
| OGG Vorbis | Final output (compressed) | ogg, vorbis |
| MP3 | Export (optional) | mp3lame |
