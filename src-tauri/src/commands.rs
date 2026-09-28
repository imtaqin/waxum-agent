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
pub async fn waxum_session_messages(
    base_url: String,
    token: String,
    session_id: String,
    limit: u32,
) -> AppResult<serde_json::Value> {
    waxum::session_messages(&base_url, &token, &session_id, limit).await
}

#[tauri::command]
pub async fn waxum_group_info(
    base_url: String,
    token: String,
    session_id: String,
    group_jid: String,
) -> AppResult<serde_json::Value> {
    waxum::group_info(&base_url, &token, &session_id, &group_jid).await
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
pub async fn elevenlabs_mint_realtime_token(api_key: String) -> AppResult<String> {
    elevenlabs::mint_realtime_token(&api_key).await
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

#[derive(serde::Deserialize)]
pub struct AiConverseArgs {
    api_url: String,
    api_key: String,
    model: String,
    transcript: String,
    contacts: Vec<ai::ContactInfo>,
    base_url: String,
    token: String,
    session_id: String,
}

#[tauri::command]
pub async fn ai_converse(
    app: AppHandle,
    state: State<'_, AppState>,
    args: AiConverseArgs,
) -> AppResult<String> {
    let mut history = state.ai_history.lock().await;
    let req = ai::ConverseRequest {
        api_url: args.api_url,
        api_key: args.api_key,
        model: args.model,
        transcript: args.transcript,
        contacts: args.contacts,
        waxum_target: ai::WaxumTarget {
            base_url: args.base_url,
            token: args.token,
            session_id: args.session_id,
        },
    };
    ai::converse(&app, req, &mut history).await
}

#[tauri::command]
pub async fn ai_reset_conversation(state: State<'_, AppState>) -> AppResult<()> {
    state.ai_history.lock().await.clear();
    Ok(())
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

/// Starts (or returns the already running) in-app waxum, downloading it
/// first if needed, and returns its URL and auto-generated token.
#[tauri::command]
pub async fn bundled_launch(
    app: AppHandle,
    state: State<'_, AppState>,
    binary_path: Option<String>,
) -> AppResult<bundled::LaunchInfo> {
    bundled::launch(&app, &state.bundled_child, binary_path).await
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
