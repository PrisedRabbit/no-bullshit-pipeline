use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::PathBuf;
use uuid::Uuid;
use chrono::Utc;

/// Metadata for a recording session
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecordingMetadata {
    pub id: String,
    pub created_at: String,
    pub title: String,
    pub tags: Vec<String>,
    pub audio: AudioFiles,
}

/// Audio files (mic + system)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioFiles {
    pub mic: Option<AudioInfo>,
    pub system: Option<AudioInfo>,
}

/// Audio file information
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioInfo {
    pub file: String,
    pub duration_sec: f64,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Get the data directory path (~/nbp-data/)
pub fn get_data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("nbp-data")
}

/// Get the path for a specific recording directory
pub fn get_recording_dir(id: &str) -> PathBuf {
    get_data_dir().join(id)
}

/// Create a new recording with metadata
pub fn create_recording(title: String, tags: Vec<String>) -> Result<RecordingMetadata, String> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    
    let metadata = RecordingMetadata {
        id: id.clone(),
        created_at,
        title,
        tags,
        audio: AudioFiles {
            mic: None,
            system: None,
        },
    };
    
    // Create the recording directory
    let recording_dir = get_recording_dir(&id);
    fs::create_dir_all(&recording_dir).map_err(|e| e.to_string())?;
    
    // Write metadata.json
    write_metadata(&metadata)?;
    
    Ok(metadata)
}

/// Write metadata to disk
pub fn write_metadata(metadata: &RecordingMetadata) -> Result<(), String> {
    let metadata_path = get_recording_dir(&metadata.id).join("metadata.json");
    let file = File::create(metadata_path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, metadata).map_err(|e| e.to_string())?;
    Ok(())
}

/// Update tags for a recording
#[tauri::command]
pub fn update_tags(recording_id: &str, tags: Vec<String>) -> Result<(), String> {
    let mut metadata = read_metadata(recording_id)?;
    metadata.tags = tags;
    write_metadata(&metadata)?;
    Ok(())
}

/// Delete a recording and all its files
#[tauri::command]
pub fn delete_recording(recording_id: &str) -> Result<(), String> {
    let recording_dir = get_recording_dir(recording_id);
    if recording_dir.exists() {
        fs::remove_dir_all(recording_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Read metadata from disk
#[tauri::command]
pub fn read_metadata(recording_id: &str) -> Result<RecordingMetadata, String> {
    let metadata_path = get_recording_dir(recording_id).join("metadata.json");
    let file = File::open(metadata_path).map_err(|e| e.to_string())?;
    let metadata = serde_json::from_reader(file).map_err(|e| e.to_string())?;
    Ok(metadata)
}

/// List all recordings, sorted by created_at (newest first)
#[tauri::command]
pub fn list_recordings() -> Result<Vec<RecordingMetadata>, String> {
    let data_dir = get_data_dir();
    
    if !data_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut recordings = Vec::new();
    
    for entry in fs::read_dir(&data_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        
        if path.is_dir() {
            let metadata_path = path.join("metadata.json");
            if metadata_path.exists() {
                if let Ok(file) = File::open(&metadata_path) {
                    if let Ok(metadata) = serde_json::from_reader::<_, RecordingMetadata>(file) {
                        recordings.push(metadata);
                    }
                }
            }
        }
    }
    
    // Sort by created_at (newest first)
    recordings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    
    Ok(recordings)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_recording() {
        let metadata = create_recording(
            "test recording".to_string(),
            vec!["test".to_string(), "storage".to_string()]
        );
        
        assert!(metadata.is_ok());
        let metadata = metadata.unwrap();
        
        // Verify UUID format (36 chars with hyphens)
        assert_eq!(metadata.id.len(), 36);
        assert!(metadata.id.contains('-'));
        
        // Verify ISO 8601 format (ends with Z)
        assert!(metadata.created_at.ends_with('Z'));
        
        // Verify fields
        assert_eq!(metadata.title, "test recording");
        assert_eq!(metadata.tags, vec!["test", "storage"]);
        assert!(metadata.audio.mic.is_none());
        assert!(metadata.audio.system.is_none());
    }
    
    #[test]
    fn test_metadata_roundtrip() {
        let original = create_recording(
            "roundtrip test".to_string(),
            vec!["test".to_string()]
        ).unwrap();
        
        // Write is already done by create_recording
        
        // Read back
        let read_back = read_metadata(&original.id).unwrap();
        
        // Verify equality
        assert_eq!(original.id, read_back.id);
        assert_eq!(original.created_at, read_back.created_at);
        assert_eq!(original.title, read_back.title);
        assert_eq!(original.tags, read_back.tags);
    }
}
