use std::ffi::c_void;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tauri_plugin_notification::NotificationExt;

use crate::config;
use crate::pipelines;

/// Global flag: a call notification was sent, awaiting user click
static PENDING_CALL: AtomicBool = AtomicBool::new(false);

/// Check and clear the pending call flag. Returns true if a call was pending.
pub fn take_pending_call() -> bool {
    PENDING_CALL.swap(false, Ordering::SeqCst)
}

/// Known call app process names
const CALL_APP_PROCESSES: &[(&str, &str)] = &[
    ("zoom.us", "Zoom"),
    ("CptHost", "Zoom"),
    ("Microsoft Teams", "Teams"),
    ("FaceTime", "FaceTime"),
    ("avconferenced", "FaceTime/Phone"),
    ("callservicesd", "Phone Call"),
    ("Slack", "Slack"),
    ("Slack Helper", "Slack"),
    ("Webex", "Webex"),
    ("Discord", "Discord"),
    ("Skype", "Skype"),
];

/// Debounce: ignore mic activations within this window after the last notification
const DEBOUNCE_SECS: u64 = 30;

/// Managed Tauri state for the call detector
pub struct CallDetectorState(pub std::sync::Mutex<Option<CallDetector>>);

/// Start or stop the detector to match the desired enabled state
pub fn sync_detector(state: &CallDetectorState, enabled: bool, app_handle: &tauri::AppHandle) {

    let mut detector = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if enabled {
        // Request notification permission if needed
        use tauri::plugin::PermissionState;
        if app_handle
            .notification()
            .permission_state()
            .unwrap_or(PermissionState::Prompt)
            != PermissionState::Granted
        {
            let _ = app_handle.notification().request_permission();
        }
        if detector.is_none() {
            *detector = Some(CallDetector::start(app_handle.clone()));
        }
    } else if let Some(mut d) = detector.take() {
        d.stop();
    }
}

pub struct CallDetector {
    should_stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl CallDetector {
    pub fn start(app_handle: tauri::AppHandle) -> Self {
        let should_stop = Arc::new(AtomicBool::new(false));
        let stop_flag = should_stop.clone();

        let handle = thread::spawn(move || {
            run_detector(stop_flag, app_handle);
        });

        CallDetector {
            should_stop,
            handle: Some(handle),
        }
    }

    pub fn stop(&mut self) {
        self.should_stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for CallDetector {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Core Audio property listener callback
extern "C" fn device_is_alive_changed(
    _device_id: u32,
    _num_addresses: u32,
    _addresses: *const AudioObjectPropertyAddress,
    user_data: *mut c_void,
) -> i32 {
    let flag = unsafe { &*(user_data as *const AtomicBool) };
    flag.store(true, Ordering::SeqCst);
    0 // noErr
}

#[repr(C)]
struct AudioObjectPropertyAddress {
    selector: u32,
    scope: u32,
    element: u32,
}

// Core Audio constants
const K_AUDIO_OBJECT_SYSTEM_OBJECT: u32 = 1;
const K_AUDIO_HARDWARE_PROPERTY_DEVICES: u32 = u32::from_be_bytes(*b"dev#");
const K_AUDIO_DEVICE_PROPERTY_DEVICE_IS_RUNNING_SOMEWHERE: u32 = u32::from_be_bytes(*b"gone");
const K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL: u32 = u32::from_be_bytes(*b"glob");
const K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT: u32 = u32::from_be_bytes(*b"inpt");
const K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN: u32 = 0;
const K_AUDIO_DEVICE_PROPERTY_STREAMS: u32 = u32::from_be_bytes(*b"stm#");

unsafe extern "C" {
    fn AudioObjectGetPropertyDataSize(
        object_id: u32,
        address: *const AudioObjectPropertyAddress,
        qualifier_data_size: u32,
        qualifier_data: *const c_void,
        out_data_size: *mut u32,
    ) -> i32;

    fn AudioObjectGetPropertyData(
        object_id: u32,
        address: *const AudioObjectPropertyAddress,
        qualifier_data_size: u32,
        qualifier_data: *const c_void,
        io_data_size: *mut u32,
        out_data: *mut c_void,
    ) -> i32;

    fn AudioObjectAddPropertyListener(
        object_id: u32,
        address: *const AudioObjectPropertyAddress,
        listener: extern "C" fn(u32, u32, *const AudioObjectPropertyAddress, *mut c_void) -> i32,
        client_data: *mut c_void,
    ) -> i32;

    fn AudioObjectRemovePropertyListener(
        object_id: u32,
        address: *const AudioObjectPropertyAddress,
        listener: extern "C" fn(u32, u32, *const AudioObjectPropertyAddress, *mut c_void) -> i32,
        client_data: *mut c_void,
    ) -> i32;
}

/// Get all audio device IDs
fn get_audio_device_ids() -> Vec<u32> {
    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_HARDWARE_PROPERTY_DEVICES,
        scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };

    let mut size: u32 = 0;
    let status = unsafe {
        AudioObjectGetPropertyDataSize(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            0,
            std::ptr::null(),
            &mut size,
        )
    };
    if status != 0 || size == 0 {
        return vec![];
    }

    let count = size as usize / std::mem::size_of::<u32>();
    let mut device_ids = vec![0u32; count];
    let status = unsafe {
        AudioObjectGetPropertyData(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            device_ids.as_mut_ptr() as *mut c_void,
        )
    };
    if status != 0 {
        return vec![];
    }

    device_ids
}

/// Check if a device has input streams (is a microphone/input device)
fn device_has_input(device_id: u32) -> bool {
    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_DEVICE_PROPERTY_STREAMS,
        scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT,
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };

    let mut size: u32 = 0;
    let status = unsafe {
        AudioObjectGetPropertyDataSize(device_id, &address, 0, std::ptr::null(), &mut size)
    };
    status == 0 && size > 0
}

/// Check if a device's mic is currently running
fn device_is_running(device_id: u32) -> bool {
    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_DEVICE_PROPERTY_DEVICE_IS_RUNNING_SOMEWHERE,
        scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };

    let mut running: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(
            device_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            &mut running as *mut u32 as *mut c_void,
        )
    };
    status == 0 && running != 0
}

