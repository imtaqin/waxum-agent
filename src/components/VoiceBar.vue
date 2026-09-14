<script setup lang="ts">
import { onUnmounted, ref, watch } from "vue";
import { motion } from "motion-v";
import type { Settings } from "../lib/types";
import { PushToTalk } from "../lib/voice";
import { RealtimeVoice } from "../lib/realtimeVoice";

const props = defineProps<{ settings: Settings; busy: boolean }>();
const emit = defineEmits<{
  transcript: [string];
  error: [string];
  listening: [boolean];
}>();

type Mode = "ptt" | "realtime";
const mode = ref<Mode>("ptt");
const recording = ref(false); // ptt: held down. realtime: session open.
const transcribing = ref(false);
const partial = ref("");

let ptt: PushToTalk | null = null;
let realtime: RealtimeVoice | null = null;

watch(recording, (v) => emit("listening", v));

function setMode(next: Mode) {
  if (recording.value) stopCurrent();
  mode.value = next;
}

function stopCurrent() {
  if (mode.value === "ptt") releasePtt();
  else stopRealtime();
}

// -- push-to-talk -----------------------------------------------------

async function pressPtt() {
  if (props.busy || recording.value) return;
  try {
    ptt = new PushToTalk();
    await ptt.start();
    recording.value = true;
  } catch (e) {
    emit("error", `mic access failed: ${e}`);
  }
}

async function releasePtt() {
  if (!recording.value || !ptt) return;
  recording.value = false;
  transcribing.value = true;
  try {
    const text = await ptt.stop(props.settings);
    if (text.trim()) emit("transcript", text.trim());
  } catch (e) {
    emit("error", `transcription failed: ${e}`);
  } finally {
    transcribing.value = false;
    ptt = null;
  }
}

// -- realtime (continuous, ElevenLabs streaming STT) -------------------

async function toggleRealtime() {
  if (props.busy) return;
  if (recording.value) {
    stopRealtime();
    return;
  }
  if (!props.settings.elevenLabsApiKey) {
    emit("error", "realtime mode needs an ElevenLabs API key in settings");
    return;
  }
  realtime = new RealtimeVoice();
  try {
    await realtime.start(props.settings, (e) => {
      if (e.kind === "partial") {
        partial.value = e.text;
      } else if (e.kind === "final") {
        partial.value = "";
        emit("transcript", e.text);
      } else if (e.kind === "error") {
        emit("error", e.message);
      } else if (e.kind === "closed") {
        recording.value = false;
        partial.value = "";
      }
    });
    recording.value = true;
  } catch (e) {
    emit("error", `realtime connect failed: ${e}`);
    realtime = null;
  }
}

function stopRealtime() {
  realtime?.stop();
  realtime = null;
  recording.value = false;
  partial.value = "";
}

onUnmounted(() => {
  ptt = null;
  realtime?.stop();
});
</script>

