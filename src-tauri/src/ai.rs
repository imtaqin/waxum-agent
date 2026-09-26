//! Free-form natural-language understanding, OpenAI-compatible. Falls
//! back from `commandParser.ts`'s fixed regex patterns for anything that
//! doesn't match one of them — "halo ada pesan apa aja" has no fixed
//! shape, so it needs an actual model in the loop, not more regex.
//!
//! `converse()` is the TARS/Siri-style path: real tool-calling
//! (`search_contact`, `send_message`) instead of a client-side substring
//! match that dead-ends whenever a heard name doesn't literally overlap a
//! stored one ("Randy" vs a contact actually named "Rendi"), plus a
//! streamed final reply for lower perceived latency, plus a rolling
//! conversation history (kept by the caller across calls) so a follow-up
//! like "kamu balas apa barusan?" is answerable instead of stateless.

use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};

use crate::error::{AppError, AppResult};
use crate::waxum;

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("reqwest client")
}

/// Asks the model to decide whether the transcript wants a WhatsApp
/// message sent, and to produce a short spoken reply either way. `context`
/// is a plain-text summary of known chats/last messages the frontend
/// already has, so the model can answer "ada pesan apa aja" without a
/// second round trip.
///
/// Kept as-is (one-shot, no tools) for the settings screen's "Test AI
/// connection" button — `converse()` below is the real conversational path.
pub async fn interpret(
    api_url: &str,
    api_key: &str,
    model: &str,
    transcript: &str,
    context: &str,
) -> AppResult<serde_json::Value> {
    let system = format!(
        "Kamu adalah Waxum Agent, asisten eksekutif profesional untuk WhatsApp. Jawab selalu \
         dalam Bahasa Indonesia yang formal, presisi, dan ringkas -- gaya seperti asisten AI \
         korporat kelas atas: sopan, tegas, tanpa basa-basi, tanpa emoji, tanpa singkatan gaul.\n\n\
         Konteks percakapan yang diketahui (riwayat beberapa pesan terakhir per kontak/grup):\n{context}\n\n\
         Balas HANYA dengan satu objek JSON, tanpa markdown, tanpa teks lain, persis bentuk ini:\n\
         {{\"action\": \"send_message\" atau \"none\", \"to\": \"<nama kontak dari konteks di atas, atau string kosong>\", \"text\": \"<isi pesan yang akan dikirim, atau string kosong>\", \"reply\": \"<balasan lisan singkat, selalu diisi>\"}}\n\n\
         Set action ke \"send_message\" hanya jika pengguna secara eksplisit minta mengirim/membalas pesan ke seseorang yang ada di konteks. \
         Untuk pertanyaan lain (misal menanyakan pesan apa saja yang masuk), gunakan action \"none\" dan jawab langsung di field reply berdasarkan konteks yang diberikan."
    );

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": transcript},
        ],
        "temperature": 0.3,
    });

    let resp = client()
        .post(format!(
            "{}/chat/completions",
            api_url.trim_end_matches('/')
        ))
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Ai(format!("HTTP {status}: {text}")));
    }

    let value: serde_json::Value = resp.json().await?;
    let content = value["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| AppError::Ai("no content in completion response".into()))?;

    parse_json_content(content)
}

/// Models routinely wrap JSON in a ```json fence despite being told not
/// to -- strip that before parsing instead of rejecting a valid response
/// over formatting.
fn parse_json_content(content: &str) -> AppResult<serde_json::Value> {
    let trimmed = content.trim();
    let stripped = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .trim_end_matches("```")
        .trim();
    serde_json::from_str(stripped).map_err(|e| {
        AppError::Ai(format!(
            "model did not return valid JSON: {e} -- got: {stripped}"
        ))
    })
}

/// One contact/chat the frontend already knows about (from `knownChats`),
/// handed in as tool-search data rather than flattened into a text blob --
/// the model gets to query it, not just read a dump of it once up front.
#[derive(serde::Deserialize, Clone)]
pub struct ContactInfo {
    pub name: String,
    pub jid: String,
    #[serde(default)]
    pub recent: Vec<String>,
}

