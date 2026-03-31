use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

const PROVIDER_OPENAI: &str = "openai";
const PROVIDER_GEMINI: &str = "gemini";
const PROVIDER_DASHSCOPE: &str = "dashscope";
const PROVIDER_CUSTOM: &str = "custom";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettings {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub validated: bool,
}

impl ProviderSettings {
    fn with_defaults(provider: &str) -> Self {
        Self {
            api_key: String::new(),
            api_url: String::new(),
            model: default_model_for_provider(provider),
            validated: false,
        }
    }

    fn normalize(&mut self, provider: &str) {
        if self.model.is_empty() {
            self.model = default_model_for_provider(provider);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfigs {
    #[serde(default = "default_openai_provider_settings")]
    pub openai: ProviderSettings,
    #[serde(default = "default_gemini_provider_settings")]
    pub gemini: ProviderSettings,
    #[serde(default = "default_dashscope_provider_settings")]
    pub dashscope: ProviderSettings,
    #[serde(default = "default_custom_provider_settings")]
    pub custom: ProviderSettings,
}

impl Default for ProviderConfigs {
    fn default() -> Self {
        Self {
            openai: default_openai_provider_settings(),
            gemini: default_gemini_provider_settings(),
            dashscope: default_dashscope_provider_settings(),
            custom: default_custom_provider_settings(),
        }
    }
}

impl ProviderConfigs {
    fn get(&self, provider: &str) -> &ProviderSettings {
        match provider {
            PROVIDER_GEMINI => &self.gemini,
            PROVIDER_DASHSCOPE => &self.dashscope,
            PROVIDER_CUSTOM => &self.custom,
            _ => &self.openai,
        }
    }

    fn get_mut(&mut self, provider: &str) -> &mut ProviderSettings {
        match provider {
            PROVIDER_GEMINI => &mut self.gemini,
            PROVIDER_DASHSCOPE => &mut self.dashscope,
            PROVIDER_CUSTOM => &mut self.custom,
            _ => &mut self.openai,
        }
    }

    fn normalize(&mut self) {
        self.openai.normalize(PROVIDER_OPENAI);
        self.gemini.normalize(PROVIDER_GEMINI);
        self.dashscope.normalize(PROVIDER_DASHSCOPE);
        self.custom.normalize(PROVIDER_CUSTOM);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub providers: ProviderConfigs,
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
    #[serde(default = "default_recording_mode")]
    pub recording_mode: String,
    #[serde(default = "default_trigger_delay_ms")]
    pub trigger_delay_ms: i64,
    #[serde(default = "default_max_recording_seconds")]
    pub max_recording_seconds: i64,
    #[serde(default = "default_model_timeout_seconds")]
    pub model_timeout_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyAppSettings {
    #[serde(default = "default_provider")]
    provider: String,
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    gemini_api_key: String,
    #[serde(default)]
    dashscope_api_key: String,
    #[serde(default)]
    custom_api_url: String,
    #[serde(default)]
    custom_api_key: String,
    #[serde(default = "default_model")]
    model: String,
    #[serde(default = "default_language")]
    language: String,
    #[serde(default = "default_shortcut")]
    shortcut: String,
    #[serde(default = "default_sound_enabled")]
    sound_enabled: bool,
    #[serde(default)]
    overlay_rx: Option<f64>,
    #[serde(default)]
    overlay_ry: Option<f64>,
    #[serde(default = "default_history_limit")]
    history_limit: i64,
    #[serde(default)]
    api_key_validated: bool,
    #[serde(default = "default_recording_mode")]
    recording_mode: String,
    #[serde(default = "default_trigger_delay_ms")]
    trigger_delay_ms: i64,
    #[serde(default = "default_max_recording_seconds")]
    max_recording_seconds: i64,
    #[serde(default = "default_model_timeout_seconds")]
    model_timeout_seconds: i64,
}

fn default_provider() -> String {
    PROVIDER_OPENAI.to_string()
}

fn default_model() -> String {
    default_model_for_provider(PROVIDER_OPENAI)
}

fn default_model_for_provider(provider: &str) -> String {
    match provider {
        PROVIDER_GEMINI => "gemini-3-flash-preview".to_string(),
        PROVIDER_DASHSCOPE => "qwen3-asr-flash".to_string(),
        PROVIDER_CUSTOM => "whisper-1".to_string(),
        _ => "gpt-4o-transcribe".to_string(),
    }
}

fn default_openai_provider_settings() -> ProviderSettings {
    ProviderSettings::with_defaults(PROVIDER_OPENAI)
}

fn default_gemini_provider_settings() -> ProviderSettings {
    ProviderSettings::with_defaults(PROVIDER_GEMINI)
}

fn default_dashscope_provider_settings() -> ProviderSettings {
    ProviderSettings::with_defaults(PROVIDER_DASHSCOPE)
}

fn default_custom_provider_settings() -> ProviderSettings {
    ProviderSettings::with_defaults(PROVIDER_CUSTOM)
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
            providers: ProviderConfigs::default(),
            language: default_language(),
            shortcut: default_shortcut(),
            sound_enabled: default_sound_enabled(),
            overlay_rx: None,
            overlay_ry: None,
            history_limit: default_history_limit(),
            recording_mode: default_recording_mode(),
            trigger_delay_ms: default_trigger_delay_ms(),
            max_recording_seconds: default_max_recording_seconds(),
            model_timeout_seconds: default_model_timeout_seconds(),
        }
    }
}

impl AppSettings {
    pub fn active_provider_settings(&self) -> &ProviderSettings {
        self.providers.get(&self.provider)
    }

    pub fn active_provider_settings_mut(&mut self) -> &mut ProviderSettings {
        self.providers.get_mut(&self.provider)
    }

    fn normalize(&mut self) {
        if !matches!(
            self.provider.as_str(),
            PROVIDER_OPENAI | PROVIDER_GEMINI | PROVIDER_DASHSCOPE | PROVIDER_CUSTOM
        ) {
            self.provider = default_provider();
        }
        self.providers.normalize();
        self.model_timeout_seconds = normalize_model_timeout_seconds(self.model_timeout_seconds);
    }
}

impl LegacyAppSettings {
    fn into_app_settings(self) -> AppSettings {
        let mut settings = AppSettings {
            provider: self.provider,
            providers: ProviderConfigs {
                openai: ProviderSettings {
                    api_key: self.api_key,
                    api_url: String::new(),
                    model: default_model_for_provider(PROVIDER_OPENAI),
                    validated: false,
                },
                gemini: ProviderSettings {
                    api_key: self.gemini_api_key,
                    api_url: String::new(),
                    model: default_model_for_provider(PROVIDER_GEMINI),
                    validated: false,
                },
                dashscope: ProviderSettings {
                    api_key: self.dashscope_api_key,
                    api_url: String::new(),
                    model: default_model_for_provider(PROVIDER_DASHSCOPE),
                    validated: false,
                },
                custom: ProviderSettings {
                    api_key: self.custom_api_key,
                    api_url: self.custom_api_url,
                    model: default_model_for_provider(PROVIDER_CUSTOM),
                    validated: false,
                },
            },
            language: self.language,
            shortcut: self.shortcut,
            sound_enabled: self.sound_enabled,
            overlay_rx: self.overlay_rx,
            overlay_ry: self.overlay_ry,
            history_limit: self.history_limit,
            recording_mode: self.recording_mode,
            trigger_delay_ms: self.trigger_delay_ms,
            max_recording_seconds: self.max_recording_seconds,
            model_timeout_seconds: self.model_timeout_seconds,
        };

        {
            let active = settings.active_provider_settings_mut();
            active.model = self.model;
            active.validated = self.api_key_validated;
        }

        settings.normalize();
        settings
    }
}

fn parse_settings(content: &str) -> AppSettings {
    let raw = match serde_json::from_str::<Value>(content) {
        Ok(raw) => raw,
        Err(_) => return AppSettings::default(),
    };

    let mut settings = if raw.get("providers").is_some() {
        serde_json::from_value::<AppSettings>(raw).unwrap_or_default()
    } else {
        serde_json::from_value::<LegacyAppSettings>(raw)
            .map(LegacyAppSettings::into_app_settings)
            .unwrap_or_default()
    };

    settings.normalize();
    settings
}

fn settings_path() -> PathBuf {
    crate::data_dir().join("settings.json")
}

pub fn get_settings() -> AppSettings {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => parse_settings(&content),
        Err(_) => AppSettings::default(),
    }
}

pub fn save_settings(settings: &AppSettings) {
    let dir = crate::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("settings.json");
    let mut normalized = settings.clone();
    normalized.normalize();
    if let Ok(json) = serde_json::to_string_pretty(&normalized) {
        let _ = std::fs::write(&path, json);
    }
}
