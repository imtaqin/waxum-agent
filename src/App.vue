<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import SetupView from "./components/SetupView.vue";
import VoiceBar from "./components/VoiceBar.vue";
import PairingModal from "./components/PairingModal.vue";
import LiveClock from "./components/LiveClock.vue";
import CoreReactor from "./components/CoreReactor.vue";
import { loadSettings, saveSettings } from "./lib/settings";
import type { IncomingMessage, Settings } from "./lib/types";
import {
  onIncomingMessage,
  onStreamStatus,
  startEventStream,
  stopEventStream,
  waxumGroupInfo,
  waxumSendText,
  waxumSessionMessages,
  waxumStatus,
} from "./lib/waxum";
import { speak } from "./lib/voice";
import { parseCommand } from "./lib/commandParser";
import { aiConfigured, aiInterpret } from "./lib/ai";
import { motion, AnimatePresence } from "motion-v";

const ready = ref(false);
const settings = ref<Settings | null>(null);
const showSettings = ref(false);
const busy = ref(false);
const statusLine = ref("");
const sessionReady = ref(false);
const showPairing = ref(false);
const typedCommand = ref("");

/** Drives CoreReactor: offline (no session) -> idle/listening (voice bar
 * mic state) -> thinking/speaking (transient, while a command/AI call or
 * TTS playback is in flight), then back to idle/listening. */
const listening = ref(false);
const transientState = ref<"thinking" | "speaking" | null>(null);
const assistantState = computed<"offline" | "idle" | "listening" | "thinking" | "speaking">(() => {
  if (!sessionReady.value) return "offline";
  if (transientState.value) return transientState.value;
  return listening.value ? "listening" : "idle";
});

async function speakState(s: Settings, text: string) {
  transientState.value = "speaking";
  try {
    await speak(s, text);
  } finally {
    transientState.value = null;
  }
}

function submitTyped() {
  const text = typedCommand.value.trim();
  if (!text) return;
  typedCommand.value = "";
  onTranscript(text);
}
const log = reactive<{ id: string; text: string; kind: "in" | "out" | "system"; time: string }[]>([]);

/** chat display name -> jid, learned from every incoming SSE message so
 * voice commands can say "balas ke budi" instead of a raw JID. */
interface ChatEntry {
  jid: string;
  messages: string[]; // oldest first, capped — enough for "cek pesan di grup ini"
}
const HISTORY_PER_CHAT = 10;
const knownChats = reactive(new Map<string, ChatEntry>());

function appendChatMessage(name: string, jid: string, text: string) {
  const key = name.toLowerCase();
  const entry = knownChats.get(key) ?? { jid, messages: [] };
  entry.jid = jid;
  entry.messages.push(text);
  if (entry.messages.length > HISTORY_PER_CHAT) entry.messages.shift();
  knownChats.set(key, entry);
}

/** group jid -> subject, so a group's context/log entries are keyed by
 * "the group" rather than whichever member happened to send last. */
const groupNames = new Map<string, string>();

async function resolveGroupName(jid: string): Promise<string> {
  const cached = groupNames.get(jid);
  if (cached) return cached;
  const fallback = `Grup ${jid.split("@")[0].slice(-6)}`;
  try {
    const info = await waxumGroupInfo(settings.value!, jid);
    const name = info.subject?.trim() || fallback;
    groupNames.set(jid, name);
    return name;
  } catch {
    groupNames.set(jid, fallback);
    return fallback;
  }
}

let unlistenMessage: (() => void) | null = null;
let unlistenStatus: (() => void) | null = null;

/** message_id set shared between history load and live SSE, so a message
 * that arrives while history is still loading (or gets fetched again on
 * a reconnect) never shows up twice in the log. */
const seenMessageIds = new Set<string>();
let historyLoaded = false;

function pushLog(text: string, kind: "in" | "out" | "system") {
  const time = new Date().toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  log.push({ id: crypto.randomUUID(), text, kind, time });
  if (log.length > 200) log.shift();
}

/** Primes knownChats from waxum's stored history — silently, as context
 * for voice/AI commands ("cek pesan di grup ini") and contact resolution.
 * Deliberately NOT pushed into the visible log: the screen only ever
 * shows live messages and things explicitly asked for, not a full history
 * dump on every launch. */