/// waxum connection details, needed here because `send_message` is
/// executed server-side (Rust) as soon as the model calls the tool,
/// instead of being handed back to the frontend as a decision to re-parse.
#[derive(serde::Deserialize, Clone)]
pub struct WaxumTarget {
    pub base_url: String,
    pub token: String,
    pub session_id: String,
}

const MAX_TOOL_ITERATIONS: usize = 5;
const MAX_HISTORY_MESSAGES: usize = 24;

fn system_message() -> serde_json::Value {
    serde_json::json!({
        "role": "system",
        "content": "Kamu adalah Waxum Agent, asisten eksekutif profesional -- karakternya presisi dan andal \
            seperti TARS di Interstellar, disiplin seperti kepala staf senior. Bicara dalam Bahasa Indonesia \
            yang formal, jelas, dan padat: setiap kalimat harus berisi, tanpa basa-basi, tanpa emoji, tanpa \
            singkatan gaul atau candaan yang tidak perlu. Nada tetap hangat dan sopan, bukan kaku atau dingin \
            seperti mesin.\n\n\
            Ini adalah percakapan yang berkelanjutan: kamu mengingat apa yang baru terjadi di sesi ini, termasuk \
            apa yang baru kamu balas atau kamu lakukan sendiri -- kalau pengguna menyinggung hal itu \
            (\"kamu balas apa barusan?\"), jawab berdasarkan riwayat percakapan, jangan berpura-pura tidak tahu.\n\n\
            Untuk pertanyaan yang menyangkut isi pesan lampau (bukan hanya percakapan yang sedang aktif di \
            layar), panggil tool search_messages dengan kata kunci yang relevan alih-alih menjawab dari ingatan \
            atau menebak -- data itu tersimpan di server, bukan di kepalamu.\n\n\
            Untuk kontak WhatsApp: jangan pernah menebak atau mencocokkan nama kontak secara manual sendiri. \
            Transkrip suara sering meleset (\"Randy\" padahal kontak aslinya \"Rendi\"). Selalu panggil tool \
            search_contact dengan nama yang kamu dengar, evaluasi kandidat yang dikembalikan secara cermat, lalu \
            gunakan JID dari kandidat yang paling sesuai untuk memanggil send_message. Bila tidak ada kandidat \
            yang meyakinkan, sampaikan hal itu secara langsung kepada pengguna dan minta klarifikasi -- jangan \
            pernah mengirim pesan ke kontak yang salah."
    })
}

fn tools_schema() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "search_contact",
                "description": "Cari kontak/percakapan WhatsApp yang diketahui berdasarkan nama yang didengar dari ucapan (boleh typo atau salah dengar). Mengembalikan kandidat kontak beserta JID dan beberapa pesan terakhirnya, diurutkan dari yang paling cocok.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Nama kontak yang didengar dari ucapan pengguna" }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_messages",
                "description": "Cari isi pesan WhatsApp berdasarkan kata kunci di seluruh riwayat percakapan yang tersimpan di server (bukan hanya percakapan yang sedang aktif di layar). Gunakan ini untuk pertanyaan seperti \"pesan soal invoice dari siapa\" atau \"cari chat yang menyebut deadline\".",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Kata kunci yang dicari di isi pesan" }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "send_message",
                "description": "Kirim pesan WhatsApp. `jid` HARUS berasal dari hasil search_contact -- jangan pernah menebak atau menyusun JID sendiri.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "jid": { "type": "string" },
                        "text": { "type": "string" }
                    },
                    "required": ["jid", "text"]
                }
            }
        }
    ])
}

/// Plain Levenshtein edit distance over chars -- enough for "Randy" vs
/// "Rendi"-shaped mis-hearings without pulling in a crate for it.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (la, lb) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; lb + 1]; la + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=la {
        for j in 1..=lb {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1])
            };
        }
    }
    dp[la][lb]
}

