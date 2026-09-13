/**
 * Turns a transcript into an intent. Regex-based on purpose — voice
 * commands here are a handful of fixed shapes ("baca pesan dari X",
 * "balas ke X bilang Y"), not open-ended natural language, so a small
 * language model would be overkill for what a few patterns already cover.
 * Add a pattern here rather than reaching for an LLM unless free-form
 * commands become a real requirement.
 */

export type Intent =
  | { kind: "read_latest"; from?: string }
  | { kind: "send_message"; to: string; text: string }
  | { kind: "list_sessions" }
  | { kind: "unknown"; raw: string };

const READ_PATTERNS = [
  /^(?:tolong\s+)?bacain?\s+(?:pesan\s*)?(?:dari\s+(.+))?$/i,
  /^(?:tolong\s+)?baca\s+pesan\s*(?:dari\s+(.+))?$/i,
  /^(?:apa\s+)?pesan\s+(?:terbaru|masuk)\s*(?:dari\s+(.+))?$/i,
];

const SEND_PATTERNS = [
  /^(?:balas|reply)\s+(?:ke\s+)?(.+?)\s+(?:bilang|dengan|isi(?:nya)?)\s+(.+)$/i,
  /^kirim(?:kan)?\s+pesan\s+ke\s+(.+?)\s+(?:bilang|isi(?:nya)?|:)\s*(.+)$/i,
  /^kirim(?:kan)?\s+ke\s+(.+?)\s+(?:bilang|:)\s*(.+)$/i,
];

const LIST_PATTERNS = [/^(?:list|daftar)\s+(?:sesi|session)$/i];

export function parseCommand(raw: string): Intent {
  const text = raw.trim();
  if (!text) return { kind: "unknown", raw };

  for (const pattern of READ_PATTERNS) {
    const m = text.match(pattern);
    if (m) return { kind: "read_latest", from: m[1]?.trim() };
  }

  for (const pattern of SEND_PATTERNS) {
    const m = text.match(pattern);
    if (m) return { kind: "send_message", to: m[1].trim(), text: m[2].trim() };
  }

  for (const pattern of LIST_PATTERNS) {
    if (pattern.test(text)) return { kind: "list_sessions" };
  }

  return { kind: "unknown", raw: text };
}
