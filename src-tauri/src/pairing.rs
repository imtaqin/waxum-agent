//! Drives waxum's one-shot `/sessions/{id}/connect/wait` endpoint: creates
//! the session (with a caller-supplied, locally-generated id — never
//! listed from the server) and streams qr_code/pair_code/connected/ready
//! events to the frontend so it can render a QR without a second round
//! trip to fetch the code separately.

use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;

use crate::state::AppState;

pub async fn start(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    base_url: String,
    token: String,
    session_id: String,
    timeout_seconds: u64,
) {
    let (stop_tx, stop_rx) = watch::channel(false);
    {
        let mut guard = state.pairing_stop.lock().await;
        if let Some(old) = guard.take() {
            let _ = old.send(true);
        }
        *guard = Some(stop_tx);
    }
    tokio::spawn(run(app, base_url, token, session_id, timeout_seconds, stop_rx));
}

pub async fn stop(state: tauri::State<'_, AppState>) {
    let mut guard = state.pairing_stop.lock().await;
    if let Some(tx) = guard.take() {
        let _ = tx.send(true);
    }
}

async fn run(
    app: AppHandle,
    base_url: String,
    token: String,
    session_id: String,
    timeout_seconds: u64,
    mut stop_rx: watch::Receiver<bool>,
) {
    let url = format!(
        "{}/sessions/{session_id}/connect/wait?timeout_seconds={timeout_seconds}",
        base_url.trim_end_matches('/')
    );
    let client = reqwest::Client::new();
    let req = client.get(&url).bearer_auth(&token).send();

    let resp = tokio::select! {
        r = req => r,
        _ = stop_rx.changed() => return,
    };

    let resp = match resp {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            emit(&app, "error", &format!(r#"{{"message":"HTTP {}"}}"#, r.status()));
            return;
        }
        Err(e) => {
            emit(&app, "error", &format!(r#"{{"message":"{e}"}}"#));
            return;
        }
    };

    let mut byte_stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut event_name = String::new();
    let mut data_line = String::new();

    loop {
        let next = tokio::select! {
            n = byte_stream.next() => n,
            _ = stop_rx.changed() => return,
        };
        let Some(Ok(chunk)) = next else { return };
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(idx) = buf.find('\n') {
            let line: String = buf.drain(..=idx).collect();
            let line = line.trim_end_matches(['\r', '\n']);
            if let Some(rest) = line.strip_prefix("event:") {
                event_name = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("data:") {
                data_line = rest.trim().to_string();
            } else if line.is_empty() && !event_name.is_empty() {
                emit(&app, &event_name, &data_line);
                let terminal = matches!(event_name.as_str(), "ready" | "error" | "timeout");
                event_name.clear();
                data_line.clear();
                if terminal {
                    return;
                }
            }
        }
    }
}

fn emit(app: &AppHandle, event: &str, data: &str) {
    let value: serde_json::Value =
        serde_json::from_str(data).unwrap_or_else(|_| serde_json::json!({}));
    let _ = app.emit(
        "waxum-agent://pairing",
        serde_json::json!({ "event": event, "data": value }),
    );
}