fn score_contact(query: &str, name: &str) -> i32 {
    let q = query.trim().to_lowercase();
    let n = name.trim().to_lowercase();
    if q.is_empty() || n.is_empty() {
        return 0;
    }
    if q == n {
        return 100;
    }
    if n.contains(&q) || q.contains(&n) {
        return 85;
    }
    let dist = levenshtein(&q, &n);
    let longest = q.chars().count().max(n.chars().count()).max(1);
    100 - ((dist * 100) / longest) as i32
}

fn run_search_contact(query: &str, contacts: &[ContactInfo]) -> serde_json::Value {
    let mut scored: Vec<(i32, &ContactInfo)> = contacts
        .iter()
        .map(|c| (score_contact(query, &c.name), c))
        .filter(|(score, _)| *score >= 35)
        .collect();
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    let candidates: Vec<serde_json::Value> = scored
        .into_iter()
        .take(5)
        .map(|(score, c)| {
            serde_json::json!({
                "name": c.name,
                "jid": c.jid,
                "match_score": score,
                "recent_messages": c.recent,
            })
        })
        .collect();
    if candidates.is_empty() {
        serde_json::json!({ "candidates": [], "note": "tidak ada kontak yang cocok ditemukan" })
    } else {
        serde_json::json!({ "candidates": candidates })
    }
}

async fn run_search_messages(query: &str, target: &WaxumTarget) -> serde_json::Value {
    match waxum::search_contact(&target.base_url, &target.token, &target.session_id, query).await {
        Ok(results) => serde_json::json!({ "results": results }),
        Err(e) => serde_json::json!({ "ok": false, "error": e.to_string() }),
    }
}

async fn run_send_message(jid: &str, text: &str, target: &WaxumTarget) -> serde_json::Value {
    match waxum::send_text(
        &target.base_url,
        &target.token,
        &target.session_id,
        jid,
        text,
    )
    .await
    {
        Ok(_) => serde_json::json!({ "ok": true }),
        Err(e) => serde_json::json!({ "ok": false, "error": e.to_string() }),
    }
}

struct PendingToolCall {
    id: String,
    name: String,
    arguments: String,
}

struct StreamOutcome {
    content: String,
    tool_calls: Vec<PendingToolCall>,
}

/// Streams one chat-completion request, forwarding content deltas to the
/// frontend live (`ai-stream` events) as they arrive, and buffering any
/// tool-call deltas (which come as fragments across many chunks, per the
/// OpenAI streaming contract) until the stream ends.
async fn stream_completion(
    app: &AppHandle,
    api_url: &str,
    api_key: &str,
    model: &str,
    messages: &[serde_json::Value],
    tools: Option<&serde_json::Value>,
) -> AppResult<StreamOutcome> {
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "temperature": 0.4,
        "stream": true,
    });
    if let Some(t) = tools {
        body["tools"] = t.clone();
    }

    let resp = client()
        .post(format!(
            "{}/chat/completions",
            api_url.trim_end_matches('/')
        ))
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Ai(format!("HTTP {status}: {text}")));
    }

    let mut content = String::new();
    let mut tool_calls: Vec<PendingToolCall> = Vec::new();
    let mut buf = String::new();
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find("\n\n") {
            let frame = buf[..pos].to_string();
            buf.drain(..pos + 2);

            for line in frame.lines() {
                let Some(data) = line
                    .strip_prefix("data: ")
                    .or_else(|| line.strip_prefix("data:"))
                else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    continue;
                }
                let Ok(value) = serde_json::from_str::<serde_json::Value>(data) else {
                    continue;
                };
                let delta = &value["choices"][0]["delta"];
                if let Some(piece) = delta["content"].as_str() {
                    if !piece.is_empty() {
                        content.push_str(piece);
                        let _ = app.emit("waxum-agent://ai-stream", piece);
                    }
                }
                if let Some(calls) = delta["tool_calls"].as_array() {
                    for call in calls {
                        let index = call["index"].as_u64().unwrap_or(0) as usize;
                        while tool_calls.len() <= index {
                            tool_calls.push(PendingToolCall {
                                id: String::new(),
                                name: String::new(),
                                arguments: String::new(),
                            });
                        }
                        let slot = &mut tool_calls[index];
                        if let Some(id) = call["id"].as_str() {
                            slot.id = id.to_string();
                        }
                        if let Some(name) = call["function"]["name"].as_str() {
                            slot.name = name.to_string();
                        }
                        if let Some(args) = call["function"]["arguments"].as_str() {
                            slot.arguments.push_str(args);
                        }
                    }
                }
            }
        }
    }

    Ok(StreamOutcome {
        content,
        tool_calls,
    })
}