async function loadHistory() {
  if (historyLoaded) return;
  const s = settings.value!;
  try {
    const res = await waxumSessionMessages(s, 50);
    const messages = [...(res.messages ?? [])].reverse();
    for (const m of messages) {
      if (seenMessageIds.has(m.message_id)) continue;
      seenMessageIds.add(m.message_id);
      const text = m.body || `[${m.msg_type}]`;
      const isGroup = (m.chat_jid as string).endsWith("@g.us");
      if (isGroup) {
        const groupName = await resolveGroupName(m.chat_jid);
        const sender = m.push_name || (m.sender_jid as string).split("@")[0];
        appendChatMessage(groupName, m.chat_jid, `${sender}: ${text}`);
      } else {
        const name = m.push_name || (m.chat_jid as string).split("@")[0];
        appendChatMessage(name, m.chat_jid, text);
      }
    }
    historyLoaded = true;
  } catch (e) {
    pushLog(`gagal memuat riwayat pesan: ${e}`, "system");
  }
}

async function boot() {
  settings.value = await loadSettings();
  const s = settings.value;
  const configured = s.token && s.sessionId && (s.mode === "remote" ? s.baseUrl : true);
  showSettings.value = !configured;
  ready.value = true;
  if (configured) await connect();
}

async function connect() {
  const s = settings.value!;
  busy.value = true;
  statusLine.value = "connecting…";
  try {
    if (s.mode === "bundled") {
      const running = await invoke<boolean>("bundled_is_running");
      if (!running) {
        statusLine.value = s.bundledBinaryPath
          ? "starting bundled waxum…"
          : "downloading waxum binary…";
        const resolvedPath = await invoke<string>("bundled_start", {
          binaryPath: s.bundledBinaryPath,
          port: 3451,
          env: [],
        });
        if (resolvedPath !== s.bundledBinaryPath) {
          s.bundledBinaryPath = resolvedPath;
          await saveSettings(s);
        }
        await new Promise((r) => setTimeout(r, 1500));
      }
    }
    await startEventStream(s);
    await refreshSessionStatus();
  } catch (e) {
    statusLine.value = `connection failed: ${e}`;
    sessionReady.value = false;
  } finally {
    busy.value = false;
  }
}

/** Checks the actual waxum session status — a live SSE stream and a
 * "connect succeeded" toast mean nothing if the session was never paired,
 * so this is the only thing allowed to claim "connected". */
async function refreshSessionStatus() {
  const s = settings.value!;
  try {
    const status = await waxumStatus(s);
    sessionReady.value = status.status === "logged_in";
    statusLine.value = sessionReady.value
      ? "connected — listening for messages"
      : `session not paired yet (status: ${status.status ?? "unknown"})`;
    if (sessionReady.value) await loadHistory();
  } catch (e) {
    sessionReady.value = false;
    statusLine.value = `couldn't check session status: ${e}`;
  }
}

function onListeningChange(v: boolean) {
  listening.value = v;
}

function onPairedFromMain() {
  showPairing.value = false;
  refreshSessionStatus();
}

async function onSaveSettings(s: Settings) {
  await saveSettings(s);
  settings.value = s;
  showSettings.value = false;
  await stopEventStream().catch(() => {});
  await connect();
}

function chatDisplayName(msg: IncomingMessage): string {
  return msg.data.push_name || msg.data.from_phone || msg.data.chat;
}

async function handleIncoming(msg: IncomingMessage) {
  if (msg.data.is_from_me) return;
  if (seenMessageIds.has(msg.data.message_id)) return;
  seenMessageIds.add(msg.data.message_id);

  const sender = chatDisplayName(msg);
  const text = msg.data.text || msg.data.caption || `[${msg.data.message_type}]`;

  let chatName = sender;
  let logLine = `${sender}: ${text}`;
  let spoken = `Pesan masuk dari ${sender}. ${text}`;
  if (msg.data.is_group) {
    chatName = await resolveGroupName(msg.data.chat);
    logLine = `[${chatName}] ${sender}: ${text}`;
    spoken = `Pesan masuk di grup ${chatName} dari ${sender}. ${text}`;
  }

  appendChatMessage(chatName, msg.data.chat, msg.data.is_group ? `${sender}: ${text}` : text);
  pushLog(logLine, "in");

  if (settings.value?.autoReadIncoming) {
    await speakState(settings.value, spoken).catch((e) => pushLog(`tts failed: ${e}`, "system"));
  }
}

