<script setup lang="ts">
import { ref } from "vue";
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
  <div class="flex flex-col items-center gap-2 p-4 border-t border-white/10">
    <button
      class="w-16 h-16 rounded-full flex items-center justify-center transition-all select-none"
      :class="recording
        ? 'bg-red-500 scale-110 shadow-lg shadow-red-500/30'
        : 'bg-emerald-600 hover:bg-emerald-500'"
      :disabled="busy || transcribing"
      @mousedown="press"
      @mouseup="release"
      @mouseleave="recording && release()"
      @touchstart.prevent="press"
      @touchend.prevent="release">
      <svg viewBox="0 0 24 24" fill="currentColor" class="w-7 h-7 text-white">
        <path d="M12 14a3 3 0 0 0 3-3V6a3 3 0 0 0-6 0v5a3 3 0 0 0 3 3Z" />
        <path d="M19 11a7 7 0 0 1-14 0H3a9 9 0 0 0 8 8.94V22h2v-2.06A9 9 0 0 0 21 11h-2Z" />
      </svg>
    </button>
    <div class="text-xs text-white/40 h-4">
      <span v-if="recording">listening… release to send</span>
      <span v-else-if="transcribing">transcribing…</span>
      <span v-else>hold to talk</span>
    </div>
  </div>
</template>
