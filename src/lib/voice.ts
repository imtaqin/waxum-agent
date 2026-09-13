import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "./types";

/** Speaks `text` aloud via ElevenLabs TTS. Resolves once playback ends. */
export async function speak(s: Settings, text: string): Promise<void> {
  if (!s.elevenLabsApiKey || !text.trim()) return;
  const b64 = await invoke<string>("tts_speak", {
    apiKey: s.elevenLabsApiKey,
    voiceId: s.elevenLabsVoiceId,
    text,
  });
  const audio = new Audio(`data:audio/mpeg;base64,${b64}`);
  await new Promise<void>((resolve) => {
    audio.onended = () => resolve();
    audio.onerror = () => resolve();
    audio.play().catch(() => resolve());
  });
}

/**
 * Push-to-talk recorder: call `start()`, then `stop()` to get back the
 * transcript via ElevenLabs STT. One recorder instance per press.
 */
export class PushToTalk {
  private recorder: MediaRecorder | null = null;
  private chunks: Blob[] = [];
  private stream: MediaStream | null = null;

  async start(): Promise<void> {
    this.stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    this.chunks = [];
    this.recorder = new MediaRecorder(this.stream, { mimeType: pickMimeType() });
    this.recorder.ondataavailable = (e) => {
      if (e.data.size > 0) this.chunks.push(e.data);
    };
    this.recorder.start();
  }

  async stop(s: Settings): Promise<string> {
    const recorder = this.recorder;
    if (!recorder) return "";
    const mimeType = recorder.mimeType;

    const blob = await new Promise<Blob>((resolve) => {
      recorder.onstop = () => resolve(new Blob(this.chunks, { type: mimeType }));
      recorder.stop();
    });
    this.stream?.getTracks().forEach((t) => t.stop());
    this.stream = null;
    this.recorder = null;

    const b64 = await blobToBase64(blob);
    if (!s.elevenLabsApiKey) return "";
    return invoke<string>("stt_transcribe", {
      apiKey: s.elevenLabsApiKey,
      audioBase64: b64,
      mimeType,
    });
  }
}

function pickMimeType(): string {
  const candidates = ["audio/webm;codecs=opus", "audio/webm", "audio/mp4", "audio/ogg"];
  for (const c of candidates) {
    if (MediaRecorder.isTypeSupported(c)) return c;
  }
  return "audio/webm";
}

function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onloadend = () => {
      const result = reader.result as string;
      resolve(result.split(",")[1] ?? "");
    };
    reader.onerror = reject;
    reader.readAsDataURL(blob);
  });
}
