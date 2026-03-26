use anyhow::{Context, Result};
use base64::Engine;
use reqwest::multipart;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

/// Validate API key by sending a tiny silent WAV to the transcription endpoint.
pub async fn validate_api_key(client: &reqwest::Client, api_key: &str) -> Result<()> {
    let wav = generate_silent_wav();
    let file_part = multipart::Part::bytes(wav)
        .file_name("test.wav")
        .mime_str("audio/wav")?;
    let form = multipart::Form::new()
        .part("file", file_part)
        .text("model", "whisper-1".to_string());

    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .context("Network error")?;

    if resp.status() == 401 {
        anyhow::bail!("Invalid API key");
    }
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("{}", body);
    }
    Ok(())
}

pub async fn validate_custom_api_key(client: &reqwest::Client, url: &str, api_key: Option<&str>) -> Result<()> {
    let wav = generate_silent_wav();
    let file_part = multipart::Part::bytes(wav)
        .file_name("test.wav")
        .mime_str("audio/wav")?;
    let form = multipart::Form::new()
        .part("file", file_part)
        .text("model", "test".to_string());

    let mut req = client.post(url);
    if let Some(key) = api_key {
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }
    }
    
    let resp = req
        .multipart(form)
        .send()
        .await
        .context("Network error")?;

    let status = resp.status();
    if status.as_u16() == 401 {
        anyhow::bail!("Invalid API key");
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("API error {}: {}", status, body);
    }
    Ok(())
}

/// Generate a minimal valid WAV file (0.5s silence, 16kHz mono 16-bit).
fn generate_silent_wav() -> Vec<u8> {
    let sample_rate: u32 = 16000;
    let num_samples: u32 = sample_rate / 2; // 0.5 seconds
    let data_size = num_samples * 2; // 16-bit = 2 bytes per sample
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(file_size as usize + 8);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVEfmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    buf.resize(buf.len() + data_size as usize, 0); // silence
    buf
}

pub async fn transcribe_audio(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    wav_data: Vec<u8>,
    language: Option<&str>,
) -> Result<TranscriptionResult> {
    let file_part = multipart::Part::bytes(wav_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;

    let mut form = multipart::Form::new()
        .part("file", file_part)
        .text("model", model.to_string());

    if let Some(lang) = language {
        if lang != "auto" {
            form = form.text("language", lang.to_string());
        }
    }

    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .context("Failed to send transcription request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("API error {}: {}", status, body);
    }

    let json: serde_json::Value = resp.json().await.context("Failed to parse API response")?;
    let text = json["text"]
        .as_str()
        .context("Missing 'text' field in response")?
        .to_string();

    let input_tokens = json["usage"]["input_tokens"].as_i64();
    let output_tokens = json["usage"]["output_tokens"].as_i64();

    Ok(TranscriptionResult {
        text,
        input_tokens,
        output_tokens,
    })
}

pub async fn transcribe_custom(
    client: &reqwest::Client,
    url: &str,
    api_key: Option<&str>,
    model: &str,
    wav_data: Vec<u8>,
    language: Option<&str>,
) -> Result<TranscriptionResult> {
    let file_part = multipart::Part::bytes(wav_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;

    let mut form = multipart::Form::new()
        .part("file", file_part)
        .text("model", model.to_string());

    if let Some(lang) = language {
        if lang != "auto" {
            form = form.text("language", lang.to_string());
        }
    }

    let mut req = client.post(url);
    if let Some(key) = api_key {
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }
    }

    let resp = req
        .multipart(form)
        .send()
        .await
        .context("Failed to send transcription request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("API error {}: {}", status, body);
    }

    let json: serde_json::Value = resp.json().await.context("Failed to parse API response")?;
    let text = json["text"]
        .as_str()
        .context("Missing 'text' field in response")?
        .to_string();

    let input_tokens = json["usage"]["input_tokens"].as_i64()
        .or_else(|| json["usage"]["prompt_tokens"].as_i64());
    let output_tokens = json["usage"]["output_tokens"].as_i64()
        .or_else(|| json["usage"]["completion_tokens"].as_i64());

    Ok(TranscriptionResult {
        text,
        input_tokens,
        output_tokens,
    })
}

// ── Google Gemini (generateContent) ─────────────────────────────────

const GEMINI_INLINE_LIMIT: usize = 20 * 1024 * 1024; // 20 MB

/// Validate Gemini API key by sending a tiny silent WAV.
pub async fn validate_gemini_api_key(client: &reqwest::Client, api_key: &str) -> Result<()> {
    let wav = generate_silent_wav();
    let b64 = base64::engine::general_purpose::STANDARD.encode(&wav);

    let body = serde_json::json!({
        "contents": [{
            "parts": [
                {"text": "Transcribe the following audio. Output only the transcribed text, nothing else."},
                {"inline_data": {"mime_type": "audio/wav", "data": b64}}
            ]
        }]
    });

    let resp = client
        .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-3-flash-preview:generateContent")
        .header("x-goog-api-key", api_key)
        .json(&body)
        .send()
        .await
        .context("Network error")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        if body.contains("API_KEY_INVALID") || body.contains("PERMISSION_DENIED") || status.as_u16() == 401 {
            anyhow::bail!("Invalid API key");
        }
        anyhow::bail!("Gemini API error {}: {}", status, body);
    }
    Ok(())
}

