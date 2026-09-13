<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Settings } from "../lib/types";
import { waxumStatus } from "../lib/waxum";
import PairingModal from "./PairingModal.vue";

const props = defineProps<{ modelValue: Settings }>();
const emit = defineEmits<{ save: [Settings] }>();

const form = ref<Settings>({ ...props.modelValue });
if (!form.value.sessionId) form.value.sessionId = crypto.randomUUID();

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
    const path = await invoke<string>(
      form.value.bundledBinaryPath ? "bundled_update_binary" : "bundled_ensure_binary",
    );
    form.value.bundledBinaryPath = path;
    testResult.value = `downloaded to ${path}`;
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
    const status = await waxumStatus(form.value);
    testResult.value = `ok — status: ${status.status ?? "unknown"}`;
  } catch (e) {
    const msg = String(e);
    if (msg.includes("HTTP 401")) {
      testResult.value = "failed: token rejected — check base URL/token";
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
        <div class="w-8 h-8 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-500 text-sm font-bold">
          W
        </div>
        <div>
          <div class="text-sm font-semibold">waxum agent</div>
          <div class="text-xs text-white/40">voice-driven WhatsApp assistant</div>
        </div>
      </div>

      <div class="flex gap-2 p-1 bg-white/[0.03] rounded-lg">
        <button
          class="flex-1 text-xs py-1.5 rounded-md transition-colors"
          :class="form.mode === 'remote' ? 'bg-emerald-600 text-white' : 'text-white/50'"
          @click="form.mode = 'remote'">
          Remote URL
        </button>
        <button
          class="flex-1 text-xs py-1.5 rounded-md transition-colors"
          :class="form.mode === 'bundled' ? 'bg-emerald-600 text-white' : 'text-white/50'"
          @click="form.mode = 'bundled'">
          Bundled binary
        </button>
      </div>

      <template v-if="form.mode === 'remote'">
        <label class="flex flex-col gap-1">
          <span class="text-[11px] uppercase tracking-wide text-white/40">waxum base URL</span>
          <input v-model="form.baseUrl" class="input" placeholder="http://127.0.0.1:3451/api/v1" />
        </label>
      </template>
      <template v-else>
        <label class="flex flex-col gap-1">
          <span class="text-[11px] uppercase tracking-wide text-white/40">waxum binary path</span>
          <div class="flex gap-2">
            <input v-model="form.bundledBinaryPath" class="input" placeholder="leave empty to auto-download" />
            <button class="btn-ghost shrink-0" :disabled="pickingBinary" @click="pickBinaryPath">Browse</button>
          </div>
          <p class="text-[11px] text-white/30">
            Leave empty and waxum agent downloads the latest waxum release
            for your OS automatically the first time it connects.
          </p>
        </label>
        <button class="btn-ghost" :disabled="downloading" @click="downloadNow">
          {{ downloading ? "Downloading…" : form.bundledBinaryPath ? "Re-download / update binary" : "Download waxum now" }}
        </button>
        <label class="flex flex-col gap-1">
          <span class="text-[11px] uppercase tracking-wide text-white/40">local port</span>
          <input v-model="form.baseUrl" class="input" placeholder="http://127.0.0.1:3451/api/v1" />
        </label>
        <p class="text-[11px] text-white/30 -mt-2">
          The bundled binary is launched on demand; base URL should point at
          its own <code>127.0.0.1:&lt;port&gt;/api/v1</code>.
        </p>
      </template>

      <label class="flex flex-col gap-1">
        <span class="text-[11px] uppercase tracking-wide text-white/40">waxum bearer token</span>
        <input v-model="form.token" type="password" class="input" placeholder="superadmin or session token" />
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-[11px] uppercase tracking-wide text-white/40">session id (local only)</span>
        <div class="flex gap-2">
          <input v-model="form.sessionId" class="input font-mono text-xs" placeholder="generated locally" />
          <button class="btn-ghost shrink-0 text-xs" @click="newSessionId">New</button>
        </div>
        <p class="text-[11px] text-white/30">
          Generated on this device and never fetched from the server — this
          app never lists other sessions on a shared waxum instance.
        </p>
      </label>

      <div class="flex gap-2">
        <button class="btn-ghost flex-1" :disabled="testing" @click="testWaxum">
          {{ testing ? "Testing…" : "Test connection" }}
        </button>
        <button class="btn-primary flex-1" @click="showPairing = true">Pair (scan QR)</button>
      </div>

      <div class="w-px h-px" />

      <label class="flex flex-col gap-1">
        <span class="text-[11px] uppercase tracking-wide text-white/40">ElevenLabs API key</span>
        <input v-model="form.elevenLabsApiKey" type="password" class="input" placeholder="sk_..." />
      </label>
      <label class="flex flex-col gap-1">
        <span class="text-[11px] uppercase tracking-wide text-white/40">ElevenLabs voice id</span>
        <input v-model="form.elevenLabsVoiceId" class="input" />
      </label>
      <button class="btn-ghost" :disabled="testing" @click="testElevenLabs">
        {{ testing ? "Testing…" : "Test ElevenLabs key" }}
      </button>

      <label class="flex items-center gap-2 text-sm text-white/70">
        <input v-model="form.autoReadIncoming" type="checkbox" class="accent-emerald-500" />
        Read incoming messages aloud automatically
      </label>

      <p v-if="testResult" class="text-xs" :class="testResult.startsWith('failed') ? 'text-red-400' : 'text-emerald-400'">
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
