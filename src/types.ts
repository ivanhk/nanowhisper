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

export interface AppSettings {
  provider: string;
  api_key: string;
  gemini_api_key: string;
  dashscope_api_key: string;
  custom_api_url: string;
  custom_api_key: string;
  model: string;
  language: string;
  shortcut: string;
  sound_enabled: boolean;
  overlay_rx: number | null;
  overlay_ry: number | null;
  history_limit: number;
  api_key_validated: boolean;
  recording_mode: "toggle" | "hold";
  trigger_delay_ms: number;
  max_recording_seconds: number;
  model_timeout_seconds: number;
}
