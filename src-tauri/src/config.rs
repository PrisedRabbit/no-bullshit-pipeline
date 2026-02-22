use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::PathBuf;
use std::os::unix::fs::PermissionsExt;
use crate::storage::get_data_dir;
use crate::integrations::IntegrationsConfig;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TranscriptionProvider {
    FluidAudio,
    LocalWhisper,
    OpenAI,
    Google,
    Anthropic,
    #[serde(other)]
    Unknown,
}

/// API keys for cloud AI services
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ApiKeys {
    #[serde(default)]
    pub openai: Option<String>,
    #[serde(default)]
    pub google: Option<String>,
    #[serde(default)]
    pub anthropic: Option<String>,
}

/// Per-provider configuration (API key + capabilities)
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ProviderConfig {
    /// API key for this provider (None for local providers)
    #[serde(default)]
    pub api_key: Option<String>,
    /// Capability badges: e.g. ["Transcription", "Processing", "Embedding"]
    #[serde(default)]
    pub capabilities: Vec<String>,
}

impl ProviderConfig {
    pub fn with_capabilities(caps: &[&str]) -> Self {
        Self {
            api_key: None,
            capabilities: caps.iter().map(|s| s.to_string()).collect(),
        }
    }
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
            enabled: true,
            provider: TranscriptionProvider::FluidAudio,
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
    #[serde(default)]
    pub integrations: IntegrationsConfig,
    /// Default pipeline to auto-assign to new recordings (set in Settings > Audio)
    #[serde(default)]
    pub default_pipeline: Option<String>,
    /// Last pipeline used — highlighted in chip bar on next launch
    #[serde(default)]
    pub last_used_pipeline: Option<String>,
    /// Whether the user has completed the interactive UI walkthrough
    #[serde(default)]
    pub walkthrough_completed: bool,
    /// Local LLM settings
    #[serde(default)]
    pub local_llm: LocalLlmConfig,
    /// Provider-first model config: keyed by provider ID ("openai", "google", "anthropic", "local")
    #[serde(default = "default_providers")]
    pub providers: HashMap<String, ProviderConfig>,
}

fn default_true() -> bool {
    true
}

fn default_providers() -> HashMap<String, ProviderConfig> {
    let mut map = HashMap::new();
    map.insert("openai".to_string(), ProviderConfig::with_capabilities(&["Transcription", "Processing"]));
    map.insert("google".to_string(), ProviderConfig::with_capabilities(&["Transcription", "Processing"]));
    map.insert("anthropic".to_string(), ProviderConfig::with_capabilities(&["Processing"]));
    map.insert("local".to_string(), ProviderConfig::with_capabilities(&["Processing"]));
    map
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
            integrations: IntegrationsConfig::default(),
            default_pipeline: None,
            last_used_pipeline: None,
            walkthrough_completed: false,
            local_llm: LocalLlmConfig::default(),
            providers: default_providers(),
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

pub fn get_llm_models_dir() -> PathBuf {
    get_config_dir().join("models").join("llm")
}

/// Local LLM configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LocalLlmConfig {
    pub enabled: bool,
    /// Selected model ID (e.g. "phi-3.5-mini")
    pub model_id: Option<String>,
    /// Number of layers to offload to GPU (99 = all)
    pub gpu_layers: u32,
    /// Context window size in tokens
    pub context_size: u32,
}

impl Default for LocalLlmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_id: None,
            gpu_layers: 99,
            context_size: 8192,
        }
    }
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

            // Migration v1: move legacy api_key to api_keys.openai
            if let Some(legacy_key) = settings.transcription.api_key.take() {
                if settings.transcription.api_keys.openai.is_none() {
                    settings.transcription.api_keys.openai = Some(legacy_key);
                }
            }

            // Migration v2: sync transcription.api_keys into providers map
            // If a provider entry has no api_key set but transcription.api_keys has one, migrate it.
            // This ensures the new providers field stays populated after upgrade.
            for (provider_id, legacy_key) in [
                ("openai", settings.transcription.api_keys.openai.clone()),
                ("google", settings.transcription.api_keys.google.clone()),
                ("anthropic", settings.transcription.api_keys.anthropic.clone()),
            ] {
                if let Some(key) = legacy_key {
                    let entry = settings.providers.entry(provider_id.to_string())
                        .or_insert_with(|| ProviderConfig::with_capabilities(&[]));
                    if entry.api_key.is_none() {
                        entry.api_key = Some(key);
                    }
                }
            }

            // Ensure all default providers exist (for existing settings files missing new entries)
            for (id, default_cfg) in default_providers() {
                settings.providers.entry(id).or_insert(default_cfg);
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

/// Resolve the API key for a provider.
/// Checks `providers` map first, then falls back to `transcription.api_keys` for backward compat.
pub fn get_api_key_for_provider(settings: &AppSettings, provider: &str) -> Option<String> {
    // Check new providers map first
    if let Some(cfg) = settings.providers.get(provider) {
        if cfg.api_key.is_some() {
            return cfg.api_key.clone();
        }
    }
    // Fall back to legacy transcription.api_keys
    match provider {
        "openai" => settings.transcription.api_keys.openai.clone(),
        "google" => settings.transcription.api_keys.google.clone(),
        "anthropic" => settings.transcription.api_keys.anthropic.clone(),
        _ => None,
    }
}

/// Get templates directory path
pub fn get_templates_dir() -> PathBuf {
    get_config_dir().join("templates")
}

/// Get integrations directory path (~/.nbp/integrations/)
pub fn get_integrations_dir() -> PathBuf {
    get_config_dir().join("integrations")
}
