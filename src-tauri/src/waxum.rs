//! Thin client for the waxum REST API. Every call takes `base_url`/`token`
//! as plain arguments rather than a stored client — settings live in the
//! frontend's tauri-plugin-store, so there is nothing to keep in sync here.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, AppResult};

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .expect("reqwest client")
}

fn auth(req: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    req.bearer_auth(token)
}

async fn json_or_err(resp: reqwest::Response) -> AppResult<Value> {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::Waxum(format!("HTTP {status}: {text}")));
    }
    serde_json::from_str(&text).map_err(|e| AppError::Waxum(format!("bad JSON: {e}")))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionSummary {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

pub async fn session_status(base_url: &str, token: &str, session_id: &str) -> AppResult<Value> {
    let url = format!("{}/sessions/{session_id}/status", base_url.trim_end_matches('/'));
    let resp = auth(client().get(url), token).send().await?;
    json_or_err(resp).await
}

pub async fn list_sessions(base_url: &str, token: &str) -> AppResult<Vec<SessionSummary>> {
    let url = format!("{}/sessions", base_url.trim_end_matches('/'));
    let resp = auth(client().get(url), token).send().await?;
    let body = json_or_err(resp).await?;
    let sessions = body
        .get("sessions")
        .cloned()
        .unwrap_or(Value::Array(vec![]));
    serde_json::from_value(sessions).map_err(|e| AppError::Waxum(format!("bad session list: {e}")))
}

pub async fn send_text(
    base_url: &str,
    token: &str,
    session_id: &str,
    to: &str,
    text: &str,
) -> AppResult<Value> {
    let url = format!(
        "{}/sessions/{session_id}/messages/text",
        base_url.trim_end_matches('/')
    );
    let resp = auth(client().post(url), token)
        .json(&serde_json::json!({ "to": to, "text": text }))
        .send()
        .await?;
    json_or_err(resp).await
}

/// Recent messages for a chat, used to answer "baca pesan dari X" without
/// waiting for a new one to arrive over the event stream.
pub async fn chat_messages(
    base_url: &str,
    token: &str,
    session_id: &str,
    chat_jid: &str,
    limit: u32,
) -> AppResult<Value> {
    let url = format!(
        "{}/sessions/{session_id}/messages/chat/{chat_jid}?limit={limit}",
        base_url.trim_end_matches('/')
    );
    let resp = auth(client().get(url), token).send().await?;
    json_or_err(resp).await
}

/// Session-wide message history, newest first — used to prime the chat
/// log/known-contacts map on connect instead of starting blank and only
/// ever seeing messages that arrive after the app happens to be open.
pub async fn session_messages(
    base_url: &str,
    token: &str,
    session_id: &str,
    limit: u32,
) -> AppResult<Value> {
    let url = format!(
        "{}/sessions/{session_id}/messages?limit={limit}",
        base_url.trim_end_matches('/')
    );
    let resp = auth(client().get(url), token).send().await?;
    json_or_err(resp).await
}

/// Group subject (display name), used so a group's context/log entries are
/// keyed by "the group" rather than whichever member happened to send the
/// most recent message.
pub async fn group_info(
    base_url: &str,
    token: &str,
    session_id: &str,
    group_jid: &str,
) -> AppResult<Value> {
    let url = format!(
        "{}/sessions/{session_id}/groups/{}",
        base_url.trim_end_matches('/'),
        urlencoding_lite(group_jid)
    );
    let resp = auth(client().get(url), token).send().await?;
    json_or_err(resp).await
}

pub async fn search_contact(
    base_url: &str,
    token: &str,
    session_id: &str,
    query: &str,
) -> AppResult<Value> {
    let url = format!(
        "{}/sessions/{session_id}/messages/search?q={}",
        base_url.trim_end_matches('/'),
        urlencoding_lite(query)
    );
    let resp = auth(client().get(url), token).send().await?;
    json_or_err(resp).await
}

/// No `url` crate dependency for one call site — percent-encode the handful
/// of characters that break a query string.
fn urlencoding_lite(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
