//! Compatibility shim over rubato's rewritten (3.0+) API.
//!
//! rubato 3.0 replaced the old `SincFixedIn` / `FftFixedInOut` resamplers
//! (which took `&[AsRef<[f32]>]` and returned `Vec<Vec<f32>>`) with a buffer-
//! adapter model (`process_into_buffer` over `audioadapter` buffers). Rather
//! than rewrite all five call sites against the new API, this module wraps the
//! new resamplers and re-exposes the exact surface the existing code uses:
//!
//!   - `SincFixedIn::<f32>::new(ratio, max_rel, &params, chunk, channels)`
//!   - `FftFixedInOut::<f32>::new(src, dst, chunk, channels)`
//!   - `.input_frames_next() -> usize`
//!   - `.process(&[impl AsRef<[f32]>], None) -> Result<Vec<Vec<f32>>, String>`
//!   - `.process_partial(Some(&[..]), None) -> Result<Vec<Vec<f32>>, String>`
//!
//! All call sites operate on mono audio (channels == 1), but the wrappers keep
//! the channel count general.

use audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::{Async, Fft, FixedAsync, FixedSync, Indexing};

// Re-export the sinc parameter types unchanged so existing `use` lines keep
// compiling against this module instead of `rubato`.
pub use rubato::{SincInterpolationParameters, SincInterpolationType, WindowFunction};

/// Marker re-export: the old code imports `Resampler` for trait-method access.
/// Our wrappers expose the same methods inherently, but keeping the name
/// importable avoids touching every `use` list.
pub use rubato::Resampler;

/// Drive a rubato 3.0 resampler over one fixed-size input chunk and collect the
/// produced frames as planar `Vec<Vec<f32>>`, mirroring the pre-3.0 `process`.
fn run_chunk<R: rubato::Resampler<f32>>(
    resampler: &mut R,
    channels: usize,
    input: &[Vec<f32>],
    in_frames: usize,
    partial: bool,
) -> Result<Vec<Vec<f32>>, String> {
    let input_adapter = SequentialSliceOfVecs::new(input, channels, input[0].len())
        .map_err(|e| format!("resample input adapter: {e}"))?;

    let out_max = resampler.output_frames_max();
    let mut out_data: Vec<Vec<f32>> = vec![vec![0.0f32; out_max]; channels];
    let mut out_adapter = SequentialSliceOfVecs::new_mut(&mut out_data, channels, out_max)
        .map_err(|e| format!("resample output adapter: {e}"))?;

    let indexing = Indexing {
        input_offset: 0,
        output_offset: 0,
        active_channels_mask: None,
        partial_len: if partial { Some(in_frames) } else { None },
    };

    let (_in, out_written) = resampler
        .process_into_buffer(&input_adapter, &mut out_adapter, Some(&indexing))
        .map_err(|e| format!("resample: {e}"))?;

    for ch in out_data.iter_mut() {
        ch.truncate(out_written);
    }
    Ok(out_data)
}

/// Build an owned planar buffer from the caller's `&[impl AsRef<[f32]>]`,
/// padding each channel up to `frames` with silence (needed for partial tails).
fn to_planar<T: AsRef<[f32]>>(input: &[T], channels: usize, frames: usize) -> Vec<Vec<f32>> {
    let mut planar: Vec<Vec<f32>> = Vec::with_capacity(channels);
    for ch in 0..channels {
        let src = input.get(ch).map(|c| c.as_ref()).unwrap_or(&[]);
        let mut v = vec![0.0f32; frames];
        let n = src.len().min(frames);
        v[..n].copy_from_slice(&src[..n]);
        planar.push(v);
    }
    planar
}

/// FFT-based fixed-ratio resampler (replacement for `rubato::FftFixedInOut`).
pub struct FftFixedInOut<T = f32> {
    inner: Fft<f32>,
    channels: usize,
    _marker: std::marker::PhantomData<T>,
}

