use anyhow::Result;
use cidre::{cat, cf, core_audio as ca, os};
use hound::WavWriter;
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapProd, HeapRb,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;

pub struct SystemAudioRecorder {
    should_stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

struct AudioContext {
    producer: HeapProd<f32>,
}

impl SystemAudioRecorder {
    pub fn new(output_path: std::path::PathBuf) -> Result<Self> {
        let should_stop = Arc::new(AtomicBool::new(false));
        let should_stop_clone = should_stop.clone();

        let handle = thread::spawn(move || {
            if let Err(e) = run_audio_capture(output_path, should_stop_clone) {
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

pub fn start_system_capture(output_path: std::path::PathBuf) -> Result<SystemAudioRecorder> {
    SystemAudioRecorder::new(output_path)
}

fn run_audio_capture(path: std::path::PathBuf, should_stop: Arc<AtomicBool>) -> Result<()> {
    // 1. Get Default Output Device
    let output_device = ca::System::default_output_device()?;
    let output_uid = output_device.uid()?;

    // 2. Create Process Tap (Mono Global Tap)
    // Note: cidre::ns::Array::new() creates an empty array to exclude NO processes (capture all)
    let tap_desc = ca::TapDesc::with_mono_global_tap_excluding_processes(&cidre::ns::Array::new());
    let tap = tap_desc.create_process_tap()?;

    // 3. Create Aggregate Device Descriptor
    // This virtual device combines the Output Device and the Tap
    let sub_tap = cf::DictionaryOf::with_keys_values(
        &[ca::sub_device_keys::uid()],
        &[tap.uid().unwrap().as_type_ref()],
    );

    let agg_desc = cf::DictionaryOf::with_keys_values(
        &[
            ca::aggregate_device_keys::is_private(),
            // ca::aggregate_device_keys::is_stacked(), // Default false
            ca::aggregate_device_keys::tap_auto_start(),
            ca::aggregate_device_keys::name(),
            ca::aggregate_device_keys::main_sub_device(),
            ca::aggregate_device_keys::uid(),
            ca::aggregate_device_keys::tap_list(),
        ],
        &[
            cf::Boolean::value_true().as_type_ref(),
            // cf::Boolean::value_false(),
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
    let ring_buffer_size = 48000 * 4; // ~4 seconds buffer
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
        
        // Extract samples
        // We use av::AudioPcmBuf to wrap the buffer list safely if possible,
        // or just access raw data manually if needed.
        // Assuming Float32 format from tap.
        
        let buffers = &input_data.buffers;
        if !buffers.is_empty() {
             let buffer = &buffers[0];
             let ptr = buffer.data as *const f32;
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

    // 9. Writer Setup
    // Tap usually gives what is played. Default is often 44100 or 48000.
    // We can query the tap's ASBD.
    let asbd = tap.asbd()?;
    let sample_rate = asbd.sample_rate as u32;
    // Tap is Mono Global -> 1 channel? 
    // Wait, the creating function `with_mono_global_tap...` implies mono.
    // But `asbd.channels_per_frame` will tell.
    let channels = asbd.channels_per_frame as u16;

    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = WavWriter::create(&path, spec)?;

    // 10. Loop
    while !should_stop.load(Ordering::Relaxed) {
        // Pop from ring buffer and write to file
        let mut buffer = [0.0f32; 1024];
        let n = consumer.pop_slice(&mut buffer);
        if n > 0 {
            for &sample in &buffer[..n] {
                writer.write_sample(sample)?;
            }
        } else {
            thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    // Stop device
    // started_device dropped acts as stop? 
    // "StartedDevice" RAII usually stops on drop?
    // Checking cidre docs/source... StartedDevice has Drop impl that calls AudioDeviceStop.
    
    writer.finalize()?;
    Ok(())
}
