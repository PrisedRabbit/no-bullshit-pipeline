# Storage Module

## Directory Structure

```
~/.myapp/                      # Config directory
├── settings.json             # App settings
├── models/                   # Downloaded models
└── templates/                # User templates

~/myapp-data/                  # Data directory
├── {uuid}/                   # Item folder
│   ├── metadata.json        # Item metadata
│   └── files/               # Item files
└── .index                   # Optional cache/index
```

## Settings Storage

```rust
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppSettings {
    pub storage_path: String,
    pub theme: String,
    #[serde(default)]
    pub feature_flags: FeatureFlags,
}

pub fn get_config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or(".".into());
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
    File::open(path)
        .ok()
        .and_then(|f| serde_json::from_reader(f).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn save_settings(settings: AppSettings) -> Result<(), String> {
    let config_dir = get_config_dir();
    fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

    let path = get_settings_path();
    let file = File::create(path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, &settings).map_err(|e| e.to_string())?;

    Ok(())
}
```

## Item Metadata

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ItemMetadata {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub extra: serde_json::Value,
}

impl ItemMetadata {
    pub fn new(title: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            created_at: now.clone(),
            updated_at: now,
            tags: vec![],
            extra: serde_json::Value::Null,
        }
    }
}
```

## File Operations

```rust
use std::path::Path;

pub fn get_data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or(".".into());
    PathBuf::from(home).join("myapp-data")
}

pub fn get_item_dir(id: &str) -> PathBuf {
    get_data_dir().join(id)
}

pub fn ensure_item_dir(id: &str) -> Result<PathBuf, std::io::Error> {
    let dir = get_item_dir(id);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn save_metadata(metadata: &ItemMetadata) -> Result<(), String> {
    let dir = ensure_item_dir(&metadata.id).map_err(|e| e.to_string())?;
    let path = dir.join("metadata.json");
    let file = File::create(path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, metadata).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_metadata(id: &str) -> Option<ItemMetadata> {
    let path = get_item_dir(id).join("metadata.json");
    File::open(path)
        .ok()
        .and_then(|f| serde_json::from_reader(f).ok())
}

pub fn list_items() -> Vec<ItemMetadata> {
    let data_dir = get_data_dir();
    if !data_dir.exists() {
        return vec![];
    }

    fs::read_dir(data_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().to_string();
            load_metadata(&id)
        })
        .collect()
}

pub fn delete_item(id: &str) -> Result<(), String> {
    let dir = get_item_dir(id);
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

## Tauri Commands

```rust
#[tauri::command]
pub fn get_items() -> Vec<ItemMetadata> {
    list_items()
}

#[tauri::command]
pub fn create_item(title: String) -> Result<ItemMetadata, String> {
    let metadata = ItemMetadata::new(title);
    ensure_item_dir(&metadata.id).map_err(|e| e.to_string())?;
    save_metadata(&metadata)?;
    Ok(metadata)
}

#[tauri::command]
pub fn update_item(metadata: ItemMetadata) -> Result<(), String> {
    save_metadata(&metadata)
}

#[tauri::command]
pub fn remove_item(id: String) -> Result<(), String> {
    delete_item(&id)
}
```