<template>
  <div class="flex flex-col items-center gap-2 p-4 border-t border-hud-500/20">
    <div class="flex gap-1 p-0.5 bg-hud-500/5 rounded-lg mb-1">
      <button
        class="text-[9px] uppercase tracking-wide px-2.5 py-1 rounded-md transition-colors"
        :class="mode === 'ptt' ? 'bg-hud-500 text-charcoal-900 font-bold' : 'text-hud-400/50'"
        @click="setMode('ptt')">
        Push-to-talk
      </button>
      <button
        class="text-[9px] uppercase tracking-wide px-2.5 py-1 rounded-md transition-colors"
        :class="mode === 'realtime' ? 'bg-cyber-pink text-charcoal-900 font-bold' : 'text-hud-400/50'"
        @click="setMode('realtime')">
        Realtime
      </button>
    </div>

    <div class="flex items-end gap-[3px] h-6">
      <motion.div
        v-for="i in 9"
        :key="i"
        class="w-[3px] rounded-full"
        :class="recording ? 'bg-cyber-pink' : 'bg-hud-500/40'"
        :animate="recording
          ? { scaleY: [0.25, 1, 0.4, 0.85, 0.25] }
          : { scaleY: [0.2, 0.4, 0.2] }"
        :transition="{
          duration: recording ? 0.7 : 2.2,
          repeat: Infinity,
          ease: 'easeInOut',
          delay: i * 0.07,
        }"
        style="height: 24px; transform-origin: bottom" />
    </div>

    <motion.button
      v-if="mode === 'ptt'"
      class="w-16 h-16 rounded-full flex items-center justify-center select-none border"
      :class="recording
        ? 'bg-cyber-pink border-pink-200 shadow-[0_0_0_1px_rgba(255,47,180,0.4),0_0_32px_rgba(255,47,180,0.5)]'
        : 'bg-hud-500 border-hud-300 shadow-hud'"
      :animate="recording ? { scale: [1, 1.1, 1] } : { scale: 1 }"
      :transition="recording ? { duration: 0.9, repeat: Infinity, ease: 'easeInOut' } : { type: 'spring', stiffness: 400, damping: 20 }"
      :while-press="{ scale: 0.9 }"
      :disabled="busy || transcribing"
      @mousedown="pressPtt"
      @mouseup="releasePtt"
      @mouseleave="recording && releasePtt()"
      @touchstart.prevent="pressPtt"
      @touchend.prevent="releasePtt">
      <svg viewBox="0 0 24 24" fill="currentColor" class="w-7 h-7 text-charcoal-900">
        <path d="M12 14a3 3 0 0 0 3-3V6a3 3 0 0 0-6 0v5a3 3 0 0 0 3 3Z" />
        <path d="M19 11a7 7 0 0 1-14 0H3a9 9 0 0 0 8 8.94V22h2v-2.06A9 9 0 0 0 21 11h-2Z" />
      </svg>
    </motion.button>

    <motion.button
      v-else
      class="w-16 h-16 rounded-full flex items-center justify-center select-none border"
      :class="recording
        ? 'bg-cyber-pink border-pink-200 shadow-[0_0_0_1px_rgba(255,47,180,0.4),0_0_32px_rgba(255,47,180,0.5)]'
        : 'bg-hud-500 border-hud-300 shadow-hud'"
      :animate="recording ? { scale: [1, 1.06, 1] } : { scale: 1 }"
      :transition="recording ? { duration: 1.4, repeat: Infinity, ease: 'easeInOut' } : { type: 'spring', stiffness: 400, damping: 20 }"
      :while-press="{ scale: 0.9 }"
      :disabled="busy"
      @click="toggleRealtime">
      <svg v-if="!recording" viewBox="0 0 24 24" fill="currentColor" class="w-7 h-7 text-charcoal-900">
        <path d="M12 14a3 3 0 0 0 3-3V6a3 3 0 0 0-6 0v5a3 3 0 0 0 3 3Z" />
        <path d="M19 11a7 7 0 0 1-14 0H3a9 9 0 0 0 8 8.94V22h2v-2.06A9 9 0 0 0 21 11h-2Z" />
      </svg>
      <svg v-else viewBox="0 0 24 24" fill="currentColor" class="w-6 h-6 text-charcoal-900">
        <rect x="6" y="6" width="12" height="12" rx="2" />
      </svg>
    </motion.button>

    <div class="text-[10px] uppercase tracking-widest text-hud-400/50 h-4 max-w-[260px] truncate">
      <span v-if="partial" class="text-hud-400/70 normal-case tracking-normal">{{ partial }}</span>
      <span v-else-if="mode === 'ptt' && recording" class="text-cyber-pink hud-glow-text">listening // release to transmit</span>
      <span v-else-if="mode === 'ptt' && transcribing">decoding audio…</span>
      <span v-else-if="mode === 'ptt'">hold to speak</span>
      <span v-else-if="recording" class="text-cyber-pink hud-glow-text">realtime // always listening</span>
      <span v-else>tap to start realtime</span>
    </div>
  </div>
</template>