function resolveChat(spokenName: string): ChatEntry | undefined {
  const key = spokenName.trim().toLowerCase();
  if (knownChats.has(key)) return knownChats.get(key);
  for (const [name, entry] of knownChats) {
    if (name.includes(key) || key.includes(name)) return entry;
  }
  return undefined;
}

async function onTranscript(raw: string) {
  pushLog(raw, "out");
  const s = settings.value;
  if (!s) return;
  transientState.value = "thinking";
  const intent = parseCommand(raw);

  switch (intent.kind) {
    case "read_latest": {
      const entry = intent.from ? resolveChat(intent.from) : [...knownChats.values()].at(-1);
      if (!entry) {
        await speakState(s, "Tidak ada pesan yang sesuai dalam basis data.");
        break;
      }
      await speakState(s, entry.messages.at(-1) ?? "Tidak ada isi pesan.");
      break;
    }
    case "send_message": {
      const entry = resolveChat(intent.to);
      if (!entry) {
        await speakState(s, `Kontak ${intent.to} tidak ditemukan dalam percakapan aktif.`);
        pushLog(`unresolved contact: "${intent.to}"`, "system");
        break;
      }
      try {
        await waxumSendText(s, entry.jid, intent.text);
        pushLog(`sent to ${intent.to}: ${intent.text}`, "system");
        await speakState(s, `Pesan telah dikirim kepada ${intent.to}.`);
      } catch (e) {
        await speakState(s, "Pengiriman pesan gagal.");
        pushLog(`send failed: ${e}`, "system");
      }
      break;
    }
    case "list_sessions":
      await speakState(s, `${knownChats.size} percakapan aktif terdeteksi.`);
      break;
    case "unknown":
      await handleWithAi(s, intent.raw);
      break;
  }
}

/** Fallback for anything the fixed regex patterns don't cover — free-form
 * questions like "halo ada pesan apa aja" have no fixed shape, so this
 * hands the transcript plus known-chat context to the configured LLM and
 * either executes the send it decides on or just speaks its reply. */
async function handleWithAi(s: Settings, transcript: string) {
  if (!aiConfigured(s)) {
    pushLog(`unrecognized command: "${transcript}"`, "system");
    await speakState(s, "Perintah tidak dikenali.");
    return;
  }
  const context =
    [...knownChats.entries()]
      .map(([name, e]) => `## ${name}\n${e.messages.join("\n")}`)
      .join("\n\n") || "(belum ada percakapan tercatat)";

  try {
    const decision = await aiInterpret(s, transcript, context);
    if (decision.action === "send_message" && decision.to && decision.text) {
      const entry = resolveChat(decision.to);
      if (entry) {
        try {
          await waxumSendText(s, entry.jid, decision.text);
          pushLog(`sent to ${decision.to}: ${decision.text}`, "system");
        } catch (e) {
          pushLog(`send failed: ${e}`, "system");
        }
      } else {
        pushLog(`ai wanted to message unresolved contact "${decision.to}"`, "system");
      }
    }
    pushLog(decision.reply, "in");
    await speakState(s, decision.reply);
  } catch (e) {
    pushLog(`ai request failed: ${e}`, "system");
    await speakState(s, "Modul AI tidak merespons.");
  }
}

onMounted(async () => {
  await boot();
  unlistenMessage = await onIncomingMessage(handleIncoming);
  unlistenStatus = await onStreamStatus((m) => (statusLine.value = m));
});

onUnmounted(() => {
  unlistenMessage?.();
  unlistenStatus?.();
});
</script>

