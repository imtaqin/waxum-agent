<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { motion } from "motion-v";

const props = defineProps<{
  state: "offline" | "idle" | "listening" | "thinking" | "speaking";
}>();

const color = computed(() => (props.state === "offline" ? "#3a4a56" : "#22d3ee"));

/** One shape language for every expression — a filled crescent, varied
 * only by curve depth/thickness/symmetry — instead of mixing strokes,
 * circles and bars per state, which read as disjointed rather than one
 * character with different expressions. */
function crescent(curve: number, thickness: number, yBase = 26): string {
  const x0 = 2;
  const x1 = 38;
  const cx = 20;
  const topY = yBase - curve;
  const innerY = topY + thickness;
  return `M${x0} ${yBase} Q${cx} ${topY} ${x1} ${yBase} Q${cx} ${innerY} ${x0} ${yBase} Z`;
}

const HAPPY = { left: crescent(22, 14), right: crescent(22, 14) };
const LISTEN = { left: crescent(18, 18), right: crescent(18, 18) };
const SUSPECT = { left: crescent(20, 12), right: crescent(6, 7, 22) };
const SPEAKING = HAPPY;
const OFFLINE = { left: crescent(3, 6), right: crescent(3, 6) };

const eyes = computed(() => {
  switch (props.state) {
    case "listening":
      return LISTEN;
    case "thinking":
      return SUSPECT;
    case "speaking":
      return SPEAKING;
    case "offline":
      return OFFLINE;
    default:
      return HAPPY;
  }
});

const breathe = computed(() => props.state !== "offline");

// A random blink layered over whichever expression is active, so the
// face reads as alive at rest instead of only reacting to state changes.
const blinking = ref(false);
let blinkTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleBlink() {
  const delay = 2800 + Math.random() * 3200;
  blinkTimer = setTimeout(() => {
    if (props.state !== "offline") blinking.value = true;
    setTimeout(() => {
      blinking.value = false;
      scheduleBlink();
    }, 120);
  }, delay);
}

onMounted(scheduleBlink);
onUnmounted(() => {
  if (blinkTimer) clearTimeout(blinkTimer);
});
</script>

<template>
  <motion.div
    class="relative w-full max-w-[188px] mx-auto aspect-square"
    :animate="breathe ? { scale: [1, 1.02, 1] } : { scale: 1 }"
    :transition="{ duration: 3.2, repeat: Infinity, ease: 'easeInOut' }">
    <div
      class="absolute inset-0 rounded-[30%] transition-shadow duration-500"
      :style="{
        background: 'radial-gradient(circle at 50% 38%, #0c0f10 0%, #050607 75%)',
        border: `3px solid ${state === 'offline' ? '#2a343d' : '#95a2ad'}`,
        boxShadow: state === 'offline'
          ? 'inset 0 0 20px rgba(0,0,0,0.6)'
          : `inset 0 0 20px rgba(0,0,0,0.6), 0 0 26px ${color}40`,
      }" />
    <div class="absolute inset-[6%] rounded-[26%] border border-white/5 pointer-events-none" />

    <div class="absolute inset-0 flex items-center justify-center gap-[9%]">
        <motion.div
          v-if="blinking"
          key="blink"
          :initial="{ opacity: 0, scaleY: 0.3 }"
          :animate="{ opacity: 1, scaleY: 1 }"
          :transition="{ duration: 0.1 }"
          class="flex gap-[9%]">
          <div class="w-[26%] h-[5%] rounded-full" :style="{ backgroundColor: color }" />
          <div class="w-[26%] h-[5%] rounded-full" :style="{ backgroundColor: color }" />
        </motion.div>

        <motion.div
          v-else
          :key="state"
          :initial="false"
          :animate="{ scale: [0.92, 1] }"
          :transition="{ duration: 0.22, ease: 'easeOut' }"
          class="flex items-center gap-[9%] w-[84%]">
          <motion.svg
            viewBox="0 0 40 30"
            class="w-1/2"
            :animate="state === 'listening'
              ? { scale: [1, 1.06, 1] }
              : state === 'speaking'
                ? { scaleY: [1, 0.45, 1, 0.7, 1] }
                : { scale: 1 }"
            :transition="state === 'listening'
              ? { duration: 1.1, repeat: Infinity, ease: 'easeInOut' }
              : state === 'speaking'
                ? { duration: 0.55, repeat: Infinity, ease: 'easeInOut' }
                : { duration: 0.3 }"
            style="transform-origin: center">
            <path :d="eyes.left" :fill="color" :style="{ filter: state === 'offline' ? 'none' : `drop-shadow(0 0 3px ${color})` }" />
          </motion.svg>
          <motion.svg
            viewBox="0 0 40 30"
            class="w-1/2"
            :animate="state === 'listening'
              ? { scale: [1, 1.06, 1] }
              : state === 'speaking'
                ? { scaleY: [1, 0.7, 1, 0.45, 1] }
                : { scale: 1 }"
            :transition="state === 'listening'
              ? { duration: 1.1, repeat: Infinity, ease: 'easeInOut', delay: 0.15 }
              : state === 'speaking'
                ? { duration: 0.55, repeat: Infinity, ease: 'easeInOut', delay: 0.08 }
                : { duration: 0.3 }"
            style="transform-origin: center">
            <path :d="eyes.right" :fill="color" :style="{ filter: state === 'offline' ? 'none' : `drop-shadow(0 0 3px ${color})` }" />
          </motion.svg>
        </motion.div>
    </div>
  </motion.div>
</template>
