mod ai;
mod bundled;
mod commands;
mod download;
mod elevenlabs;
mod error;
mod events;
mod pairing;
mod state;
mod waxum;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::waxum_status,
            commands::waxum_list_sessions,
            commands::waxum_send_text,
            commands::waxum_chat_messages,
            commands::waxum_search,
            commands::waxum_start_events,
            commands::waxum_stop_events,
            commands::waxum_start_pairing,
            commands::waxum_stop_pairing,
            commands::tts_speak,
            commands::stt_transcribe,
            commands::elevenlabs_check,
            commands::ai_interpret,
            commands::bundled_start,
            commands::bundled_stop,
            commands::bundled_is_running,
            commands::bundled_ensure_binary,
            commands::bundled_update_binary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
