import type { Settings } from "./types";

export type RealtimeEvent =
  | { kind: "partial"; text: string }
  | { kind: "final"; text: string }
  | { kind: "error"; message: string }
  | { kind: "closed" };

/**
 * Continuous-listening mode via ElevenLabs' realtime STT websocket
 * (wss://api.elevenlabs.io/v1/speech-to-text/realtime) — mic audio
 * streams out as raw 16kHz PCM for as long as the session is open, and
 * the server's VAD decides when an utterance ends and commits a
 * transcript, instead of push-to-talk's record-then-upload cycle.
 *
 * Browsers cannot set custom headers on a WebSocket handshake, so the
 * key goes in the connection URL's query string rather than the
 * `xi-api-key` header the REST endpoints use — acceptable for a local
 * desktop app talking directly to ElevenLabs, not something to do from
 * a page anyone else can load.
 */
export class RealtimeVoice {
  private ws: WebSocket | null = null;
  private audioCtx: AudioContext | null = null;
  private source: MediaStreamAudioSourceNode | null = null;
  private processor: ScriptProcessorNode | null = null;
  private mute: GainNode | null = null;
  private stream: MediaStream | null = null;
  private readonly sampleRate = 16000;

  async start(settings: Settings, onEvent: (e: RealtimeEvent) => void): Promise<void> {
    const url =
      `wss://api.elevenlabs.io/v1/speech-to-text/realtime` +
      `?xi-api-key=${encodeURIComponent(settings.elevenLabsApiKey)}` +
      `&sample_rate=${this.sampleRate}&commit_strategy=vad`;
    const ws = new WebSocket(url);
    this.ws = ws;

    ws.onmessage = (ev) => {
      try {
        const msg = JSON.parse(ev.data);
        if (msg.message_type === "partial_transcript" && msg.text) {
          onEvent({ kind: "partial", text: msg.text });
        } else if (msg.message_type === "committed_transcript" && msg.text?.trim()) {
          onEvent({ kind: "final", text: msg.text.trim() });
        }
      } catch {
        // malformed/unrecognized frame, ignore
      }
    };
    ws.onerror = () => onEvent({ kind: "error", message: "realtime connection error" });
    ws.onclose = () => onEvent({ kind: "closed" });

    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error("timed out connecting")), 8000);
      ws.onopen = () => {
        clearTimeout(timeout);
        resolve();
      };
    });

    this.stream = await navigator.mediaDevices.getUserMedia({ audio: { channelCount: 1 } });
    const audioCtx = new AudioContext();
    this.audioCtx = audioCtx;
    this.source = audioCtx.createMediaStreamSource(this.stream);
    this.processor = audioCtx.createScriptProcessor(4096, 1, 1);
    // ScriptProcessorNode only fires onaudioprocess once connected through
    // to a destination — route through a silent gain node so the mic is
    // never actually monitored back out the speakers.
    this.mute = audioCtx.createGain();
    this.mute.gain.value = 0;

    this.processor.onaudioprocess = (e) => {
      if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;
      const input = e.inputBuffer.getChannelData(0);
      const audioBase64 = downsampleAndEncode(input, audioCtx.sampleRate, this.sampleRate);
      this.ws.send(
        JSON.stringify({
          message_type: "input_audio_chunk",
          audio_base_64: audioBase64,
          sample_rate: this.sampleRate,
        }),
      );
    };

    this.source.connect(this.processor);
    this.processor.connect(this.mute);
    this.mute.connect(audioCtx.destination);
  }

  stop(): void {
    this.processor?.disconnect();
    this.source?.disconnect();
    this.mute?.disconnect();
    this.stream?.getTracks().forEach((t) => t.stop());
    this.audioCtx?.close().catch(() => {});
    this.ws?.close();
    this.processor = null;
    this.source = null;
    this.mute = null;
    this.stream = null;
    this.audioCtx = null;
    this.ws = null;
  }
}

function downsampleAndEncode(input: Float32Array, inRate: number, outRate: number): string {
  const ratio = inRate / outRate;
  const outLength = Math.floor(input.length / ratio);
  const pcm16 = new Int16Array(outLength);
  for (let i = 0; i < outLength; i++) {
    const sample = input[Math.floor(i * ratio)] ?? 0;
    const clamped = Math.max(-1, Math.min(1, sample));
    pcm16[i] = clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff;
  }
  const bytes = new Uint8Array(pcm16.buffer);
  let binary = "";
  for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
  return btoa(binary);
}
