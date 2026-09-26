import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Settings } from "./types";

export interface AiDecision {
  action: "send_message" | "none";
  to: string;
  text: string;
  reply: string;
}

export interface AiContact {
  name: string;
  jid: string;
  recent: string[];
}

export function aiConfigured(s: Settings): boolean {
  return Boolean(s.aiApiUrl && s.aiApiKey && s.aiModel);
}

/** One-shot, no tools -- kept only for the settings screen's "Test AI
 * connection" button. `aiConverse` below is the real conversational path. */
export function aiInterpret(s: Settings, transcript: string, context: string) {
  return invoke<AiDecision>("ai_interpret", {
    apiUrl: s.aiApiUrl,
    apiKey: s.aiApiKey,
    model: s.aiModel,
    transcript,
    context,
  });
}

/**
 * The TARS/Siri path: the model can call `search_contact`/`send_message`
 * itself instead of this side matching a heard name against `knownChats`
 * with a plain substring check, and the reply streams back as it's
 * generated (`onAiStream`) instead of arriving all at once. Conversation
 * history lives on the Rust side across calls, so this only ever sends the
 * newest transcript.
 */
export function aiConverse(s: Settings, transcript: string, contacts: AiContact[]) {
  return invoke<string>("ai_converse", {
    args: {
      apiUrl: s.aiApiUrl,
      apiKey: s.aiApiKey,
      model: s.aiModel,
      transcript,
      contacts,
      baseUrl: s.baseUrl,
      token: s.token,
      sessionId: s.sessionId,
    },
  });
}

export function aiResetConversation() {
  return invoke<void>("ai_reset_conversation");
}

export function onAiStream(cb: (chunk: string) => void): Promise<UnlistenFn> {
  return listen<string>("waxum-agent://ai-stream", (e) => cb(e.payload));
}

export function onAiDone(cb: (fullText: string) => void): Promise<UnlistenFn> {
  return listen<string>("waxum-agent://ai-done", (e) => cb(e.payload));
}

export function onAiTool(cb: (call: { name: string; arguments: unknown }) => void): Promise<UnlistenFn> {
  return listen<{ name: string; arguments: unknown }>("waxum-agent://ai-tool", (e) => cb(e.payload));
}
