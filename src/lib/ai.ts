import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "./types";

export interface AiDecision {
  action: "send_message" | "none";
  to: string;
  text: string;
  reply: string;
}

export function aiConfigured(s: Settings): boolean {
  return Boolean(s.aiApiUrl && s.aiApiKey && s.aiModel);
}

export function aiInterpret(s: Settings, transcript: string, context: string) {
  return invoke<AiDecision>("ai_interpret", {
    apiUrl: s.aiApiUrl,
    apiKey: s.aiApiKey,
    model: s.aiModel,
    transcript,
    context,
  });
}