/// Detect which call app might be running
fn detect_call_app() -> Option<String> {
    let output = Command::new("ps").args(["-eo", "comm="]).output().ok()?;
    let ps_output = String::from_utf8_lossy(&output.stdout);

    for (process_name, friendly_name) in CALL_APP_PROCESSES {
        for line in ps_output.lines() {
            let basename = line.rsplit('/').next().unwrap_or(line).trim();
            if basename == *process_name {
                return Some(friendly_name.to_string());
            }
        }
    }
    None
}

/// Get pipeline names for notification body
fn get_pipeline_names() -> Vec<String> {
    let settings = config::load_settings();

    let pipelines = match pipelines::load_pipelines() {
        Ok(p) => p,
        Err(_) => return vec![],
    };

    let mut names: Vec<String> = Vec::new();

    // Put last-used pipeline first
    if let Some(ref last) = settings.last_used_pipeline {
        if pipelines.contains_key(last) {
            names.push(last.clone());
        }
    }

    // Put default pipeline second (if different from last-used)
    if let Some(ref default) = settings.default_pipeline {
        if pipelines.contains_key(default) && !names.contains(default) {
            names.push(default.clone());
        }
    }

    // Add remaining pipelines
    for name in pipelines.keys() {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }

    // Limit to 5
    names.truncate(5);
    names
}

/// Send macOS notification only — no window show, no auto-start.
/// Recording starts only when the user clicks the notification (which activates the app).
fn send_notification(app_handle: &tauri::AppHandle, call_app: Option<&str>, _pipeline_names: &[String]) {
    let title = match call_app {
        Some(app_name) => format!("{app_name} — Call Detected"),
        None => "Call Detected".to_string(),
    };

    let _ = app_handle
        .notification()
        .builder()
        .title(&title)
        .body("Click to start recording")
        .show();

    // Set pending flag — will be consumed when window gets focus (user clicked notification)
    PENDING_CALL.store(true, Ordering::SeqCst);
}

/// Test notification — callable from frontend
#[tauri::command]
pub fn test_call_notification(app_handle: tauri::AppHandle) -> Result<String, String> {
    let call_app = detect_call_app();
    let pipeline_names = get_pipeline_names();
    let mic_active = {
        let ids = get_audio_device_ids();
        ids.iter().any(|&id| device_has_input(id) && device_is_running(id))
    };

    let status = format!(
        "mic_active={}, call_app={:?}, pipelines={:?}",
        mic_active, call_app, pipeline_names
    );
    log::debug!("call_detector test: {status}");

    send_notification(&app_handle, call_app.as_deref(), &pipeline_names);

    Ok(status)
}

/// Main detector loop
fn run_detector(should_stop: Arc<AtomicBool>, app_handle: tauri::AppHandle) {
    // Find input devices and track their initial running state
    let device_ids = get_audio_device_ids();
    let input_devices: Vec<u32> = device_ids
        .into_iter()
        .filter(|&id| device_has_input(id))
        .collect();

    if input_devices.is_empty() {
        log::warn!("call_detector: no input devices found");
        return;
    }

    log::info!("call_detector: monitoring {} input device(s)", input_devices.len());

    // Install property listeners on all input devices
    let mic_activated = Arc::new(AtomicBool::new(false));

    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_DEVICE_PROPERTY_DEVICE_IS_RUNNING_SOMEWHERE,
        scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };

    for &device_id in &input_devices {
        unsafe {
            AudioObjectAddPropertyListener(
                device_id,
                &address,
                device_is_alive_changed,
                Arc::as_ptr(&mic_activated) as *mut c_void,
            );
        }
    }

    // Small delay to let notification permission grant propagate
    thread::sleep(Duration::from_secs(2));

    // Check if mic is already active (call in progress) — notify immediately
    let mut was_running = input_devices.iter().any(|&id| device_is_running(id));
    let mut last_notification = Instant::now() - Duration::from_secs(DEBOUNCE_SECS + 1);

    if was_running {
        let call_app = detect_call_app();
        let pipeline_names = get_pipeline_names();
        log::info!("call_detector: mic already active on start, app={:?}", call_app);
        send_notification(&app_handle, call_app.as_deref(), &pipeline_names);
        last_notification = Instant::now();
    }

    // Poll loop — the listener sets the flag, we check periodically
    while !should_stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(500));

        if !mic_activated.swap(false, Ordering::SeqCst) {
            continue;
        }

        let is_running = input_devices.iter().any(|&id| device_is_running(id));

        // Only notify on transition from not-running to running
        if is_running && !was_running {
            let since_last = last_notification.elapsed();
            if since_last >= Duration::from_secs(DEBOUNCE_SECS) {
                let call_app = detect_call_app();
                let pipeline_names = get_pipeline_names();

                log::info!("call_detector: mic activated, app={:?}", call_app);

                send_notification(&app_handle, call_app.as_deref(), &pipeline_names);
                last_notification = Instant::now();
            }
        }

        was_running = is_running;
    }

    // Cleanup: remove listeners
    for &device_id in &input_devices {
        unsafe {
            AudioObjectRemovePropertyListener(
                device_id,
                &address,
                device_is_alive_changed,
                Arc::as_ptr(&mic_activated) as *mut c_void,
            );
        }
    }

    log::info!("call_detector: stopped");
}
