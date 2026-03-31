use crate::history::{HistoryEntry, HistoryManager, Statistics};
use crate::paste::EnigoState;
use crate::recorder::AudioRecorder;
use crate::settings::{self, AppSettings};
use crate::updater;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[tauri::command]
pub fn get_history(history: State<'_, Arc<HistoryManager>>) -> Result<Vec<HistoryEntry>, String> {
    history.get_entries().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_statistics(history: State<'_, Arc<HistoryManager>>) -> Result<Statistics, String> {
    history.get_statistics().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_history_entry(
    history: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history.delete_entry(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_history(history: State<'_, Arc<HistoryManager>>) -> Result<(), String> {
    history.clear_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_statistics(history: State<'_, Arc<HistoryManager>>) -> Result<(), String> {
    history.clear_statistics().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings() -> AppSettings {
    settings::get_settings()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) {
    let old_settings = settings::get_settings();
    settings::save_settings(&settings);

    // Hot-reload shortcut if changed
    if settings.shortcut != old_settings.shortcut {
        crate::re_register_shortcut(&app, &old_settings.shortcut, &settings);
    }
}

#[tauri::command]
pub fn check_accessibility() -> bool {
    crate::paste::is_accessibility_trusted()
}

#[tauri::command]
pub fn request_accessibility() -> bool {
    crate::paste::request_accessibility_with_prompt()
}

#[tauri::command]
pub fn check_microphone() -> bool {
    crate::permissions::check_microphone_permission()
}

#[tauri::command]
pub fn request_microphone() -> bool {
    crate::permissions::request_microphone_permission()
}

#[tauri::command]
pub async fn validate_api_key(
    _app: AppHandle,
    api_key: String,
    provider: String,
    custom_url: Option<String>,
    model: String,
) -> Result<(), String> {
    let timeout_seconds = settings::get_settings().model_timeout_seconds;
    let client = crate::build_http_client(timeout_seconds)?;
    match provider.as_str() {
        "gemini" => crate::transcribe::validate_gemini_api_key(&client, &api_key)
            .await
            .map_err(|e| e.to_string()),
        "dashscope" => crate::transcribe::validate_dashscope_api_key(&client, &api_key, &model)
            .await
            .map_err(|e| e.to_string()),
        "custom" => {
            let url = custom_url.ok_or("Custom URL is required")?;
            crate::transcribe::validate_custom_api_key(&client, &url, Some(&api_key), &model)
                .await
                .map_err(|e| e.to_string())
        }
        _ => crate::transcribe::validate_api_key(&client, &api_key)
            .await
            .map_err(|e| e.to_string()),
    }
}

#[tauri::command]
pub fn pause_shortcut(app: AppHandle) {
    crate::hotkey::pause();
    let settings = settings::get_settings();
    if let Ok(shortcut) = settings.shortcut.parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(shortcut);
    }
    log::info!("Shortcuts paused for capture");
}

#[tauri::command]
pub fn resume_shortcut(app: AppHandle) {
    crate::hotkey::resume();
    let settings = settings::get_settings();
    crate::register_shortcut(&app, &settings);
    log::info!("Shortcuts resumed");
}

#[tauri::command]
pub fn save_overlay_position(app: AppHandle, x: f64, y: f64) {
    let (sx, sy, sw, sh) = crate::cursor_screen_bounds(&app);
    let mut s = settings::get_settings();
    s.overlay_rx = Some(((x - sx) / sw).clamp(0.0, 1.0));
    s.overlay_ry = Some(((y - sy) / sh).clamp(0.0, 1.0));
    settings::save_settings(&s);
}

#[tauri::command]
pub fn initialize_enigo(app: AppHandle) -> Result<(), String> {
    if !crate::paste::is_accessibility_trusted() {
        return Err("Accessibility not granted".into());
    }
    if app.try_state::<EnigoState>().is_some() {
        return Ok(());
    }
    let state = EnigoState::new()?;
    app.manage(state);
    Ok(())
}

#[tauri::command]
pub async fn retry_transcription(
    app: AppHandle,
    history: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<String, String> {
    use crate::transcribe;

    let entry = history
        .get_entry_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or("Entry not found")?;
    let audio_path = entry
        .audio_path
        .as_ref()
        .ok_or("No audio file for this entry")?;

    let wav_data = std::fs::read(audio_path).map_err(|e| e.to_string())?;

    let settings = crate::settings::get_settings();
    let active_provider = settings.active_provider_settings();
    let active_key = active_provider.api_key.clone();
    if active_key.is_empty() && settings.provider != "custom" {
        return Err("API key not configured".into());
    }
    if settings.provider == "custom" && active_provider.api_url.is_empty() {
        return Err("Custom API URL not configured".into());
    }

    let lang = if settings.language == "auto" {
        None
    } else {
        Some(settings.language.as_str())
    };

    let client = crate::build_http_client(settings.model_timeout_seconds)?;

    let result = match settings.provider.as_str() {
        "gemini" => transcribe::transcribe_gemini(
            &client,
            &active_key,
            &active_provider.model,
            wav_data,
            lang,
        )
        .await
        .map_err(|e| e.to_string())?,
        "dashscope" => transcribe::transcribe_dashscope(
            &client,
            &active_key,
            &active_provider.model,
            wav_data,
            lang,
        )
        .await
        .map_err(|e| e.to_string())?,
        "custom" => {
            let api_key = if active_provider.api_key.is_empty() {
                None
            } else {
                Some(active_provider.api_key.as_str())
            };
            transcribe::transcribe_custom(
                &client,
                &active_provider.api_url,
                api_key,
                &active_provider.model,
                wav_data,
                lang,
            )
            .await
            .map_err(|e| e.to_string())?
        }
        _ => transcribe::transcribe_audio(
            &client,
            &active_key,
            &active_provider.model,
            wav_data,
            lang,
        )
        .await
        .map_err(|e| e.to_string())?,
    };

    history
        .update_entry(
            id,
            &result.text,
            &active_provider.model,
            result.input_tokens,
            result.output_tokens,
        )
        .map_err(|e| e.to_string())?;

    let _ = app.clipboard().write_text(&result.text);
    crate::paste::simulate_paste(&app).ok();

    let _ = app.emit("history-updated", ());

    Ok(result.text)
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<Option<String>, String> {
    updater::check_and_download(&app)
        .await
        .map_err(|e| e.to_string())?;
    let state = app.state::<updater::UpdateState>();
    let version = state
        .pending
        .lock()
        .unwrap()
        .as_ref()
        .map(|u| u.version.clone());
    Ok(version)
}

#[tauri::command]
pub fn restart_to_update(
    app: AppHandle,
    recorder: State<'_, Arc<AudioRecorder>>,
) -> Result<(), String> {
    if recorder.is_recording() {
        return Err("Recording in progress".into());
    }
    let state = app.state::<updater::UpdateState>();
    let mut guard = state.pending.lock().unwrap();
    if let Some(p) = guard.as_ref() {
        // Install first; only remove pending data on success
        p.update.install(&p.bytes).map_err(|e| e.to_string())?;
        guard.take();
        drop(guard);
        app.restart();
    }
    Ok(())
}
