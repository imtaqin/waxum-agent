import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { IncomingMessage, Settings, SessionSummary } from "./types";

export function waxumStatus(s: Settings) {
  return invoke<any>("waxum_status", {
    baseUrl: s.baseUrl,
    token: s.token,
    sessionId: s.sessionId,
  });
}

export function waxumListSessions(s: Pick<Settings, "baseUrl" | "token">) {
  return invoke<SessionSummary[]>("waxum_list_sessions", {
    baseUrl: s.baseUrl,
    token: s.token,
  });
}

export function waxumSendText(s: Settings, to: string, text: string) {
  return invoke<any>("waxum_send_text", {
    baseUrl: s.baseUrl,
    token: s.token,
    sessionId: s.sessionId,
    to,
    text,
  });
}

export function waxumChatMessages(s: Settings, chatJid: string, limit = 10) {
  return invoke<any>("waxum_chat_messages", {
    baseUrl: s.baseUrl,
    token: s.token,
    sessionId: s.sessionId,
    chatJid,
    limit,
  });
}

export function waxumSearch(s: Settings, query: string) {
  return invoke<any>("waxum_search", {
    baseUrl: s.baseUrl,
    token: s.token,
    sessionId: s.sessionId,
    query,
  });
}

export function startPairing(base: { baseUrl: string; token: string }, sessionId: string, timeoutSeconds = 180) {
  return invoke<void>("waxum_start_pairing", {
    baseUrl: base.baseUrl,
    token: base.token,
    sessionId,
    timeoutSeconds,
  });
}

export function stopPairing() {
  return invoke<void>("waxum_stop_pairing");
}

export interface PairingEvent {
  event: "qr_code" | "pair_code" | "connected" | "ready" | "error" | "timeout";
  data: any;
}

export function onPairingEvent(cb: (e: PairingEvent) => void): Promise<UnlistenFn> {
  return listen<PairingEvent>("waxum-agent://pairing", (e) => cb(e.payload));
}

export function startEventStream(s: Settings) {
  return invoke<void>("waxum_start_events", {
    baseUrl: s.baseUrl,
    token: s.token,
    sessionId: s.sessionId,
  });
}

export function stopEventStream() {
  return invoke<void>("waxum_stop_events");
}

export function onIncomingMessage(cb: (msg: IncomingMessage) => void): Promise<UnlistenFn> {
  return listen<IncomingMessage>("waxum://message", (e) => cb(e.payload));
}

export function onStreamStatus(cb: (msg: string) => void): Promise<UnlistenFn> {
  return listen<string>("waxum://status", (e) => cb(e.payload));
}