pub async fn transcribe_gemini(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    wav_data: Vec<u8>,
    language: Option<&str>,
) -> Result<TranscriptionResult> {
    // Check inline size limit (base64 expands ~33%)
    let estimated_b64_size = (wav_data.len() * 4 + 2) / 3;
    if estimated_b64_size > GEMINI_INLINE_LIMIT {
        anyhow::bail!(
            "Audio file too large for Gemini ({:.1} MB). Maximum is ~15 MB WAV.",
            wav_data.len() as f64 / 1_048_576.0
        );
    }

    let b64 = base64::engine::general_purpose::STANDARD.encode(&wav_data);

    let prompt = match language {
        Some(lang) if lang != "auto" => {
            let lang_name = language_code_to_name(lang);
            format!(
                "Transcribe the following audio. Output only the transcribed text, nothing else. The language is {}.",
                lang_name
            )
        }
        _ => "Transcribe the following audio. Output only the transcribed text, nothing else.".to_string(),
    };

    let body = serde_json::json!({
        "contents": [{
            "parts": [
                {"text": prompt},
                {"inline_data": {"mime_type": "audio/wav", "data": b64}}
            ]
        }]
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model
    );

    let resp = client
        .post(&url)
        .header("x-goog-api-key", api_key)
        .json(&body)
        .send()
        .await
        .context("Failed to send transcription request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Gemini API error {}: {}", status, body);
    }

    let json: serde_json::Value = resp.json().await.context("Failed to parse API response")?;

    // Check for content filtering / prompt block
    if let Some(reason) = json["promptFeedback"]["blockReason"].as_str() {
        anyhow::bail!("Gemini blocked the request: {}", reason);
    }

    // Concatenate all text parts from the first candidate
    let text = json["candidates"][0]["content"]["parts"]
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    if text.is_empty() {
        anyhow::bail!("Gemini returned empty transcription");
    }

    let input_tokens = json["usageMetadata"]["promptTokenCount"].as_i64();
    let output_tokens = json["usageMetadata"]["candidatesTokenCount"].as_i64();

    Ok(TranscriptionResult {
        text: text.trim().to_string(),
        input_tokens,
        output_tokens,
    })
}

fn language_code_to_name(code: &str) -> &str {
    match code {
        "zh" => "Chinese",
        "en" => "English",
        "ja" => "Japanese",
        "ko" => "Korean",
        "es" => "Spanish",
        "fr" => "French",
        "de" => "German",
        _ => code,
    }
}

// ── DashScope (Qwen-ASR / Fun-ASR / Paraformer) ─────────────────────────

const DASHSCOPE_BASE64_LIMIT: usize = 10 * 1024 * 1024;

pub async fn validate_dashscope_api_key(client: &reqwest::Client, api_key: &str) -> Result<()> {
    let wav = generate_silent_wav();
    let b64 = base64::engine::general_purpose::STANDARD.encode(&wav);
    let data_uri = format!("data:audio/wav;base64,{}", b64);

    let body = serde_json::json!({
        "model": "qwen3-asr-flash",
        "messages": [{
            "role": "user",
            "content": [{
                "type": "input_audio",
                "input_audio": { "data": data_uri }
            }]
        }]
    });

    let resp = client
        .post("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .context("Network error")?;

    let status = resp.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        anyhow::bail!("Invalid API key");
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        if body.contains("invalid_api_key") || body.contains("InvalidApiKey") {
            anyhow::bail!("Invalid API key");
        }
        anyhow::bail!("DashScope API error {}: {}", status, body);
    }
    Ok(())
}

pub async fn transcribe_dashscope(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    wav_data: Vec<u8>,
    language: Option<&str>,
) -> Result<TranscriptionResult> {
    let estimated_b64_size = (wav_data.len() * 4 + 2) / 3;
    if estimated_b64_size > DASHSCOPE_BASE64_LIMIT {
        anyhow::bail!(
            "Audio file too large for DashScope ({:.1} MB). Maximum is ~7.5 MB WAV.",
            wav_data.len() as f64 / 1_048_576.0
        );
    }

    let b64 = base64::engine::general_purpose::STANDARD.encode(&wav_data);
    let data_uri = format!("data:audio/wav;base64,{}", b64);

    let mut body = serde_json::json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "input_audio",
                "input_audio": { "data": data_uri }
            }]
        }]
    });

    if let Some(lang) = language {
        if lang != "auto" {
            if let Some(dashscope_lang) = to_dashscope_language(lang) {
                body["asr_options"] = serde_json::json!({ "language": dashscope_lang });
            }
        }
    }

    let resp = client
        .post("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .context("Failed to send transcription request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("DashScope API error {}: {}", status, body);
    }

    let json: serde_json::Value = resp.json().await.context("Failed to parse API response")?;
    let text = json["choices"][0]["message"]["content"]
        .as_str()
        .context("Missing 'content' in response")?
        .to_string();

    if text.is_empty() {
        anyhow::bail!("DashScope returned empty transcription");
    }

    let input_tokens = json["usage"]["prompt_tokens"].as_i64();
    let output_tokens = json["usage"]["completion_tokens"].as_i64();

    Ok(TranscriptionResult {
        text: text.trim().to_string(),
        input_tokens,
        output_tokens,
    })
}

fn to_dashscope_language(code: &str) -> Option<&'static str> {
    match code {
        "zh" => Some("zh"),
        "yue" => Some("yue"),
        "en" => Some("en"),
        "ja" => Some("ja"),
        "ko" => Some("ko"),
        "de" => Some("de"),
        "fr" => Some("fr"),
        "ru" => Some("ru"),
        "pt" => Some("pt"),
        "ar" => Some("ar"),
        "it" => Some("it"),
        "es" => Some("es"),
        "hi" => Some("hi"),
        "id" => Some("id"),
        "th" => Some("th"),
        "tr" => Some("tr"),
        "vi" => Some("vi"),
        _ => None,
    }
}
