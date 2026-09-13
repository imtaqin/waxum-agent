<script setup lang="ts">
import { onMounted, onUnmounted, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import SetupView from "./components/SetupView.vue";
import VoiceBar from "./components/VoiceBar.vue";
import PairingModal from "./components/PairingModal.vue";
import { loadSettings, saveSettings } from "./lib/settings";
import type { IncomingMessage, Settings } from "./lib/types";
import {
  onIncomingMessage,
  onStreamStatus,
  startEventStream,
  stopEventStream,
  waxumSendText,
  waxumStatus,
} from "./lib/waxum";
import { speak } from "./lib/voice";
import { parseCommand } from "./lib/commandParser";

const ready = ref(false);
const settings = ref<Settings | null>(null);
const showSettings = ref(false);
const busy = ref(false);
const statusLine = ref("");
const sessionReady = ref(false);
const showPairing = ref(false);
const log = reactive<{ id: string; text: string; kind: "in" | "out" | "system" }[]>([]);

/** chat display name -> jid, learned from every incoming SSE message so
 * voice commands can say "balas ke budi" instead of a raw JID. */
const knownChats = reactive(new Map<string, { jid: string; lastText: string }>());

let unlistenMessage: (() => void) | null = null;
let unlistenStatus: (() => void) | null = null;

function pushLog(text: string, kind: "in" | "out" | "system") {
  log.push({ id: crypto.randomUUID(), text, kind });
  if (log.length > 200) log.shift();
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
  } catch (e) {
    sessionReady.value = false;
    statusLine.value = `couldn't check session status: ${e}`;
  }
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
  const name = chatDisplayName(msg);
  const text = msg.data.text || msg.data.caption || `[${msg.data.message_type}]`;
  knownChats.set(name.toLowerCase(), { jid: msg.data.chat, lastText: text });
  pushLog(`${name}: ${text}`, "in");

  if (settings.value?.autoReadIncoming) {
    await speak(settings.value, `${name} bilang: ${text}`).catch((e) =>
      pushLog(`tts failed: ${e}`, "system"),
    );
  }
}

function resolveChat(spokenName: string): { jid: string; lastText: string } | undefined {
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
  const intent = parseCommand(raw);

  switch (intent.kind) {
    case "read_latest": {
      const entry = intent.from ? resolveChat(intent.from) : [...knownChats.values()].at(-1);
      if (!entry) {
        await speak(s, "belum ada pesan yang cocok");
        break;
      }
      await speak(s, entry.lastText);
      break;
    }
    case "send_message": {
      const entry = resolveChat(intent.to);
      if (!entry) {
        await speak(s, `gak nemu kontak ${intent.to} di percakapan terbaru`);
        pushLog(`unresolved contact: "${intent.to}"`, "system");
        break;
      }
      try {
        await waxumSendText(s, entry.jid, intent.text);
        pushLog(`sent to ${intent.to}: ${intent.text}`, "system");
        await speak(s, `oke, terkirim ke ${intent.to}`);
      } catch (e) {
        await speak(s, "gagal kirim pesan");
        pushLog(`send failed: ${e}`, "system");
      }
      break;
    }
    case "list_sessions":
      await speak(s, `${knownChats.size} percakapan aktif`);
      break;
    case "unknown":
      pushLog(`unrecognized command: "${intent.raw}"`, "system");
      await speak(s, "gak ngerti perintahnya");
      break;
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

  <div v-else class="h-screen flex flex-col">
    <header class="flex items-center justify-between px-4 h-14 border-b border-white/10 shrink-0">
      <div class="flex items-center gap-2">
        <div class="w-7 h-7 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-500 text-xs font-bold">
          W
        </div>
        <div class="text-sm font-semibold">waxum agent</div>
      </div>
      <button class="text-xs text-white/40 hover:text-white/70" @click="showSettings = true">
        settings
      </button>
    </header>

    <div class="px-4 py-1.5 text-[11px] border-b border-white/5" :class="sessionReady ? 'text-white/40' : 'text-amber-400'">
      {{ statusLine }}
    </div>

    <div v-if="!sessionReady" class="mx-4 mt-3 card p-3 flex items-center justify-between gap-3">
      <p class="text-xs text-white/60">
        This session isn't paired yet — scan a QR to link WhatsApp before sending or reading messages.
      </p>
      <button class="btn-primary shrink-0 text-xs" @click="showPairing = true">Pair now</button>
    </div>

    <main class="flex-1 overflow-y-auto p-4 flex flex-col gap-2">
      <div
        v-for="entry in log"
        :key="entry.id"
        class="max-w-[85%] px-3 py-2 rounded-xl text-sm"
        :class="{
          'self-start card text-white/80': entry.kind === 'in',
          'self-end bg-emerald-600 text-white': entry.kind === 'out',
          'self-center text-[11px] text-white/30': entry.kind === 'system',
        }">
        {{ entry.text }}
      </div>
      <p v-if="log.length === 0 && sessionReady" class="text-center text-white/30 text-sm mt-8">
        Belum ada pesan. Coba bilang "baca pesan" atau "balas ke [nama] bilang [isi]" sambil tahan tombol mic.
      </p>
    </main>

    <VoiceBar
      :settings="settings!"
      :busy="busy || !sessionReady"
      @transcript="onTranscript"
      @error="(e) => pushLog(e, 'system')" />

    <PairingModal
      v-if="showPairing"
      :base-url="settings!.baseUrl"
      :token="settings!.token"
      :session-id="settings!.sessionId"
      @close="showPairing = false"
      @paired="onPairedFromMain" />
  </div>
</template>
