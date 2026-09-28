export type ConnectionMode = "remote" | "bundled";

export interface Settings {
  mode: ConnectionMode;
  baseUrl: string;
  token: string;
  sessionId: string;
  bundledBinaryPath: string;
  elevenLabsApiKey: string;
  elevenLabsVoiceId: string;
  autoReadIncoming: boolean;
  aiApiUrl: string;
  aiApiKey: string;
  aiModel: string;
}

/** New installs run waxum inside the app on desktop; phones can't spawn
 * a child process, so they start in remote mode. */
const IS_MOBILE = typeof navigator !== "undefined" && /Android|iPhone|iPad|iPod/i.test(navigator.userAgent);

export const DEFAULT_SETTINGS: Settings = {
  mode: IS_MOBILE ? "remote" : "bundled",
  baseUrl: "http://127.0.0.1:3451/api/v1",
  token: "",
  sessionId: "",
  bundledBinaryPath: "",
  // ElevenLabs "Rachel" — a reasonable default voice, replaceable in settings.
  elevenLabsVoiceId: "21m00Tcm4TlvDq8ikWAM",
  elevenLabsApiKey: "",
  autoReadIncoming: true,
  aiApiUrl: "",
  aiApiKey: "",
  aiModel: "",
};

export interface SessionSummary {
  id: string;
  name?: string;
  status?: string;
}

export interface IncomingMessage {
  session_id: string;
  event: string;
  timestamp: number;
  offline: boolean;
  data: {
    from: string;
    from_phone?: string;
    chat: string;
    message_id: string;
    is_from_me: boolean;
    push_name?: string;
    message_type: string;
    text?: string;
    caption?: string;
    is_group: boolean;
  };
}