/// Everything one `converse()` call needs beyond the running history --
/// bundled so the function stays under clippy's argument-count limit.
pub struct ConverseRequest {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    pub transcript: String,
    pub contacts: Vec<ContactInfo>,
    pub waxum_target: WaxumTarget,
}

/// The TARS/Siri path: multi-turn tool-calling loop over a streamed
/// chat-completion, backed by a rolling `history` the caller keeps across
/// calls (so "kamu balas apa barusan?" is answerable). Returns the final
/// spoken reply -- also already streamed to the frontend chunk-by-chunk
/// via `ai-stream`/`ai-tool` events as it was produced.
pub async fn converse(
    app: &AppHandle,
    req: ConverseRequest,
    history: &mut Vec<serde_json::Value>,
) -> AppResult<String> {
    let ConverseRequest {
        api_url,
        api_key,
        model,
        transcript,
        contacts,
        waxum_target,
    } = req;

    if history.is_empty() {
        history.push(system_message());
    }
    history.push(serde_json::json!({ "role": "user", "content": transcript }));

    let tools = tools_schema();
    let mut final_reply = String::new();

    for _ in 0..MAX_TOOL_ITERATIONS {
        let outcome =
            stream_completion(app, &api_url, &api_key, &model, history, Some(&tools)).await?;

        if outcome.tool_calls.is_empty() {
            final_reply = outcome.content;
            history.push(serde_json::json!({ "role": "assistant", "content": final_reply }));
            break;
        }

        let tool_calls_json: Vec<serde_json::Value> = outcome
            .tool_calls
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "type": "function",
                    "function": { "name": t.name, "arguments": t.arguments },
                })
            })
            .collect();
        history.push(serde_json::json!({
            "role": "assistant",
            "content": if outcome.content.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(outcome.content) },
            "tool_calls": tool_calls_json,
        }));

        for call in &outcome.tool_calls {
            let args: serde_json::Value =
                serde_json::from_str(&call.arguments).unwrap_or(serde_json::json!({}));
            let _ = app.emit(
                "waxum-agent://ai-tool",
                serde_json::json!({ "name": call.name, "arguments": args }),
            );
            let result = match call.name.as_str() {
                "search_contact" => {
                    let query = args["query"].as_str().unwrap_or_default();
                    run_search_contact(query, &contacts)
                }
                "search_messages" => {
                    let query = args["query"].as_str().unwrap_or_default();
                    run_search_messages(query, &waxum_target).await
                }
                "send_message" => {
                    let jid = args["jid"].as_str().unwrap_or_default();
                    let text = args["text"].as_str().unwrap_or_default();
                    run_send_message(jid, text, &waxum_target).await
                }
                other => {
                    serde_json::json!({ "ok": false, "error": format!("unknown tool {other}") })
                }
            };
            history.push(serde_json::json!({
                "role": "tool",
                "tool_call_id": call.id,
                "content": result.to_string(),
            }));
        }
    }

    if final_reply.is_empty() {
        final_reply = "Maaf, saya tidak bisa menyelesaikan permintaan itu sekarang.".to_string();
        history.push(serde_json::json!({ "role": "assistant", "content": final_reply.clone() }));
    }

    if history.len() > MAX_HISTORY_MESSAGES {
        let system = history[0].clone();
        let keep_from = history.len() - (MAX_HISTORY_MESSAGES - 1);
        let mut trimmed = vec![system];
        trimmed.extend_from_slice(&history[keep_from..]);
        *history = trimmed;
    }

    let _ = app.emit("waxum-agent://ai-done", &final_reply);
    Ok(final_reply)
}
