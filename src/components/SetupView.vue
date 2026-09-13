<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Settings } from "../lib/types";
import { waxumListSessions } from "../lib/waxum";
import type { SessionSummary } from "../lib/types";

const props = defineProps<{ modelValue: Settings }>();
const emit = defineEmits<{ save: [Settings] }>();

const form = ref<Settings>({ ...props.modelValue });
const testing = ref(false);
const testResult = ref<string | null>(null);
const sessions = ref<SessionSummary[]>([]);
const pickingBinary = ref(false);

async function testWaxum() {
  testing.value = true;
  testResult.value = null;
  try {
    sessions.value = await waxumListSessions(form.value);
    if (!form.value.sessionId && sessions.value.length > 0) {
      form.value.sessionId = sessions.value[0].id;
    }
    testResult.value = `ok — ${sessions.value.length} session(s) found`;
  } catch (e) {
    testResult.value = `failed: ${e}`;
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
            <input v-model="form.bundledBinaryPath" class="input" placeholder="/usr/local/bin/waxum" />
            <button class="btn-ghost shrink-0" :disabled="pickingBinary" @click="pickBinaryPath">Browse</button>
          </div>
        </label>
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
        <span class="text-[11px] uppercase tracking-wide text-white/40">session id</span>
        <select v-if="sessions.length" v-model="form.sessionId" class="input">
          <option v-for="s in sessions" :key="s.id" :value="s.id">{{ s.name || s.id }} ({{ s.status }})</option>
        </select>
        <input v-else v-model="form.sessionId" class="input" placeholder="paired waxum session id" />
      </label>

      <button class="btn-ghost" :disabled="testing" @click="testWaxum">
        {{ testing ? "Testing…" : "Test waxum connection" }}
      </button>

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
  </div>
</template>
