use anyhow::Result;
use log::warn;

/// True peak limiter with lookahead buffer (prevents clipping)
struct TruePeakLimiter {
    lookahead_samples: usize,
    buffer: Vec<f32>,
    gain_reduction: Vec<f32>,
    current_position: usize,
}

impl TruePeakLimiter {
    fn new(sample_rate: u32) -> Self {
        const LIMITER_LOOKAHEAD_MS: usize = 10;
        let lookahead_samples = ((sample_rate as usize * LIMITER_LOOKAHEAD_MS) / 1000).max(1);

        Self {
            lookahead_samples,
            buffer: vec![0.0; lookahead_samples],
            gain_reduction: vec![1.0; lookahead_samples],
            current_position: 0,
        }
    }

    fn process(&mut self, sample: f32, true_peak_limit: f32) -> f32 {
        self.buffer[self.current_position] = sample;

        let sample_abs = sample.abs();
        if sample_abs > true_peak_limit {
            let reduction = true_peak_limit / sample_abs;
            self.gain_reduction[self.current_position] = reduction;
        } else {
            self.gain_reduction[self.current_position] = 1.0;
        }

        let output_position = (self.current_position + 1) % self.lookahead_samples;
        let output_sample = self.buffer[output_position] * self.gain_reduction[output_position];

        self.current_position = output_position;
        output_sample
    }
}

/// Professional loudness normalizer using EBU R128 standard
/// This is a STATEFUL normalizer that tracks cumulative loudness over time
pub struct LoudnessNormalizer {
    ebur128: ebur128::EbuR128,
    limiter: TruePeakLimiter,
    gain_linear: f32,
    loudness_buffer: Vec<f32>,
    true_peak_limit: f32,
}

impl LoudnessNormalizer {
    /// Create a new EBU R128 loudness normalizer
    pub fn new(channels: u32, sample_rate: u32) -> Result<Self> {
        const TRUE_PEAK_LIMIT: f64 = -1.0;
        const ANALYZE_CHUNK_SIZE: usize = 512;

        let ebur128 = ebur128::EbuR128::new(
            channels,
            sample_rate,
            ebur128::Mode::S | ebur128::Mode::TRUE_PEAK,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create EBU R128 normalizer: {}", e))?;

        let true_peak_limit = 10_f32.powf(TRUE_PEAK_LIMIT as f32 / 20.0);

        Ok(Self {
            ebur128,
            limiter: TruePeakLimiter::new(sample_rate),
            gain_linear: 1.0,
            loudness_buffer: Vec::with_capacity(ANALYZE_CHUNK_SIZE),
            true_peak_limit,
        })
    }

    /// Normalize loudness using EBU R128 standard with true peak limiting
    /// Target: -23 LUFS
    pub fn normalize_loudness(&mut self, samples: &[f32]) -> Vec<f32> {
        if samples.is_empty() {
            return Vec::new();
        }

        const TARGET_LUFS: f64 = -23.0;
        const ANALYZE_CHUNK_SIZE: usize = 512;

        let mut normalized_samples = Vec::with_capacity(samples.len());

        for &sample in samples {
            // Accumulate samples for loudness analysis
            self.loudness_buffer.push(sample);

            // Analyze loudness every 512 samples
            if self.loudness_buffer.len() >= ANALYZE_CHUNK_SIZE {
                if let Err(e) = self.ebur128.add_frames_f32(&self.loudness_buffer) {
                    warn!("Failed to add frames to EBU R128: {}", e);
                } else {
                    // Update gain based on cumulative loudness
                    if let Ok(current_lufs) = self.ebur128.loudness_shortterm()
                        && current_lufs.is_finite()
                        && current_lufs < 0.0
                    {
                        let gain_db = TARGET_LUFS - current_lufs;
                        self.gain_linear = 10_f32.powf(gain_db as f32 / 20.0);
                    }
                }
                self.loudness_buffer.clear();
            }

            // Apply gain and true peak limiting
            let amplified = sample * self.gain_linear;
            let limited = self.limiter.process(amplified, self.true_peak_limit);

            normalized_samples.push(limited);
        }

        normalized_samples
    }
}
