export interface HistoryEntry {
  id: number;
  text: string;
  model: string;
  timestamp: number;
  duration_ms: number | null;
  audio_path: string | null;
  input_tokens: number | null;
  output_tokens: number | null;
}

export interface Statistics {
  input_tokens: number;
  output_tokens: number;
  total_duration_ms: number;
  count: number;
}

export type ProviderId = "openai" | "gemini" | "dashscope" | "custom";

export interface ProviderSettings {
  api_key: string;
  api_url: string;
  model: string;
  validated: boolean;
}

export interface ProviderConfigMap {
  openai: ProviderSettings;
  gemini: ProviderSettings;
  dashscope: ProviderSettings;
  custom: ProviderSettings;
}

export interface AppSettings {
  provider: ProviderId;
  providers: ProviderConfigMap;
  language: string;
  shortcut: string;
  sound_enabled: boolean;
  overlay_rx: number | null;
  overlay_ry: number | null;
  history_limit: number;
  recording_mode: "toggle" | "hold";
  trigger_delay_ms: number;
  max_recording_seconds: number;
  model_timeout_seconds: number;
}