<template>
  <div v-if="!ready" class="h-screen flex items-center justify-center text-white/40 text-sm">
    loading…
  </div>

  <SetupView
    v-else-if="showSettings"
    :model-value="settings!"
    @save="onSaveSettings" />

  <div v-else class="h-screen flex flex-col hud-frame overflow-hidden">
    <span class="hud-corner-tr" />
    <span class="hud-corner-br" />

    <header class="flex items-center justify-between px-4 h-11 shrink-0">
      <div class="text-[10px] font-semibold tracking-[0.3em] uppercase hud-glow-text">waxum // agent</div>
      <div class="flex items-center gap-3">
        <LiveClock />
        <button class="text-[10px] uppercase tracking-widest text-hud-400/50 hover:text-hud-400" @click="showSettings = true">
          config
        </button>
      </div>
    </header>
    <div class="cyber-line hud-flicker" />

    <div class="shrink-0 pt-4 pb-2 flex flex-col items-center">
      <CoreReactor :state="assistantState" />
      <div
        class="mt-2 text-[10px] uppercase tracking-wide max-w-[85%] text-center truncate"
        :class="sessionReady ? 'text-hud-400/50' : 'text-cyber-pink'">
        &gt; {{ statusLine }}
      </div>
    </div>

    <AnimatePresence>
      <motion.div
        v-if="!sessionReady"
        :initial="{ opacity: 0, y: -8 }"
        :animate="{ opacity: 1, y: 0 }"
        :exit="{ opacity: 0, y: -8 }"
        class="mx-4 mb-2 card p-3 flex items-center justify-between gap-3 shrink-0">
        <p class="text-xs text-hud-400/70">
          Sesi belum tertaut. Pindai kode QR untuk mengaktifkan modul WhatsApp.
        </p>
        <button class="btn-primary shrink-0 text-xs" @click="showPairing = true">Pair now</button>
      </motion.div>
    </AnimatePresence>

    <div class="cyber-line" />
    <main class="flex-1 overflow-y-auto px-4 py-2 flex flex-col gap-1.5 font-mono text-xs">
      <AnimatePresence>
        <motion.div
          v-for="entry in log"
          :key="entry.id"
          :initial="{ opacity: 0, x: -8 }"
          :animate="{ opacity: 1, x: 0 }"
          :transition="{ duration: 0.18 }"
          class="flex gap-2 items-baseline"
          :class="{
            'text-hud-300': entry.kind === 'in',
            'text-hud-500/80': entry.kind === 'out',
            'text-hud-400/25 text-[10px]': entry.kind === 'system',
          }">
          <span class="text-hud-400/30 shrink-0 tabular-nums">{{ entry.time }}</span>
          <span
            class="shrink-0 uppercase tracking-wide"
            :class="{
              'text-cyber-pink/70': entry.kind === 'in',
              'text-hud-500': entry.kind === 'out',
              'opacity-0 w-0': entry.kind === 'system',
            }">
            {{ entry.kind === 'in' ? '< RX' : entry.kind === 'out' ? '> TX' : '' }}
          </span>
          <span class="min-w-0 break-words">{{ entry.text }}</span>
        </motion.div>
      </AnimatePresence>
      <p v-if="log.length === 0 && sessionReady" class="text-center text-hud-400/30 text-xs mt-6 uppercase tracking-wide">
        Standby. Ucapkan "baca pesan" atau ketik perintah di bawah.
      </p>
    </main>
    <div class="cyber-line" />

    <form class="flex gap-2 px-4 pt-2 shrink-0" @submit.prevent="submitTyped">
      <input
        v-model="typedCommand"
        class="input flex-1"
        placeholder="ketik perintah…"
        :disabled="!sessionReady" />
      <motion.button
        type="submit"
        class="btn-ghost shrink-0"
        :while-press="{ scale: 0.94 }"
        :disabled="!sessionReady || !typedCommand.trim()">
        Send
      </motion.button>
    </form>

    <VoiceBar
      :settings="settings!"
      :busy="busy || !sessionReady"
      @transcript="onTranscript"
      @error="(e) => pushLog(e, 'system')"
      @listening="onListeningChange" />

    <AnimatePresence>
      <PairingModal
        v-if="showPairing"
        :base-url="settings!.baseUrl"
        :token="settings!.token"
        :session-id="settings!.sessionId"
        @close="showPairing = false"
        @paired="onPairedFromMain" />
    </AnimatePresence>
  </div>
</template>
