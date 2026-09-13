//! Free-form natural-language understanding, OpenAI-compatible. Falls
//! back from `commandParser.ts`'s fixed regex patterns for anything that
//! doesn't match one of them — "halo ada pesan apa aja" has no fixed
//! shape, so it needs an actual model in the loop, not more regex.

use crate::error::{AppError, AppResult};

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
pub async fn interpret(
    api_url: &str,
    api_key: &str,
    model: &str,
    transcript: &str,
    context: &str,
) -> AppResult<serde_json::Value> {
    let system = format!(
        "Kamu adalah Waxum Agent, asisten suara robotik untuk WhatsApp. Jawab selalu dalam \
         Bahasa Indonesia yang formal dan ringkas, gaya seperti AI asisten (mirip Jarvis) -- \
         tegas, tanpa basa-basi, tanpa emoji.\n\n\
         Konteks percakapan yang diketahui (nama: pesan terakhir):\n{context}\n\n\
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
        .post(format!("{}/chat/completions", api_url.trim_end_matches('/')))
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
    serde_json::from_str(stripped)
        .map_err(|e| AppError::Ai(format!("model did not return valid JSON: {e} -- got: {stripped}")))
}
