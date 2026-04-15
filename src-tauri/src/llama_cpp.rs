use serde::Serialize;
use serde_json::Value;

const TRANSCRIPTION_SUFFIXES: [&str; 2] = ["/v1/audio/transcriptions", "/audio/transcriptions"];

#[derive(Debug, Clone, Serialize)]
pub struct LlamaCppProbeResult {
    pub reachable: bool,
    pub audio_capable: Option<bool>,
    pub detected_model: Option<String>,
    pub warning: Option<String>,
}

impl LlamaCppProbeResult {
    fn unreachable(warning: impl Into<String>) -> Self {
        Self {
            reachable: false,
            audio_capable: None,
            detected_model: None,
            warning: Some(warning.into()),
        }
    }
}

pub fn map_error_message(message: &str) -> String {
    if message.contains("The current model does not support audio input.") {
        return "The current llama.cpp model does not support audio input. Check that you loaded an audio-capable model such as Qwen3-ASR and that the server is configured correctly.".to_string();
    }

    if message.contains("you may need to provide the mmproj") || message.contains("missing mmproj")
    {
        return "The llama.cpp server reported that audio input is unavailable. You may need to start the model with a matching --mmproj file.".to_string();
    }

    message.to_string()
}

pub fn server_root_from_transcription_url(url: &str) -> Result<String, String> {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("llama.cpp endpoint URL is required".to_string());
    }

    let parsed = reqwest::Url::parse(trimmed)
        .map_err(|_| "Invalid llama.cpp endpoint URL".to_string())?;
    let normalized = parsed.to_string().trim_end_matches('/').to_string();

    for suffix in TRANSCRIPTION_SUFFIXES {
        if normalized.ends_with(suffix) {
            let root = normalized.trim_end_matches(suffix).trim_end_matches('/');
            return Ok(root.to_string());
        }
    }

    Err("llama.cpp endpoint must end with /v1/audio/transcriptions or /audio/transcriptions".to_string())
}

pub async fn probe_endpoint(
    client: &reqwest::Client,
    transcription_url: &str,
    api_key: Option<&str>,
) -> LlamaCppProbeResult {
    let root = match server_root_from_transcription_url(transcription_url) {
        Ok(root) => root,
        Err(message) => return LlamaCppProbeResult::unreachable(message),
    };

    let props_url = format!("{}/props", root);
    if let Some(result) = probe_props(client, &props_url, api_key).await {
        return result;
    }

    for models_path in ["/v1/models", "/models"] {
        let models_url = format!("{}{}", root, models_path);
        if let Some(result) = probe_models(client, &models_url, api_key).await {
            return result;
        }
    }

    LlamaCppProbeResult::unreachable(
        "Could not reach llama.cpp /props or /models. Check the endpoint or whether a proxy exposes these routes.",
    )
}

async fn probe_props(
    client: &reqwest::Client,
    url: &str,
    api_key: Option<&str>,
) -> Option<LlamaCppProbeResult> {
    let response = send_request(client, url, api_key).await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let json: Value = response.json().await.ok()?;
    let audio_capable = json
        .get("modalities")
        .and_then(|value| value.get("audio"))
        .and_then(Value::as_bool);
    let detected_model = extract_props_model(&json);
    let warning = match audio_capable {
        Some(true) => None,
        Some(false) => Some(
            "This llama.cpp model does not advertise audio input. It may not be an audio-capable model or may be missing --mmproj."
                .to_string(),
        ),
        None => Some(
            "The endpoint responded, but /props did not expose modalities.audio. The server may not be llama.cpp or a proxy may be hiding /props."
                .to_string(),
        ),
    };

    Some(LlamaCppProbeResult {
        reachable: true,
        audio_capable,
        detected_model,
        warning,
    })
}

async fn probe_models(
    client: &reqwest::Client,
    url: &str,
    api_key: Option<&str>,
) -> Option<LlamaCppProbeResult> {
    let response = send_request(client, url, api_key).await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let json: Value = response.json().await.ok()?;
    let detected_model = json
        .get("data")
        .and_then(Value::as_array)
        .and_then(|models| models.first())
        .and_then(|model| model.get("id"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    Some(LlamaCppProbeResult {
        reachable: true,
        audio_capable: None,
        detected_model,
        warning: Some(
            "The server is reachable, but /props was unavailable so audio capability could not be confirmed.".to_string(),
        ),
    })
}

async fn send_request(
    client: &reqwest::Client,
    url: &str,
    api_key: Option<&str>,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut request = client.get(url);
    if let Some(key) = api_key {
        if !key.is_empty() {
            request = request.bearer_auth(key);
        }
    }
    request.send().await
}

fn extract_props_model(json: &Value) -> Option<String> {
    if let Some(model_alias) = json.get("model_alias").and_then(Value::as_str) {
        if !model_alias.is_empty() {
            return Some(model_alias.to_string());
        }
    }

    json.get("model_path")
        .and_then(Value::as_str)
        .and_then(|path| std::path::Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_server_root_from_transcription_endpoint() {
        assert_eq!(
            server_root_from_transcription_url("http://127.0.0.1:8080/v1/audio/transcriptions")
                .unwrap(),
            "http://127.0.0.1:8080"
        );
        assert_eq!(
            server_root_from_transcription_url(
                "http://127.0.0.1:8080/prefix/v1/audio/transcriptions"
            )
            .unwrap(),
            "http://127.0.0.1:8080/prefix"
        );
    }

    #[test]
    fn maps_audio_capability_errors_to_friendlier_messages() {
        assert!(map_error_message("The current model does not support audio input.")
            .contains("does not support audio input"));
        assert!(map_error_message("audio input is not supported - hint: if this is unexpected, you may need to provide the mmproj")
            .contains("--mmproj"));
    }
}
