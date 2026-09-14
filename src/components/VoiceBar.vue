<script setup lang="ts">
import { onUnmounted, ref, watch } from "vue";
import { motion } from "motion-v";
import type { Settings } from "../lib/types";
import { RealtimeVoice } from "../lib/realtimeVoice";

const props = defineProps<{ settings: Settings; active: boolean; speaking: boolean }>();
const emit = defineEmits<{
  transcript: [string];
  error: [string];
  listening: [boolean];
}>();

const listening = ref(false);
const muted = ref(false);
const partial = ref("");
let realtime: RealtimeVoice | null = null;

watch(listening, (v) => emit("listening", v));

// Half-duplex turn-taking, same as Gemini/ChatGPT voice mode: suspend
// sending mic audio while the assistant is talking, otherwise it hears
// its own TTS out of the speakers and transcribes it as a new command
// -- a "talking to itself" loop, not a conversation.
watch(
  () => props.speaking,
  (v) => {
    if (v) {
      realtime?.setSuspended(true);
    } else {
      // Small grace period before resuming — the room's speaker output
      // hasn't fully decayed the instant playback "ends" in JS, and the
      // mic would otherwise catch the tail of the assistant's own voice.
      setTimeout(() => realtime?.setSuspended(false), 400);
    }
  },
);

// Fully automatic — starts as soon as the session is ready, stops the
// moment it isn't (or the user mutes it). No push-to-talk, no button to
// press before the assistant will hear you.
watch(
  () => props.active && !muted.value,
  (shouldListen) => {
    if (shouldListen) start();
    else stop();
  },
  { immediate: true },
);

async function start() {
  if (listening.value || !props.settings.elevenLabsApiKey) {
    if (!props.settings.elevenLabsApiKey) emit("error", "realtime mode needs an ElevenLabs API key in settings");
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
        listening.value = false;
        partial.value = "";
      }
    });
    realtime.setSuspended(props.speaking);
    listening.value = true;
  } catch (e) {
    emit("error", `realtime connect failed: ${e}`);
    realtime = null;
  }
}

function stop() {
  realtime?.stop();
  realtime = null;
  listening.value = false;
  partial.value = "";
}

function toggleMute() {
  muted.value = !muted.value;
}

onUnmounted(() => realtime?.stop());
</script>

<template>
  <div class="flex flex-col items-center gap-1.5 p-3 border-t border-hud-500/20">
    <div class="flex items-end gap-[3px] h-5">
      <motion.div
        v-for="i in 9"
        :key="i"
        class="w-[3px] rounded-full"
        :class="listening ? 'bg-cyber-pink' : 'bg-hud-500/30'"
        :animate="listening
          ? { scaleY: [0.25, 1, 0.4, 0.85, 0.25] }
          : { scaleY: [0.2, 0.35, 0.2] }"
        :transition="{
          duration: listening ? 0.7 : 2.4,
          repeat: Infinity,
          ease: 'easeInOut',
          delay: i * 0.07,
        }"
        style="height: 20px; transform-origin: bottom" />
    </div>

    <div class="flex items-center gap-2">
      <div class="text-[9px] uppercase tracking-widest text-hud-400/50 max-w-[220px] truncate">
        <span v-if="partial" class="text-hud-400/70 normal-case tracking-normal">{{ partial }}</span>
        <span v-else-if="muted" class="text-cyber-pink/70">muted</span>
        <span v-else-if="listening" class="text-cyber-pink hud-glow-text">realtime // always listening</span>
        <span v-else>standby</span>
      </div>
      <motion.button
        class="w-6 h-6 rounded-full flex items-center justify-center border shrink-0"
        :class="muted ? 'border-cyber-pink/60 text-cyber-pink' : 'border-hud-500/40 text-hud-400/70'"
        :while-press="{ scale: 0.9 }"
        :disabled="!active"
        @click="toggleMute">
        <svg v-if="!muted" viewBox="0 0 24 24" fill="currentColor" class="w-3 h-3">
          <path d="M12 14a3 3 0 0 0 3-3V6a3 3 0 0 0-6 0v5a3 3 0 0 0 3 3Z" />
          <path d="M19 11a7 7 0 0 1-14 0H3a9 9 0 0 0 8 8.94V22h2v-2.06A9 9 0 0 0 21 11h-2Z" />
        </svg>
        <svg v-else viewBox="0 0 24 24" fill="currentColor" class="w-3 h-3">
          <path d="M3 3l18 18-1.4 1.4L15 17.8A7 7 0 0 1 5 11H3a9 9 0 0 0 8 8.94V22h2v-2.06a8.96 8.96 0 0 0 3.9-1.44l2.7 2.7L23 20l-20-20L3 3Zm12 8V6a3 3 0 0 0-5.9-.8L15 11Z" />
        </svg>
      </motion.button>
    </div>
  </div>
</template>
