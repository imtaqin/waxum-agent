<script setup lang="ts">
import { ref } from "vue";
import { motion } from "motion-v";
import type { Settings } from "../lib/types";
import { PushToTalk } from "../lib/voice";

const props = defineProps<{ settings: Settings; busy: boolean }>();
const emit = defineEmits<{ transcript: [string]; error: [string] }>();

const recording = ref(false);
const transcribing = ref(false);
let ptt: PushToTalk | null = null;

async function press() {
  if (props.busy || recording.value) return;
  try {
    ptt = new PushToTalk();
    await ptt.start();
    recording.value = true;
  } catch (e) {
    emit("error", `mic access failed: ${e}`);
  }
}

async function release() {
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
</script>

<template>
  <div class="flex flex-col items-center gap-2 p-4 border-t border-hud-500/20">
    <motion.button
      class="w-16 h-16 rounded-full flex items-center justify-center select-none border"
      :class="recording
        ? 'bg-cyber-pink border-pink-200 shadow-[0_0_0_1px_rgba(255,47,180,0.4),0_0_32px_rgba(255,47,180,0.5)]'
        : 'bg-hud-500 border-hud-300 shadow-hud'"
      :animate="recording ? { scale: [1, 1.1, 1] } : { scale: 1 }"
      :transition="recording ? { duration: 0.9, repeat: Infinity, ease: 'easeInOut' } : { type: 'spring', stiffness: 400, damping: 20 }"
      :while-press="{ scale: 0.9 }"
      :disabled="busy || transcribing"
      @mousedown="press"
      @mouseup="release"
      @mouseleave="recording && release()"
      @touchstart.prevent="press"
      @touchend.prevent="release">
      <svg viewBox="0 0 24 24" fill="currentColor" class="w-7 h-7 text-charcoal-900">
        <path d="M12 14a3 3 0 0 0 3-3V6a3 3 0 0 0-6 0v5a3 3 0 0 0 3 3Z" />
        <path d="M19 11a7 7 0 0 1-14 0H3a9 9 0 0 0 8 8.94V22h2v-2.06A9 9 0 0 0 21 11h-2Z" />
      </svg>
    </motion.button>
    <div class="text-[10px] uppercase tracking-widest text-hud-400/50 h-4">
      <span v-if="recording" class="text-cyber-pink hud-glow-text">listening // release to transmit</span>
      <span v-else-if="transcribing">decoding audio…</span>
      <span v-else>hold to speak</span>
    </div>
  </div>
</template>
