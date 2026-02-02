# Permissions Module

## macOS Permissions

| Permission | Purpose | Grant Method |
|------------|---------|--------------|
| Microphone | Audio input | System prompt (automatic) |
| Screen Recording | System audio capture | System Preferences (manual) |
| Files/Folders | Data storage | User-initiated (automatic) |

## Permission Checking

```rust
#[cfg(target_os = "macos")]
pub mod macos_permissions {
    use std::process::Command;

    pub fn check_mic_permission() -> bool {
        // Check via AVFoundation authorization status
        let output = Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to get exists of every process whose name is \"coreaudiod\""])
            .output();

        // Simplified check - real implementation uses objc bindings
        true
    }

    pub fn check_screen_recording_permission() -> bool {
        // Check via CGWindowListCreateImageFromArray
        // Returns false if permission not granted

        // Simplified - real implementation attempts to capture
        // and checks if result is valid
        true
    }

    pub fn request_mic_permission() {
        // Triggers system permission dialog
        // Use AVCaptureDevice.requestAccess(for: .audio)
    }

    pub fn open_screen_recording_preferences() {
        let _ = Command::new("open")
            .args(["x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"])
            .spawn();
    }

    pub fn open_microphone_preferences() {
        let _ = Command::new("open")
            .args(["x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"])
            .spawn();
    }
}
```

## Tauri Commands

```rust
#[derive(Serialize, Clone)]
pub struct PermissionStatus {
    pub mic: bool,
    pub screen_recording: bool,
}

#[tauri::command]
pub fn get_permissions() -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        PermissionStatus {
            mic: macos_permissions::check_mic_permission(),
            screen_recording: macos_permissions::check_screen_recording_permission(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        PermissionStatus {
            mic: true,
            screen_recording: false, // Not available on other platforms
        }
    }
}

#[tauri::command]
pub fn request_mic_access() {
    #[cfg(target_os = "macos")]
    macos_permissions::request_mic_permission();
}

#[tauri::command]
pub fn open_permission_settings(permission_type: String) {
    #[cfg(target_os = "macos")]
    match permission_type.as_str() {
        "mic" => macos_permissions::open_microphone_preferences(),
        "screen" => macos_permissions::open_screen_recording_preferences(),
        _ => {}
    }
}
```

## Entitlements (macOS)

```xml
<!-- src-tauri/entitlements.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Microphone access -->
    <key>com.apple.security.device.audio-input</key>
    <true/>

    <!-- Screen recording (for system audio) -->
    <key>com.apple.security.device.screen-capture</key>
    <true/>

    <!-- Allow JIT for Whisper/ML -->
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>

    <!-- Hardened runtime -->
    <key>com.apple.security.app-sandbox</key>
    <false/>
</dict>
</plist>
```

## Info.plist Descriptions

```xml
<!-- Required for permission dialogs -->
<key>NSMicrophoneUsageDescription</key>
<string>MyApp needs microphone access to record audio.</string>

<key>NSScreenCaptureUsageDescription</key>
<string>MyApp needs screen recording permission to capture system audio.</string>
```

## Frontend Permission UI

```javascript
async function checkPermissions() {
    const status = await invoke("get_permissions");

    updatePermissionUI("mic", status.mic);
    updatePermissionUI("screen", status.screen_recording);
}

async function requestPermission(type) {
    if (type === "mic") {
        await invoke("request_mic_access");
    } else {
        await invoke("open_permission_settings", { permissionType: type });
    }

    // Re-check after delay
    setTimeout(checkPermissions, 1000);
}
```
