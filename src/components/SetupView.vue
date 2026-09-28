<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Settings } from "../lib/types";
import { waxumStatus } from "../lib/waxum";
import { aiInterpret } from "../lib/ai";
import { isMobile, launchBundled } from "../lib/bundled";
import PairingModal from "./PairingModal.vue";
import RadarRing from "./RadarRing.vue";

const props = defineProps<{ modelValue: Settings }>();
const emit = defineEmits<{ save: [Settings] }>();

const form = ref<Settings>({ ...props.modelValue });
if (!form.value.sessionId) form.value.sessionId = crypto.randomUUID();
if (isMobile && form.value.mode === "bundled") form.value.mode = "remote";

const launching = ref(false);
const bundledStatus = ref<string | null>(null);

/** In bundled mode, starts the in-app waxum (if needed) and fills in its
 * URL and token before anything talks to it. */
async function ensureBundled(): Promise<boolean> {
  if (form.value.mode !== "bundled") return true;
  launching.value = true;
  bundledStatus.value = "starting waxum (first run downloads it)…";
  try {
    const info = await launchBundled(form.value);
    bundledStatus.value = `running on 127.0.0.1:${info.port}`;
    return true;
  } catch (e) {
    bundledStatus.value = null;
    testResult.value = `failed: couldn't start waxum — ${e}`;
    return false;
  } finally {
    launching.value = false;
  }
}

async function openPairing() {
  if (await ensureBundled()) showPairing.value = true;
}

const testing = ref(false);
const testResult = ref<string | null>(null);
const pickingBinary = ref(false);
const downloading = ref(false);
const showPairing = ref(false);

function newSessionId() {
  form.value.sessionId = crypto.randomUUID();
  testResult.value = null;
}

function onPaired() {
  showPairing.value = false;
  testResult.value = "paired — session is connected";
}

async function downloadNow() {
  downloading.value = true;
  testResult.value = null;
  try {
    const path = await invoke<string>("bundled_update_binary");
    await invoke("bundled_stop");
    bundledStatus.value = null;
    testResult.value = `updated — ${path}; waxum restarts on next connect`;
  } catch (e) {
    testResult.value = `download failed: ${e}`;
  } finally {
    downloading.value = false;
  }
}

/** Checks only this app's own session id — never lists sessions on the
 * server, so a shared/multi-tenant waxum instance never exposes anyone
 * else's session ids or status to this app. */
async function testWaxum() {
  testing.value = true;
  testResult.value = null;
  try {
    if (!(await ensureBundled())) return;
    const status = await waxumStatus(form.value);
    testResult.value = `ok — status: ${status.status ?? "unknown"}`;
  } catch (e) {
    const msg = String(e);
    if (msg.includes("HTTP 401")) {
      testResult.value = form.value.mode === "bundled"
        ? "failed: token rejected — restart the app to relaunch waxum"
        : "failed: token rejected — check base URL/token";
    } else if (msg.includes("HTTP 503") || msg.includes("HTTP 404")) {
      testResult.value = "reachable — token ok, session not paired yet (use Pair below)";
    } else {
      testResult.value = `failed: ${e}`;
    }
  } finally {
    testing.value = false;
  }
}

async function testElevenLabs() {
  testing.value = true;
  testResult.value = null;
  try {
    await invoke("elevenlabs_check", { apiKey: form.value.elevenLabsApiKey });
    testResult.value = "elevenlabs key ok";
  } catch (e) {
    testResult.value = `elevenlabs failed: ${e}`;
  } finally {
    testing.value = false;
  }
}

async function testAi() {
  testing.value = true;
  testResult.value = null;
  try {
    const decision = await aiInterpret(form.value, "halo, tes koneksi", "(tidak ada konteks)");
    testResult.value = `ai ok — reply: ${decision.reply}`;
  } catch (e) {
    testResult.value = `ai failed: ${e}`;
  } finally {
    testing.value = false;
  }
}

async function pickBinaryPath() {
  pickingBinary.value = true;
  try {
    const path = await open({ multiple: false });
    if (typeof path === "string") form.value.bundledBinaryPath = path;
  } finally {
    pickingBinary.value = false;
  }
}

function save() {
  emit("save", { ...form.value });
}
</script>