impl FftFixedInOut<f32> {
    pub fn new(
        src_rate: usize,
        dst_rate: usize,
        chunk_size: usize,
        channels: usize,
    ) -> Result<Self, String> {
        // rubato 5.0's `Fft::new` derives the sub-chunk count internally from
        // `chunk_size`; the explicit `sub_chunks` argument is gone.
        let inner = Fft::<f32>::new(src_rate, dst_rate, chunk_size, channels, FixedSync::Input)
            .map_err(|e| format!("FFT resampler init: {e}"))?;
        Ok(Self {
            inner,
            channels,
            _marker: std::marker::PhantomData,
        })
    }

    pub fn input_frames_next(&self) -> usize {
        self.inner.input_frames_next()
    }

    pub fn process<T: AsRef<[f32]>>(
        &mut self,
        input: &[T],
        _active: Option<&[bool]>,
    ) -> Result<Vec<Vec<f32>>, String> {
        let frames = self.inner.input_frames_next();
        let planar = to_planar(input, self.channels, frames);
        run_chunk(&mut self.inner, self.channels, &planar, frames, false)
    }

    pub fn process_partial<T: AsRef<[f32]>>(
        &mut self,
        input: Option<&[T]>,
        _active: Option<&[bool]>,
    ) -> Result<Vec<Vec<f32>>, String> {
        let frames = self.inner.input_frames_next();
        let actual = input
            .and_then(|i| i.first())
            .map(|c| c.as_ref().len())
            .unwrap_or(0);
        let planar = match input {
            Some(i) => to_planar(i, self.channels, frames),
            None => vec![vec![0.0f32; frames]; self.channels],
        };
        run_chunk(&mut self.inner, self.channels, &planar, actual, true)
    }
}

/// Sinc fixed-input resampler (replacement for `rubato::SincFixedIn`).
pub struct SincFixedIn<T = f32> {
    inner: Async<f32>,
    channels: usize,
    _marker: std::marker::PhantomData<T>,
}

impl SincFixedIn<f32> {
    pub fn new(
        resample_ratio: f64,
        max_resample_ratio_relative: f64,
        params: SincInterpolationParameters,
        chunk_size: usize,
        channels: usize,
    ) -> Result<Self, String> {
        let inner = Async::<f32>::new_sinc(
            resample_ratio,
            max_resample_ratio_relative,
            &params,
            chunk_size,
            channels,
            FixedAsync::Input,
        )
        .map_err(|e| format!("sinc resampler init: {e}"))?;
        Ok(Self {
            inner,
            channels,
            _marker: std::marker::PhantomData,
        })
    }

    // Kept for parity with FftFixedInOut; the sinc call sites drive `process`
    // directly, so this isn't currently called.
    #[allow(dead_code)]
    pub fn input_frames_next(&self) -> usize {
        self.inner.input_frames_next()
    }

    pub fn process<T: AsRef<[f32]>>(
        &mut self,
        input: &[T],
        _active: Option<&[bool]>,
    ) -> Result<Vec<Vec<f32>>, String> {
        let frames = self.inner.input_frames_next();
        let planar = to_planar(input, self.channels, frames);
        run_chunk(&mut self.inner, self.channels, &planar, frames, false)
    }

    pub fn process_partial<T: AsRef<[f32]>>(
        &mut self,
        input: Option<&[T]>,
        _active: Option<&[bool]>,
    ) -> Result<Vec<Vec<f32>>, String> {
        let frames = self.inner.input_frames_next();
        let actual = input
            .and_then(|i| i.first())
            .map(|c| c.as_ref().len())
            .unwrap_or(0);
        let planar = match input {
            Some(i) => to_planar(i, self.channels, frames),
            None => vec![vec![0.0f32; frames]; self.channels],
        };
        run_chunk(&mut self.inner, self.channels, &planar, actual, true)
    }
}
