use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_api_key")]
    pub api_key: String,
    #[serde(default)]
    pub gemini_api_key: String,
    #[serde(default)]
    pub dashscope_api_key: String,
    #[serde(default)]
    pub custom_api_url: String,
    #[serde(default)]
    pub custom_api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_shortcut")]
    pub shortcut: String,
    #[serde(default = "default_sound_enabled")]
    pub sound_enabled: bool,
    #[serde(default)]
    pub overlay_rx: Option<f64>,
    #[serde(default)]
    pub overlay_ry: Option<f64>,
    #[serde(default = "default_history_limit")]
    pub history_limit: i64,
    #[serde(default)]
    pub api_key_validated: bool,
    #[serde(default = "default_recording_mode")]
    pub recording_mode: String,
    #[serde(default = "default_trigger_delay_ms")]
    pub trigger_delay_ms: i64,
    #[serde(default = "default_max_recording_seconds")]
    pub max_recording_seconds: i64,
    #[serde(default = "default_model_timeout_seconds")]
    pub model_timeout_seconds: i64,
}

fn default_provider() -> String {
    "openai".to_string()
}
fn default_api_key() -> String {
    String::new()
}
fn default_model() -> String {
    "gpt-4o-transcribe".to_string()
}
fn default_language() -> String {
    "auto".to_string()
}
fn default_shortcut() -> String {
    String::new()
}
fn default_sound_enabled() -> bool {
    true
}
fn default_history_limit() -> i64 {
    50
}
fn default_recording_mode() -> String {
    "toggle".to_string()
}
fn default_trigger_delay_ms() -> i64 {
    400
}
fn default_max_recording_seconds() -> i64 {
    60
}
fn default_model_timeout_seconds() -> i64 {
    30
}

pub fn normalize_model_timeout_seconds(timeout_seconds: i64) -> i64 {
    timeout_seconds.clamp(15, 600)
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            api_key: default_api_key(),
            gemini_api_key: String::new(),
            dashscope_api_key: String::new(),
            custom_api_url: String::new(),
            custom_api_key: String::new(),
            model: default_model(),
            language: default_language(),
            shortcut: default_shortcut(),
            sound_enabled: default_sound_enabled(),
            overlay_rx: None,
            overlay_ry: None,
            history_limit: default_history_limit(),
            api_key_validated: false,
            recording_mode: default_recording_mode(),
            trigger_delay_ms: default_trigger_delay_ms(),
            max_recording_seconds: default_max_recording_seconds(),
            model_timeout_seconds: default_model_timeout_seconds(),
        }
    }
}

fn settings_path() -> PathBuf {
    crate::data_dir().join("settings.json")
}

pub fn get_settings() -> AppSettings {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let mut settings = serde_json::from_str::<AppSettings>(&content).unwrap_or_default();
            settings.model_timeout_seconds =
                normalize_model_timeout_seconds(settings.model_timeout_seconds);
            settings
        }
        Err(_) => AppSettings::default(),
    }
}

pub fn save_settings(settings: &AppSettings) {
    let dir = crate::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("settings.json");
    let mut normalized = settings.clone();
    normalized.model_timeout_seconds =
        normalize_model_timeout_seconds(normalized.model_timeout_seconds);
    if let Ok(json) = serde_json::to_string_pretty(&normalized) {
        let _ = std::fs::write(&path, json);
    }
}