<template>
  <div class="min-h-screen flex items-center justify-center p-6">
    <div class="card w-full max-w-sm p-6 flex flex-col gap-4">
      <div class="flex items-center gap-2">
        <RadarRing />
        <div>
          <div class="text-sm font-semibold tracking-[0.2em] uppercase hud-glow-text">waxum // agent</div>
          <div class="text-[10px] uppercase tracking-widest text-hud-400/40">voice interface — whatsapp control module</div>
        </div>
      </div>

      <div v-if="!isMobile" class="flex gap-2 p-1 bg-hud-500/5 rounded-lg">
        <button
          class="flex-1 text-xs py-1.5 rounded-md transition-colors"
          :class="form.mode === 'remote' ? 'bg-hud-500 text-charcoal-900' : 'text-hud-400/50'"
          @click="form.mode = 'remote'">
          Remote URL
        </button>
        <button
          class="flex-1 text-xs py-1.5 rounded-md transition-colors"
          :class="form.mode === 'bundled' ? 'bg-hud-500 text-charcoal-900' : 'text-hud-400/50'"
          @click="form.mode = 'bundled'">
          Built-in waxum
        </button>
      </div>

      <template v-if="form.mode === 'remote'">
        <label class="flex flex-col gap-1">
          <span class="text-[11px] uppercase tracking-wide text-hud-400/40">waxum base URL</span>
          <input v-model="form.baseUrl" class="input" placeholder="http://127.0.0.1:3451/api/v1" />
        </label>
      </template>
      <template v-else>
        <p class="text-[11px] text-hud-400/50">
          waxum runs inside this app. It is downloaded automatically the first
          time, and its address and access token are set up for you — nothing
          to install or type in.
        </p>
        <button class="btn-ghost" :disabled="launching" @click="ensureBundled">
          {{ launching ? "Starting…" : bundledStatus ? "waxum is running" : "Start waxum now" }}
        </button>
        <p v-if="bundledStatus" class="text-[11px] text-hud-400/70 -mt-2">{{ bundledStatus }}</p>
        <details class="text-[11px] text-hud-400/40">
          <summary class="cursor-pointer uppercase tracking-wide">Advanced</summary>
          <div class="flex flex-col gap-2 mt-2">
            <label class="flex flex-col gap-1">
              <span class="uppercase tracking-wide">custom waxum binary (optional)</span>
              <div class="flex gap-2">
                <input v-model="form.bundledBinaryPath" class="input" placeholder="leave empty to use the downloaded one" />
                <button class="btn-ghost shrink-0" :disabled="pickingBinary" @click="pickBinaryPath">Browse</button>
              </div>
            </label>
            <button class="btn-ghost" :disabled="downloading" @click="downloadNow">
              {{ downloading ? "Downloading…" : "Update waxum to the latest release" }}
            </button>
          </div>
        </details>
      </template>

      <label v-if="form.mode === 'remote'" class="flex flex-col gap-1">
        <span class="text-[11px] uppercase tracking-wide text-hud-400/40">waxum bearer token</span>
        <input v-model="form.token" type="password" class="input" placeholder="superadmin or session token" />
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-[11px] uppercase tracking-wide text-hud-400/40">session id (local only)</span>
        <div class="flex gap-2">
          <input v-model="form.sessionId" class="input font-mono text-xs" placeholder="generated locally" />
          <button class="btn-ghost shrink-0 text-xs" @click="newSessionId">New</button>
        </div>
        <p class="text-[11px] text-hud-400/30">
          Generated on this device and never fetched from the server — this
          app never lists other sessions on a shared waxum instance.
        </p>
      </label>

      <div class="flex gap-2">
        <button class="btn-ghost flex-1" :disabled="testing" @click="testWaxum">
          {{ testing ? "Testing…" : "Test connection" }}
        </button>
        <button class="btn-primary flex-1" :disabled="launching" @click="openPairing">Pair (scan QR)</button>
      </div>

      <div class="w-px h-px" />

      <label class="flex flex-col gap-1">
        <span class="text-[11px] uppercase tracking-wide text-hud-400/40">ElevenLabs API key</span>
        <input v-model="form.elevenLabsApiKey" type="password" class="input" placeholder="sk_..." />
      </label>
      <label class="flex flex-col gap-1">
        <span class="text-[11px] uppercase tracking-wide text-hud-400/40">ElevenLabs voice id</span>
        <input v-model="form.elevenLabsVoiceId" class="input" />
      </label>
      <button class="btn-ghost" :disabled="testing" @click="testElevenLabs">
        {{ testing ? "Testing…" : "Test ElevenLabs key" }}
      </button>

      <div class="w-px h-px" />

      <label class="flex flex-col gap-1">
        <span class="text-[11px] uppercase tracking-wide text-hud-400/40">AI endpoint (OpenAI-compatible)</span>
        <input v-model="form.aiApiUrl" class="input" placeholder="https://api.example.com/v1" />
      </label>
      <label class="flex flex-col gap-1">
        <span class="text-[11px] uppercase tracking-wide text-hud-400/40">AI API key</span>
        <input v-model="form.aiApiKey" type="password" class="input" />
      </label>
      <label class="flex flex-col gap-1">
        <span class="text-[11px] uppercase tracking-wide text-hud-400/40">AI model</span>
        <input v-model="form.aiModel" class="input" />
      </label>
      <p class="text-[11px] text-hud-400/30 -mt-2">
        Used only for free-form commands the fixed patterns don't match
        (e.g. "halo ada pesan apa aja") — leave empty to keep the assistant
        pattern-only.
      </p>
      <button class="btn-ghost" :disabled="testing" @click="testAi">
        {{ testing ? "Testing…" : "Test AI connection" }}
      </button>

      <label class="flex items-center gap-2 text-sm text-hud-400/70">
        <input v-model="form.autoReadIncoming" type="checkbox" class="accent-hud-500" />
        Read incoming messages aloud automatically
      </label>

      <p v-if="testResult" class="text-xs" :class="testResult.startsWith('failed') ? 'text-red-400' : 'text-hud-400'">
        {{ testResult }}
      </p>

      <button class="btn-primary" @click="save">Save and continue</button>
    </div>

    <PairingModal
      v-if="showPairing"
      :base-url="form.baseUrl"
      :token="form.token"
      :session-id="form.sessionId"
      @close="showPairing = false"
      @paired="onPaired" />
  </div>
</template>
