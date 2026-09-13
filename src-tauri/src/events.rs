//! Bridges waxum's `/events/tail` SSE stream into Tauri events the frontend
//! listens for (`waxum://message`). One stream runs at a time per app
//! instance; starting a new one cancels whatever was running.

use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

use crate::state::AppState;

pub async fn start(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    base_url: String,
    token: String,
    session_id: String,
) {
    let (stop_tx, stop_rx) = watch::channel(false);
    {
        let mut guard = state.event_stop.lock().await;
        if let Some(old) = guard.take() {
            let _ = old.send(true);
        }
        *guard = Some(stop_tx);
    }
    tokio::spawn(run_stream(app, base_url, token, session_id, stop_rx));
}

pub async fn stop(state: tauri::State<'_, AppState>) {
    let mut guard = state.event_stop.lock().await;
    if let Some(tx) = guard.take() {
        let _ = tx.send(true);
    }
}

async fn run_stream(
    app: AppHandle,
    base_url: String,
    token: String,
    session_id: String,
    stop_rx: watch::Receiver<bool>,
) {
    let mut backoff = std::time::Duration::from_secs(1);
    let mut stopped = WatchStream::new(stop_rx.clone());

    loop {
        if *stop_rx.borrow() {
            return;
        }
        let url = format!(
            "{}/events/tail?session={session_id}&event=message",
            base_url.trim_end_matches('/')
        );
        let client = reqwest::Client::new();
        let req = client.get(&url).bearer_auth(&token).send();

        let resp = tokio::select! {
            r = req => r,
            _ = wait_for_stop(&mut stopped) => return,
        };

        let resp = match resp {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                emit_status(&app, &format!("event stream: HTTP {}", r.status()));
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(std::time::Duration::from_secs(30));
                continue;
            }
            Err(e) => {
                emit_status(&app, &format!("event stream: {e}"));
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(std::time::Duration::from_secs(30));
                continue;
            }
        };

        backoff = std::time::Duration::from_secs(1);
        let mut byte_stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut data_line = String::new();

        loop {
            let next = tokio::select! {
                n = byte_stream.next() => n,
                _ = wait_for_stop(&mut stopped) => return,
            };
            let Some(chunk) = next else { break };
            let Ok(chunk) = chunk else { break };
            buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(idx) = buf.find('\n') {
                let line: String = buf.drain(..=idx).collect();
                let line = line.trim_end_matches(['\r', '\n']);
                if let Some(rest) = line.strip_prefix("data:") {
                    data_line = rest.trim().to_string();
                } else if line.is_empty() && !data_line.is_empty() {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&data_line) {
                        let _ = app.emit("waxum://message", value);
                    }
                    data_line.clear();
                }
            }
        }
    }
}

async fn wait_for_stop(stream: &mut WatchStream<bool>) {
    use futures_util::StreamExt as _;
    while let Some(v) = stream.next().await {
        if v {
            return;
        }
    }
}

fn emit_status(app: &AppHandle, message: &str) {
    let _ = app.emit("waxum://status", message);
}
