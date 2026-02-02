use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::PathBuf;
use std::os::unix::fs::PermissionsExt;
use crate::storage::get_data_dir;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TranscriptionProvider {
    LocalWhisper,
    OpenAI,
    Google,
    Anthropic,
}

/// API keys for cloud AI services
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ApiKeys {
    #[serde(default)]
    pub openai: Option<String>,
    #[serde(default)]
    pub google: Option<String>,
    #[serde(default)]
    pub anthropic: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WhisperModelSize {
    Tiny,
    Base,
    Small,
    Medium,
    Large,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TranscriptionConfig {
    pub enabled: bool,
    pub provider: TranscriptionProvider,
    pub whisper_model: Option<WhisperModelSize>,
    #[serde(default)]
    pub api_keys: ApiKeys,
    // Legacy field for migration - will be removed in future
    #[serde(skip_serializing, default)]
    pub api_key: Option<String>,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: TranscriptionProvider::LocalWhisper,
            whisper_model: Some(WhisperModelSize::Base),
            api_keys: ApiKeys::default(),
            api_key: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    pub storage_path: String,
    pub auto_discard_seconds: u32,
    pub theme: String,
    #[serde(default)]
    pub onboarding_completed: bool,
    #[serde(default)]
    pub transcription: TranscriptionConfig,
    #[serde(default)]
    pub show_recording_notification: bool,
    /// Save only the mixed audio file (default: true)
    #[serde(default = "default_true")]
    pub save_mix_only: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            storage_path: get_data_dir().to_string_lossy().to_string(),
            auto_discard_seconds: 3,
            theme: "neon-purple".to_string(),
            onboarding_completed: false,
            transcription: TranscriptionConfig::default(),
            show_recording_notification: true,
            save_mix_only: true,
        }
    }
}

pub fn get_config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".nbp")
}

pub fn get_settings_path() -> PathBuf {
    get_config_dir().join("settings.json")
}

pub fn get_models_dir() -> PathBuf {
    get_config_dir().join("models")
}

#[tauri::command]
pub fn load_settings() -> AppSettings {
    let path = get_settings_path();
    if !path.exists() {
        return AppSettings::default();
    }

    match File::open(&path) {
        Ok(file) => {
            let mut settings: AppSettings = serde_json::from_reader(file).unwrap_or_default();

            // Migration: move legacy api_key to api_keys.openai
            if let Some(legacy_key) = settings.transcription.api_key.take() {
                if settings.transcription.api_keys.openai.is_none() {
                    settings.transcription.api_keys.openai = Some(legacy_key);
                }
            }

            settings
        },
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
    let file = File::create(&path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, &settings).map_err(|e| e.to_string())?;

    // Set file permissions to 600 (user read/write only) for security
    let mut perms = fs::metadata(&path).map_err(|e| e.to_string())?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;

    Ok(())
}

/// Get templates directory path
pub fn get_templates_dir() -> PathBuf {
    get_config_dir().join("templates")
}
