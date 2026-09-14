<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { motion, AnimatePresence } from "motion-v";

const props = defineProps<{
  state: "offline" | "idle" | "listening" | "thinking" | "speaking";
}>();

type Expression = "closed" | "happy" | "wide" | "suspect" | "dizzy";

const expression = computed<Expression>(() => {
  switch (props.state) {
    case "offline":
      return "closed";
    case "listening":
      return "wide";
    case "thinking":
      return "suspect";
    case "speaking":
      return "dizzy";
    default:
      return "happy";
  }
});

const color = "#22d3ee";

// Occasional blink layered on top of whatever expression is active, so
// the face reads as alive even sitting idle — like the reference sheet's
// "blink" state cycling in over "happy".
const blinking = ref(false);
let blinkTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleBlink() {
  const delay = 2500 + Math.random() * 3500;
  blinkTimer = setTimeout(() => {
    blinking.value = true;
    setTimeout(() => {
      blinking.value = false;
      scheduleBlink();
    }, 140);
  }, delay);
}

onMounted(scheduleBlink);
onUnmounted(() => {
  if (blinkTimer) clearTimeout(blinkTimer);
});
</script>

<template>
  <div class="relative w-full max-w-[200px] mx-auto aspect-square">
    <div
      class="absolute inset-0 rounded-[28%] border-4"
      :style="{
        borderColor: state === 'offline' ? '#3a4a56' : '#8a97a3',
        backgroundColor: '#050607',
        boxShadow: state === 'offline' ? 'none' : `0 0 24px ${color}33`,
      }" />

    <div class="absolute inset-0 flex items-center justify-center gap-[14%]">
      <AnimatePresence mode="wait">
        <motion.div
          v-if="blinking"
          key="blink"
          :initial="{ opacity: 0 }"
          :animate="{ opacity: 1 }"
          :exit="{ opacity: 0 }"
          class="flex gap-[14%]">
          <div class="w-[22%] h-[4%] rounded-full" :style="{ backgroundColor: color }" />
          <div class="w-[22%] h-[4%] rounded-full" :style="{ backgroundColor: color }" />
        </motion.div>

        <motion.div
          v-else
          :key="expression"
          :initial="{ opacity: 0, scale: 0.85 }"
          :animate="{ opacity: 1, scale: 1 }"
          :exit="{ opacity: 0, scale: 0.85 }"
          :transition="{ duration: 0.25 }"
          class="flex items-center gap-[10%]">
          <!-- happy / idle: soft downward arcs -->
          <template v-if="expression === 'happy'">
            <svg viewBox="0 0 40 30" class="w-[26%]"><path d="M2 26 Q20 2 38 26" fill="none" :stroke="color" stroke-width="8" stroke-linecap="round" /></svg>
            <svg viewBox="0 0 40 30" class="w-[26%]"><path d="M2 26 Q20 2 38 26" fill="none" :stroke="color" stroke-width="8" stroke-linecap="round" /></svg>
          </template>

          <!-- listening: wide, round, attentive -->
          <template v-else-if="expression === 'wide'">
            <motion.div
              class="w-[24%] aspect-square rounded-full"
              :style="{ backgroundColor: color }"
              :animate="{ scale: [1, 1.08, 1] }"
              :transition="{ duration: 1, repeat: Infinity, ease: 'easeInOut' }" />
            <motion.div
              class="w-[24%] aspect-square rounded-full"
              :style="{ backgroundColor: color }"
              :animate="{ scale: [1, 1.08, 1] }"
              :transition="{ duration: 1, repeat: Infinity, ease: 'easeInOut', delay: 0.15 }" />
          </template>

          <!-- thinking: one eye narrowed, asymmetric ("suspect") -->
          <template v-else-if="expression === 'suspect'">
            <svg viewBox="0 0 40 30" class="w-[26%]"><path d="M2 22 Q20 4 38 22" fill="none" :stroke="color" stroke-width="8" stroke-linecap="round" /></svg>
            <svg viewBox="0 0 40 30" class="w-[26%]"><path d="M4 16 H36" fill="none" :stroke="color" stroke-width="8" stroke-linecap="round" /></svg>
          </template>

          <!-- speaking: eyes bounce like a talk-animation, offset phase -->
          <template v-else-if="expression === 'dizzy'">
            <motion.svg
              viewBox="0 0 40 30"
              class="w-[26%]"
              :animate="{ scaleY: [1, 0.4, 1, 0.7, 1] }"
              :transition="{ duration: 0.6, repeat: Infinity, ease: 'easeInOut' }"
              style="transform-origin: center">
              <path d="M2 26 Q20 2 38 26" fill="none" :stroke="color" stroke-width="8" stroke-linecap="round" />
            </motion.svg>
            <motion.svg
              viewBox="0 0 40 30"
              class="w-[26%]"
              :animate="{ scaleY: [1, 0.7, 1, 0.4, 1] }"
              :transition="{ duration: 0.6, repeat: Infinity, ease: 'easeInOut', delay: 0.1 }"
              style="transform-origin: center">
              <path d="M2 26 Q20 2 38 26" fill="none" :stroke="color" stroke-width="8" stroke-linecap="round" />
            </motion.svg>
          </template>

          <!-- offline: flat, closed -->
          <template v-else>
            <div class="w-[22%] h-[4%] rounded-full bg-[#3a4a56]" />
            <div class="w-[22%] h-[4%] rounded-full bg-[#3a4a56]" />
          </template>
        </motion.div>
      </AnimatePresence>
    </div>
  </div>
</template>
