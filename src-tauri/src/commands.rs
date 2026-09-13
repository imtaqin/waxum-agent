//! Every `#[tauri::command]` the frontend calls. Kept flat and thin —
//! real logic lives in `waxum.rs` / `elevenlabs.rs` / `bundled.rs` / `events.rs`.

use tauri::{AppHandle, State};

use crate::error::AppResult;
use crate::state::AppState;
use crate::{ai, bundled, download, elevenlabs, events, pairing, waxum};

#[tauri::command]
pub async fn waxum_status(
    base_url: String,
    token: String,
    session_id: String,
) -> AppResult<serde_json::Value> {
    waxum::session_status(&base_url, &token, &session_id).await
}

#[tauri::command]
pub async fn waxum_list_sessions(
    base_url: String,
    token: String,
) -> AppResult<Vec<waxum::SessionSummary>> {
    waxum::list_sessions(&base_url, &token).await
}

#[tauri::command]
pub async fn waxum_send_text(
    base_url: String,
    token: String,
    session_id: String,
    to: String,
    text: String,
) -> AppResult<serde_json::Value> {
    waxum::send_text(&base_url, &token, &session_id, &to, &text).await
}

#[tauri::command]
pub async fn waxum_chat_messages(
    base_url: String,
    token: String,
    session_id: String,
    chat_jid: String,
    limit: u32,
) -> AppResult<serde_json::Value> {
    waxum::chat_messages(&base_url, &token, &session_id, &chat_jid, limit).await
}

#[tauri::command]
pub async fn waxum_search(
    base_url: String,
    token: String,
    session_id: String,
    query: String,
) -> AppResult<serde_json::Value> {
    waxum::search_contact(&base_url, &token, &session_id, &query).await
}

#[tauri::command]
pub async fn waxum_start_events(
    app: AppHandle,
    state: State<'_, AppState>,
    base_url: String,
    token: String,
    session_id: String,
) -> AppResult<()> {
    events::start(app, state, base_url, token, session_id).await;
    Ok(())
}

#[tauri::command]
pub async fn waxum_stop_events(state: State<'_, AppState>) -> AppResult<()> {
    events::stop(state).await;
    Ok(())
}

#[tauri::command]
pub async fn tts_speak(api_key: String, voice_id: String, text: String) -> AppResult<String> {
    elevenlabs::speak(&api_key, &voice_id, &text).await
}

#[tauri::command]
pub async fn stt_transcribe(
    api_key: String,
    audio_base64: String,
    mime_type: String,
) -> AppResult<String> {
    elevenlabs::transcribe(&api_key, &audio_base64, &mime_type).await
}

#[tauri::command]
pub async fn elevenlabs_check(api_key: String) -> AppResult<serde_json::Value> {
    elevenlabs::check_subscription(&api_key).await
}

#[tauri::command]
pub async fn ai_interpret(
    api_url: String,
    api_key: String,
    model: String,
    transcript: String,
    context: String,
) -> AppResult<serde_json::Value> {
    ai::interpret(&api_url, &api_key, &model, &transcript, &context).await
}

/// Starts the bundled waxum process. When `binary_path` is empty, auto
/// downloads the latest release for this OS/arch first — "bundled mode"
/// should not require the user to go find a binary themselves.
#[tauri::command]
pub async fn waxum_start_pairing(
    app: AppHandle,
    state: State<'_, AppState>,
    base_url: String,
    token: String,
    session_id: String,
    timeout_seconds: u64,
) -> AppResult<()> {
    pairing::start(app, state, base_url, token, session_id, timeout_seconds).await;
    Ok(())
}

#[tauri::command]
pub async fn waxum_stop_pairing(state: State<'_, AppState>) -> AppResult<()> {
    pairing::stop(state).await;
    Ok(())
}

#[tauri::command]
pub async fn bundled_start(
    app: AppHandle,
    state: State<'_, AppState>,
    binary_path: String,
    port: u16,
    env: Vec<(String, String)>,
) -> AppResult<String> {
    let resolved = if binary_path.trim().is_empty() {
        download::ensure_binary(&app).await?
    } else {
        binary_path
    };
    bundled::start(&resolved, port, env, &state.bundled_child).await?;
    Ok(resolved)
}

#[tauri::command]
pub async fn bundled_ensure_binary(app: AppHandle) -> AppResult<String> {
    download::ensure_binary(&app).await
}

#[tauri::command]
pub async fn bundled_update_binary(app: AppHandle) -> AppResult<String> {
    download::download_latest_forced(&app).await
}

#[tauri::command]
pub async fn bundled_stop(state: State<'_, AppState>) -> AppResult<()> {
    bundled::stop(&state.bundled_child).await
}

#[tauri::command]
pub async fn bundled_is_running(state: State<'_, AppState>) -> AppResult<bool> {
    Ok(bundled::is_running(&state.bundled_child).await)
}
