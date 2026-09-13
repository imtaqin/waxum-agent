//! ElevenLabs TTS (read incoming messages aloud) and STT (voice commands).
//! Same pattern as `waxum.rs`: stateless functions, the API key lives in the
//! frontend's store and is passed in per call.

use base64::Engine;

use crate::error::{AppError, AppResult};

const API_BASE: &str = "https://api.elevenlabs.io/v1";

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("reqwest client")
}

/// Synthesizes `text` with the given voice and returns base64-encoded MP3 —
/// base64 because Tauri's IPC serializes command results as JSON, and raw
/// bytes would otherwise have to go through a byte-array round trip.
pub async fn speak(api_key: &str, voice_id: &str, text: &str) -> AppResult<String> {
    let url = format!("{API_BASE}/text-to-speech/{voice_id}");
    let resp = client()
        .post(url)
        .header("xi-api-key", api_key)
        .json(&serde_json::json!({
            "text": text,
            "model_id": "eleven_multilingual_v2",
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::ElevenLabs(format!("TTS HTTP {status}: {body}")));
    }
    let bytes = resp.bytes().await?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// Transcribes recorded audio (base64-encoded, any format the browser's
/// MediaRecorder produced — webm/opus in practice) via ElevenLabs Scribe.
pub async fn transcribe(api_key: &str, audio_base64: &str, mime_type: &str) -> AppResult<String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(audio_base64)
        .map_err(|e| AppError::ElevenLabs(format!("bad audio payload: {e}")))?;

    let filename = match mime_type {
        m if m.contains("webm") => "clip.webm",
        m if m.contains("wav") => "clip.wav",
        m if m.contains("mp4") => "clip.mp4",
        _ => "clip.ogg",
    };
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(mime_type)
        .map_err(|e| AppError::ElevenLabs(format!("bad mime type: {e}")))?;
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model_id", "scribe_v1");

    let resp = client()
        .post(format!("{API_BASE}/speech-to-text"))
        .header("xi-api-key", api_key)
        .multipart(form)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::ElevenLabs(format!("STT HTTP {status}: {body}")));
    }
    let value: serde_json::Value = resp.json().await?;
    value
        .get("text")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| AppError::ElevenLabs("STT response missing text field".into()))
}

/// Verifies the key works and returns the raw subscription payload, used by
/// the settings screen's "Test connection" button.
pub async fn check_subscription(api_key: &str) -> AppResult<serde_json::Value> {
    let resp = client()
        .get(format!("{API_BASE}/user/subscription"))
        .header("xi-api-key", api_key)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        return Err(AppError::ElevenLabs(format!("HTTP {status}")));
    }
    Ok(resp.json().await?)
}
