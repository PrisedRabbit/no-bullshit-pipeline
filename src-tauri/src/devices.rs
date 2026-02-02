//! Audio device enumeration module
//!
//! Provides functions to enumerate available audio input devices
//! and expose them to the frontend via Tauri commands.

use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

/// Information about an audio input device
#[derive(Debug, Clone, Serialize)]
pub struct AudioDeviceInfo {
    /// Unique device identifier (uses device name since cpal lacks stable IDs)
    pub id: String,
    /// Human-readable device name
    pub name: String,
    /// Default sample rate in Hz
    pub sample_rate: u32,
    /// Number of audio channels
    pub channels: u16,
    /// Whether this is the system default input device
    pub is_default: bool,
}

/// Lists all available audio input devices
///
/// Returns a vector of AudioDeviceInfo structs containing device details.
/// The default system input device is marked with is_default = true.
pub fn list_input_devices() -> Result<Vec<AudioDeviceInfo>, anyhow::Error> {
    let host = cpal::default_host();
    let default_device = host.default_input_device();
    let default_name = default_device.as_ref().and_then(|d| d.name().ok());

    let devices: Vec<AudioDeviceInfo> = host.input_devices()?
        .filter_map(|device| {
            let name = device.name().ok()?;
            let config = device.default_input_config().ok()?;
            Some(AudioDeviceInfo {
                id: name.clone(),
                name: name.clone(),
                sample_rate: config.sample_rate().0,
                channels: config.channels(),
                is_default: Some(&name) == default_name.as_ref(),
            })
        })
        .collect();

    Ok(devices)
}

/// Tauri command to get available input devices
///
/// Returns a list of audio input devices for the frontend to display.
#[tauri::command]
pub fn get_input_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    list_input_devices().map_err(|e| e.to_string())
}

/// Find an input device by name
///
/// Returns the device matching the given name, or None if not found.
/// Uses device name as identifier since cpal device IDs are unstable.
pub fn get_device_by_name(name: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    host.input_devices()
        .ok()?
        .find(|d| d.name().ok().as_deref() == Some(name))
}
