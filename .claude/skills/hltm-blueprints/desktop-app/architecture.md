# Architecture

## Project Structure

```
project/
├── src/                        # Frontend (HTML/JS/CSS)
│   ├── index.html             # Main HTML
│   ├── main.js                # Frontend logic
│   ├── styles.css             # Styling
│   └── viewManager.js         # View state (optional)
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── lib.rs             # Entry point, command registration
│   │   ├── config.rs          # Settings management
│   │   ├── storage.rs         # Data persistence
│   │   └── [feature].rs       # Feature modules
│   ├── Cargo.toml             # Dependencies
│   ├── tauri.conf.json        # Tauri config
│   ├── capabilities/          # Security capabilities
│   └── icons/                 # App icons
├── build.sh                    # Build script (macOS signing)
└── README.md
```

## IPC Pattern (Frontend ↔ Backend)

### Frontend → Backend (Commands)

```javascript
// Frontend: src/main.js
const { invoke } = window.__TAURI__.core;

// Call Rust command
const result = await invoke("command_name", { param1: "value", param2: 123 });
```

```rust
// Backend: src-tauri/src/lib.rs
#[tauri::command]
fn command_name(param1: String, param2: i32) -> Result<String, String> {
    // Implementation
    Ok("result".to_string())
}

// Register in lib.rs
.invoke_handler(tauri::generate_handler![command_name])
```

### Backend → Frontend (Events)

```rust
// Backend: emit event
app_handle.emit("event-name", payload)?;
```

```javascript
// Frontend: listen
const { listen } = window.__TAURI__.event;
await listen("event-name", (event) => {
  console.log(event.payload);
});
```

## Settings Pattern

```rust
// src-tauri/src/config.rs
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    pub storage_path: String,
    pub theme: String,
    // Add more settings
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            storage_path: get_data_dir().to_string_lossy().to_string(),
            theme: "default".to_string(),
        }
    }
}

pub fn get_config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".myapp")
}

pub fn get_settings_path() -> PathBuf {
    get_config_dir().join("settings.json")
}

#[tauri::command]
pub fn load_settings() -> AppSettings {
    let path = get_settings_path();
    if !path.exists() {
        return AppSettings::default();
    }
    match File::open(path) {
        Ok(file) => serde_json::from_reader(file).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

#[tauri::command]
pub fn save_settings(settings: AppSettings) -> Result<(), String> {
    let config_dir = get_config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    }
    let path = get_settings_path();
    let file = File::create(path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, &settings).map_err(|e| e.to_string())?;
    Ok(())
}
```

## Data Storage Pattern

```
~/.myapp/                      # Config directory
├── settings.json             # App settings
└── models/                   # Downloaded models (if any)

~/myapp-data/                  # Data directory (user configurable)
├── {uuid}/                   # Item folder
│   ├── metadata.json        # Item metadata
│   ├── data.bin             # Item data
│   └── output/              # Processed outputs
└── index.json               # Optional index file
```

## Error Handling

```rust
// Commands return Result<T, String> for error messages
#[tauri::command]
fn risky_operation() -> Result<Data, String> {
    do_something()
        .map_err(|e| format!("Operation failed: {}", e))?;
    Ok(data)
}
```

```javascript
// Frontend error handling
try {
    const result = await invoke("risky_operation");
} catch (err) {
    console.error("Error:", err);
    showErrorToUser(err);
}
```

## Theming Pattern

```css
/* src/styles.css */
:root {
  --bg-primary: #1a1a2e;
  --bg-secondary: #16213e;
  --text-primary: #eee;
  --text-secondary: #888;
  --accent: #a855f7;
  --border-color: #333;
}

[data-theme="light"] {
  --bg-primary: #fff;
  --bg-secondary: #f5f5f5;
  --text-primary: #111;
  --text-secondary: #666;
  --accent: #7c3aed;
  --border-color: #ddd;
}
```

```javascript
// Apply theme
document.documentElement.setAttribute("data-theme", settings.theme);
```
